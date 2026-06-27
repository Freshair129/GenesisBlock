//! Rebuild-from-source-of-truth tests.
//!
//! Validates that the engine's persistence layer is a faithful source of truth:
//! write data → save → destroy in-memory state → reload → assert identical.
//! Covers nodes, edges, vectors, multi-collection, metadata, and graph indices.

use genesis_block_native::{EdgeInput, HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
    })
    .unwrap()
}

fn add_node(s: &Storage, id: &str, emb: [f64; 4], labels: &[&str]) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: labels.iter().map(|l| l.to_string()).collect(),
        props: Some(serde_json::json!({"source": id})),
        embedding: Some(emb.to_vec()),
        lang: Some("en".to_string()),
        valid_from: Some("2024-01-01T00:00:00Z".to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

fn add_edge(s: &Storage, id: &str, from: &str, to: &str, rel: &str) {
    s.add_edge(EdgeInput {
        id: Some(id.to_string()),
        from: from.to_string(),
        to: to.to_string(),
        rel: rel.to_string(),
        props: Some(serde_json::json!({"weight": 1.0})),
        valid_from: Some("2024-02-01T00:00:00Z".to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
}

fn search_top1(s: &Storage, query: [f64; 4]) -> String {
    s.flush_index();
    let results = s
        .hybrid_search(HybridSearchInput {
            query_vector: query.to_vec(),
            k: 1,
            alpha: Some(1.0),
            lang: None,
            as_of: None,
            collection: None,
            ef_search: None,
        })
        .unwrap();
    results.first().map(|r| r.node.id.clone()).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// 1. Snapshot round-trip: save → drop → reopen → identical state
// ---------------------------------------------------------------------------
#[test]
fn snapshot_roundtrip_nodes_edges_vectors() {
    let path = fresh("rebuild_snapshot_roundtrip");

    let node_ids: Vec<String> = (0..20).map(|i| format!("node_{i}")).collect();
    let mut edge_count = 0;

    {
        let s = open(&path);
        for (i, id) in node_ids.iter().enumerate() {
            let emb = [i as f64, (20 - i) as f64, 0.5, 0.5];
            add_node(&s, id, emb, &["Person", "Test"]);
        }
        // Chain edges: node_0 → node_1 → ... → node_19
        for i in 0..19 {
            add_edge(
                &s,
                &format!("edge_{i}"),
                &node_ids[i],
                &node_ids[i + 1],
                "NEXT",
            );
            edge_count += 1;
        }
        s.flush_index();
        s.save_state().unwrap();
    }

    // Reopen from snapshot.
    let s = open(&path);
    for id in &node_ids {
        assert!(
            s.get_u32(id).is_some(),
            "{id} missing after snapshot reload"
        );
    }

    // Verify edge count.
    assert_eq!(
        s.edges.len(),
        edge_count,
        "edge count mismatch after reload"
    );

    // Verify vector search still works.
    let top = search_top1(&s, [0.0, 20.0, 0.5, 0.5]);
    assert_eq!(top, "node_0", "nearest neighbor should be node_0");
}

// ---------------------------------------------------------------------------
// 2. WAL-only round-trip: no snapshot, pure WAL replay
// ---------------------------------------------------------------------------
#[test]
fn wal_only_roundtrip() {
    let path = fresh("rebuild_wal_roundtrip");

    let node_ids: Vec<String> = (0..10).map(|i| format!("wnode_{i}")).collect();

    {
        let s = open(&path);
        for (i, id) in node_ids.iter().enumerate() {
            add_node(&s, id, [i as f64, 0.0, 0.0, 0.0], &["WALTest"]);
        }
        add_edge(&s, "wedge_01", "wnode_0", "wnode_1", "LINKED");
        s.flush_index();
        // Drop triggers save_state which creates snapshot + compacted WAL.
    }

    // Delete snapshot, keep WAL → forces WAL replay on reopen.
    let _ = fs::remove_file(Path::new(&path).join("state.json"));

    let s = open(&path);
    for id in &node_ids {
        assert!(s.get_u32(id).is_some(), "{id} missing after WAL replay");
    }
    assert!(!s.edges.is_empty(), "edges missing after WAL replay");

    // Vectors should be searchable after WAL replay.
    let top = search_top1(&s, [0.0, 0.0, 0.0, 0.0]);
    assert_eq!(top, "wnode_0");
}

// ---------------------------------------------------------------------------
// 3. Properties round-trip: node props survive save/reload
// ---------------------------------------------------------------------------
#[test]
fn node_properties_survive_roundtrip() {
    let path = fresh("rebuild_props_roundtrip");

    {
        let s = open(&path);
        s.add_node(NodeInput {
            id: Some("props_node".to_string()),
            labels: vec!["Data".to_string()],
            props: Some(serde_json::json!({
                "name": "GenesisBlock",
                "version": 42,
                "nested": {"key": "value"},
                "array": [1, 2, 3],
                "unicode": "สวัสดี 🌍"
            })),
            embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
            lang: Some("th".to_string()),
            valid_from: Some("2024-06-01T00:00:00Z".to_string()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        s.save_state().unwrap();
    }

    let s = open(&path);
    let u32_id = s.get_u32("props_node").expect("node should exist");
    let node = s.nodes.get(&u32_id).unwrap();
    let props = &node.props;
    assert_eq!(props["name"], "GenesisBlock");
    assert_eq!(props["version"], 42);
    assert_eq!(props["nested"]["key"], "value");
    assert_eq!(props["unicode"], "สวัสดี 🌍");
}

// ---------------------------------------------------------------------------
// 4. Edge properties round-trip
// ---------------------------------------------------------------------------
#[test]
fn edge_properties_survive_roundtrip() {
    let path = fresh("rebuild_edge_props");

    {
        let s = open(&path);
        add_node(&s, "ep_a", [1.0, 0.0, 0.0, 0.0], &["Node"]);
        add_node(&s, "ep_b", [0.0, 1.0, 0.0, 0.0], &["Node"]);
        s.add_edge(EdgeInput {
            id: Some("ep_edge".to_string()),
            from: "ep_a".to_string(),
            to: "ep_b".to_string(),
            rel: "WEIGHTED".to_string(),
            props: Some(serde_json::json!({"weight": 0.95, "type": "semantic"})),
            valid_from: Some("2024-03-01T00:00:00Z".to_string()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
        s.save_state().unwrap();
    }

    let s = open(&path);
    let edge = s.edges.iter().find(|e| e.id == "ep_edge");
    assert!(edge.is_some(), "edge should survive roundtrip");
    let edge = edge.unwrap();
    let props = &edge.props;
    assert_eq!(props["weight"], 0.95);
    assert_eq!(props["type"], "semantic");
}

// ---------------------------------------------------------------------------
// 5. Graph index integrity: out_idx / in_idx survive reload
// ---------------------------------------------------------------------------
#[test]
fn graph_indices_survive_roundtrip() {
    let path = fresh("rebuild_graph_idx");

    {
        let s = open(&path);
        add_node(&s, "gi_a", [1.0, 0.0, 0.0, 0.0], &["N"]);
        add_node(&s, "gi_b", [0.0, 1.0, 0.0, 0.0], &["N"]);
        add_node(&s, "gi_c", [0.0, 0.0, 1.0, 0.0], &["N"]);
        add_edge(&s, "gi_ab", "gi_a", "gi_b", "KNOWS");
        add_edge(&s, "gi_ac", "gi_a", "gi_c", "KNOWS");
        add_edge(&s, "gi_bc", "gi_b", "gi_c", "FOLLOWS");
        s.save_state().unwrap();
    }

    let s = open(&path);
    // out_idx for gi_a should have 2 outgoing edges.
    let a_id = s.get_u32("gi_a").unwrap();
    let out_edges = s.out_idx.get(&a_id);
    assert!(out_edges.is_some(), "out_idx should be rebuilt");
    assert_eq!(out_edges.unwrap().len(), 2, "gi_a should have 2 outgoing edges");

    // in_idx for gi_c should have 2 incoming edges.
    let c_id = s.get_u32("gi_c").unwrap();
    let in_edges = s.in_idx.get(&c_id);
    assert!(in_edges.is_some(), "in_idx should be rebuilt");
    assert_eq!(in_edges.unwrap().len(), 2, "gi_c should have 2 incoming edges");
}

// ---------------------------------------------------------------------------
// 6. Idempotent reload: open → save → reopen → save → reopen → identical
// ---------------------------------------------------------------------------
#[test]
fn idempotent_double_reload() {
    let path = fresh("rebuild_idempotent");

    {
        let s = open(&path);
        for i in 0..10 {
            add_node(&s, &format!("idem_{i}"), [i as f64, 0.0, 0.0, 0.0], &["I"]);
        }
        add_edge(&s, "idem_e01", "idem_0", "idem_1", "REL");
        s.save_state().unwrap();
    }

    // First reload + save.
    {
        let s = open(&path);
        assert_eq!(s.nodes.len(), 10);
        s.save_state().unwrap();
    }

    // Second reload + verify.
    {
        let s = open(&path);
        assert_eq!(s.nodes.len(), 10);
        for i in 0..10 {
            assert!(s.get_u32(&format!("idem_{i}")).is_some());
        }
        assert!(!s.edges.is_empty());
    }
}

// ---------------------------------------------------------------------------
// 7. Superseded nodes: version chain survives reload
// ---------------------------------------------------------------------------
#[test]
fn superseded_node_chain_survives() {
    let path = fresh("rebuild_supersede");

    {
        let s = open(&path);
        add_node(&s, "evolve_v1", [1.0, 0.0, 0.0, 0.0], &["Doc"]);
        // Supersede v1 with v2.
        s.add_node(NodeInput {
            id: Some("evolve_v2".to_string()),
            labels: vec!["Doc".to_string()],
            props: Some(serde_json::json!({"version": 2})),
            embedding: Some(vec![0.0, 1.0, 0.0, 0.0]),
            lang: Some("en".to_string()),
            valid_from: Some("2024-06-01T00:00:00Z".to_string()),
            caused_by: Some("evolve_v1".to_string()),
            ttl: None,
            collection: None,
        })
        .unwrap();
        s.save_state().unwrap();
    }

    let s = open(&path);
    assert!(s.get_u32("evolve_v1").is_some());
    assert!(s.get_u32("evolve_v2").is_some());
    let v2_id = s.get_u32("evolve_v2").unwrap();
    let v2 = s.nodes.get(&v2_id).unwrap();
    assert_eq!(
        v2.caused_by.as_deref(),
        Some("evolve_v1"),
        "caused_by chain should survive"
    );
}

// ---------------------------------------------------------------------------
// 8. Large batch: 1000 nodes + edges survive snapshot roundtrip
// ---------------------------------------------------------------------------
#[test]
fn large_batch_roundtrip() {
    let path = fresh("rebuild_large_batch");

    let n = 1000;
    {
        let s = open(&path);
        for i in 0..n {
            add_node(
                &s,
                &format!("bulk_{i}"),
                [(i % 100) as f64, ((i / 100) % 10) as f64, 0.0, 0.0],
                &["Bulk"],
            );
        }
        for i in 0..n - 1 {
            add_edge(
                &s,
                &format!("bulk_e_{i}"),
                &format!("bulk_{i}"),
                &format!("bulk_{}", i + 1),
                "SEQ",
            );
        }
        s.flush_index();
        s.save_state().unwrap();
    }

    let s = open(&path);
    assert_eq!(s.nodes.len(), n, "all {n} nodes should survive");
    assert_eq!(s.edges.len(), n - 1, "all {} edges should survive", n - 1);

    // Vector search on the reloaded state.
    let top = search_top1(&s, [0.0, 0.0, 0.0, 0.0]);
    assert!(
        top.starts_with("bulk_"),
        "vector search should work after large reload"
    );
}

// ---------------------------------------------------------------------------
// 9. Labels survive roundtrip
// ---------------------------------------------------------------------------
#[test]
fn labels_survive_roundtrip() {
    let path = fresh("rebuild_labels");

    {
        let s = open(&path);
        add_node(&s, "lbl_node", [1.0, 0.0, 0.0, 0.0], &["Alpha", "Beta", "Gamma"]);
        s.save_state().unwrap();
    }

    let s = open(&path);
    let u32_id = s.get_u32("lbl_node").unwrap();
    let node = s.nodes.get(&u32_id).unwrap();
    assert!(node.labels.contains(&"Alpha".to_string()));
    assert!(node.labels.contains(&"Beta".to_string()));
    assert!(node.labels.contains(&"Gamma".to_string()));
}

// ---------------------------------------------------------------------------
// 10. Logical clock persists across restarts
// ---------------------------------------------------------------------------
#[test]
fn logical_clock_persists() {
    let path = fresh("rebuild_clock");

    let clock_before;
    {
        let s = open(&path);
        for i in 0..5 {
            add_node(&s, &format!("clk_{i}"), [0.0; 4], &["C"]);
        }
        clock_before = s.get_logical_clock();
        assert!(clock_before > 0, "clock should advance");
        s.save_state().unwrap();
    }

    let s = open(&path);
    let clock_after = s.get_logical_clock();
    assert_eq!(
        clock_after, clock_before,
        "logical clock should persist: before={clock_before}, after={clock_after}"
    );
}
