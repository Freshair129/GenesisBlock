//! Storage reliability integration tests for GenesisBlockDB.
//!
//! Tests persistence (WAL replay, snapshot reload), read-only mode,
//! duplicate-ID semantics, vector round-trip, and edge-case behaviour.

use genesis_block_native::{
    EdgeInput, HybridSearchInput, NodeInput, OpenOptions, QueryInput, Storage,
};
use serde_json::json;
use std::fs;
use std::path::Path;

// ── helpers ──────────────────────────────────────────────────────────

fn fresh(name: &str) -> String {
    let db_path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&db_path).exists() {
        let _ = fs::remove_dir_all(&db_path);
    }
    db_path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
    })
    .unwrap()
}

fn open_vec(path: &str, dim: u32) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(dim),
    })
    .unwrap()
}

fn open_ro(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(true),
        vector_dim: None,
    })
    .unwrap()
}

fn node(id: &str, labels: &[&str], props: serde_json::Value) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: labels.iter().map(|s| s.to_string()).collect(),
        props: Some(props),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

fn edge(id: &str, from: &str, to: &str, rel: &str, props: serde_json::Value) -> EdgeInput {
    EdgeInput {
        id: Some(id.to_string()),
        from: from.to_string(),
        to: to.to_string(),
        rel: rel.to_string(),
        props: Some(props),
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    }
}

// ── 1. snapshot persistence: nodes ───────────────────────────────────

#[test]
fn open_close_reopen_node_persists() {
    let p = fresh("sr_node_persist");
    {
        let s = open(&p);
        s.add_node(node("n1", &["Person"], json!({"name": "Alice"})))
            .unwrap();
        s.save_state().unwrap();
    } // drop

    let s = open(&p);
    let uid = s.get_u32("n1").expect("n1 should be interned after reopen");
    let n = s.nodes.get(&uid).expect("node should exist in DashMap");
    assert_eq!(n.id, "n1");
    assert!(n.labels.contains(&"Person".to_string()));
    assert_eq!(n.props["name"], "Alice");
}

// ── 2. snapshot persistence: edges ───────────────────────────────────

#[test]
fn open_close_reopen_edge_persists() {
    let p = fresh("sr_edge_persist");
    {
        let s = open(&p);
        s.add_node(node("a", &["N"], json!({}))).unwrap();
        s.add_node(node("b", &["N"], json!({}))).unwrap();
        s.add_edge(edge("e1", "a", "b", "KNOWS", json!({"since": 2020})))
            .unwrap();
        s.save_state().unwrap();
    }

    let s = open(&p);
    let edges = s
        .query(QueryInput {
            from: Some("a".into()),
            to: Some("b".into()),
            rel: None,
            as_of: None,
            include_invalid: None,
            limit: None,
        })
        .unwrap();
    assert!(!edges.is_empty(), "edge should survive reopen");
    let e = &edges[0];
    assert_eq!(e.from, "a");
    assert_eq!(e.to, "b");
    assert_eq!(e.rel, "KNOWS");
    assert_eq!(e.props["since"], 2020);
}

// ── 3. WAL durability without snapshot ───────────────────────────────

#[test]
fn wal_durability_without_snapshot() {
    let p = fresh("sr_wal_nodes");
    {
        let s = open(&p);
        for i in 0..50 {
            s.add_node(node(&format!("wal-{}", i), &["Wal"], json!({"i": i})))
                .unwrap();
        }
        // deliberately do NOT call save_state
    }

    let s = open(&p);
    for i in 0..50 {
        let id = format!("wal-{}", i);
        assert!(
            s.get_u32(&id).is_some(),
            "node {} should survive WAL replay",
            id
        );
    }
    assert!(s.nodes.len() >= 50);
}

// ── 4. WAL durability: edges ─────────────────────────────────────────

#[test]
fn wal_durability_edges() {
    let p = fresh("sr_wal_edges");
    {
        let s = open(&p);
        for i in 0..10 {
            s.add_node(node(&format!("wn-{}", i), &["N"], json!({})))
                .unwrap();
        }
        for i in 0..20 {
            let from = format!("wn-{}", i % 10);
            let to = format!("wn-{}", (i + 1) % 10);
            s.add_edge(edge(
                &format!("we-{}", i),
                &from,
                &to,
                "LINK",
                json!({"i": i}),
            ))
            .unwrap();
        }
        // no save_state
    }

    let s = open(&p);
    assert!(
        s.edges.len() >= 20,
        "expected >= 20 edges after WAL replay, got {}",
        s.edges.len()
    );
}

// ── 5. snapshot reload with vectors ──────────────────────────────────

#[test]
fn snapshot_reload_with_vectors() {
    let p = fresh("sr_vec_reload");
    {
        let s = open_vec(&p, 4);
        s.add_node(NodeInput {
            id: Some("v1".into()),
            labels: vec!["Vec".into()],
            props: Some(json!({"tag": "unit-x"})),
            embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        s.flush_index();
        s.save_state().unwrap();
    }

    let s = open_vec(&p, 4);
    assert!(s.get_u32("v1").is_some(), "vector node should persist");

    let results = s
        .hybrid_search(HybridSearchInput {
            query_vector: vec![1.0, 0.0, 0.0, 0.0],
            k: 1,
            alpha: None,
            lang: None,
            as_of: None,
            collection: None,
            ef_search: None,
        })
        .unwrap();
    assert!(!results.is_empty(), "hybrid_search should return the node");
    assert_eq!(results[0].node.id, "v1");
}

// ── 6. unknown edge endpoint: no panic ───────────────────────────────

#[test]
fn unknown_edge_endpoint_returns_error_not_panic() {
    let p = fresh("sr_dangling_edge");
    let s = open(&p);
    // The engine does NOT validate that from/to nodes exist, so this
    // should succeed and store a dangling edge.
    let result = s.add_edge(edge("ghost-e", "ghost_a", "ghost_b", "HAUNTS", json!({})));
    match result {
        Ok(e) => {
            // Dangling edges are allowed -- verify it is stored.
            assert_eq!(e.from, "ghost_a");
            assert_eq!(e.to, "ghost_b");
            assert!(!s.edges.is_empty());
        }
        Err(e) => {
            // If the engine rejects it, that's fine too -- just no panic.
            let msg = format!("{}", e);
            assert!(!msg.is_empty(), "error should have a descriptive message");
        }
    }
    // The key invariant: we reached this line without panicking.
}

// ── 7. stable string identity across reopen ──────────────────────────

#[test]
fn stable_string_identity_across_reopen() {
    let p = fresh("sr_string_identity");
    {
        let s = open(&p);
        s.add_node(node("X", &["N"], json!({}))).unwrap();
        s.add_node(node("Y", &["N"], json!({}))).unwrap();
        s.add_edge(edge("xy", "X", "Y", "LINKS", json!({})))
            .unwrap();
        s.save_state().unwrap();
    }

    let s = open(&p);
    let edges = s
        .query(QueryInput {
            from: Some("X".into()),
            to: Some("Y".into()),
            rel: None,
            as_of: None,
            include_invalid: None,
            limit: None,
        })
        .unwrap();
    assert!(!edges.is_empty(), "edge X->Y should survive reopen");
    assert_eq!(edges[0].from, "X", "from must be the string ID, not a u32");
    assert_eq!(edges[0].to, "Y", "to must be the string ID, not a u32");
}

// ── 8. read-only blocks writes ───────────────────────────────────────

#[test]
fn read_only_blocks_writes() {
    let p = fresh("sr_ro_blocks");
    {
        let s = open(&p);
        s.add_node(node("keeper", &["N"], json!({"k": 1}))).unwrap();
        s.save_state().unwrap();
    }

    let s = open_ro(&p);

    // Reading should work.
    assert!(
        s.get_u32("keeper").is_some(),
        "read-only should still see persisted nodes"
    );

    // Writing should fail with an error, not a panic.
    let add_node_result = s.add_node(node("intruder", &["N"], json!({})));
    assert!(
        add_node_result.is_err(),
        "add_node must fail on read-only storage"
    );

    let add_edge_result = s.add_edge(edge("bad-e", "keeper", "keeper", "SELF", json!({})));
    assert!(
        add_edge_result.is_err(),
        "add_edge must fail on read-only storage"
    );
}

// ── 9. read-only does not mutate ─────────────────────────────────────

#[test]
fn read_only_does_not_mutate() {
    let p = fresh("sr_ro_no_mutate");
    {
        let s = open(&p);
        s.add_node(node("original", &["N"], json!({"v": 1})))
            .unwrap();
        s.save_state().unwrap();
    }

    // Open read-only, attempt a write (which should fail), then drop.
    {
        let s = open_ro(&p);
        let _ = s.add_node(node("sneaky", &["N"], json!({})));
        // drop without save
    }

    // Reopen writable -- only the original node should exist.
    let s = open(&p);
    assert!(
        s.get_u32("original").is_some(),
        "original node must survive"
    );
    assert!(
        s.get_u32("sneaky").is_none(),
        "sneaky node must NOT have been persisted through read-only"
    );
}

// ── 10. duplicate node ID behaviour ──────────────────────────────────

#[test]
fn duplicate_node_id_behavior() {
    let p = fresh("sr_dup_node");
    let s = open(&p);

    s.add_node(node("dup", &["First"], json!({"version": 1})))
        .unwrap();
    s.add_node(node("dup", &["Second"], json!({"version": 2})))
        .unwrap();

    // The engine silently overwrites (last-write-wins for same id).
    let uid = s.get_u32("dup").expect("dup should be interned");
    let n = s.nodes.get(&uid).expect("node should exist");
    // The latest write's data should be visible.
    assert_eq!(
        n.props["version"], 2,
        "last write wins: props should reflect the second add_node"
    );

    // There should be exactly one logical node for "dup", even though
    // internal structures may have extra version entries.  At minimum,
    // the u32 slot is unique.
    let uid2 = s.get_u32("dup").unwrap();
    assert_eq!(uid, uid2, "interned id must be stable");
}

// ── 11. status reflects counts ───────────────────────────────────────

#[test]
fn status_reflects_node_edge_counts() {
    let p = fresh("sr_status_counts");
    let s = open(&p);

    for i in 0..5 {
        s.add_node(node(&format!("sc-{}", i), &["N"], json!({})))
            .unwrap();
    }
    for i in 0..3 {
        let from = format!("sc-{}", i);
        let to = format!("sc-{}", i + 1);
        s.add_edge(edge(&format!("se-{}", i), &from, &to, "SEQ", json!({})))
            .unwrap();
    }

    let st = s.status_sync();
    assert!(st.open, "status.open should be true");

    // DatabaseStatus doesn't carry counts, so verify via public fields.
    assert_eq!(s.nodes.len(), 5, "should have 5 nodes");
    assert_eq!(s.edges.len(), 3, "should have 3 edges");
}

// ── 12. empty database status ────────────────────────────────────────

#[test]
fn empty_database_status() {
    let p = fresh("sr_empty_status");
    let s = open(&p);

    let st = s.status_sync();
    assert!(st.open, "fresh db should report open");
    assert!(!st.read_only, "fresh db should not be read_only");
    assert_eq!(s.nodes.len(), 0, "fresh db should have 0 nodes");
    assert_eq!(s.edges.len(), 0, "fresh db should have 0 edges");
}

// ── 13. large props preserved ────────────────────────────────────────

#[test]
fn large_props_preserved() {
    let p = fresh("sr_large_props");
    let filler: String = "x".repeat(100);
    let mut map = serde_json::Map::new();
    for i in 0..50 {
        map.insert(format!("key_{}", i), json!(format!("{}_{}", filler, i)));
    }
    let big_props = serde_json::Value::Object(map.clone());

    {
        let s = open(&p);
        s.add_node(NodeInput {
            id: Some("big".into()),
            labels: vec!["Large".into()],
            props: Some(big_props),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
        s.save_state().unwrap();
    }

    let s = open(&p);
    let uid = s.get_u32("big").expect("big node should be interned");
    let n = s.nodes.get(&uid).expect("big node should exist");
    let obj = n.props.as_object().expect("props should be an object");
    assert_eq!(obj.len(), 50, "all 50 keys must survive round-trip");
    for i in 0..50 {
        let key = format!("key_{}", i);
        let expected = format!("{}_{}", filler, i);
        assert_eq!(
            obj[&key].as_str().unwrap(),
            expected,
            "value for {} must match",
            key
        );
    }
}
