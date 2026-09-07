use genesis_block_native::*;
use serde_json::{json, Value};
use std::{path::Path, process::Command};

fn open(p: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: p.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}
fn node(id: &str) -> NodeInput {
    NodeInput {
        id: Some(id.into()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: Some("2020-01-01T00:00:00Z".into()),
        caused_by: None,
        ttl: None,
        collection: None,
    }
}
fn edge(from: &str, to: &str) -> EdgeInput {
    EdgeInput {
        id: Some("e".into()),
        from: from.into(),
        to: to.into(),
        rel: "R".into(),
        props: None,
        valid_from: Some("2020-01-01T00:00:00Z".into()),
        supersede: None,
        impact: None,
        caused_by: None,
    }
}
fn neighbors(s: &Storage, id: &str, t: Option<u64>, dir: &str) -> Vec<Value> {
    let mut req = json!({"contract_version":"query-ir.v1","request_id":"wave-b","operation":{"kind":"traverse","seed_id":id,"depth":1,"direction":dir,"relations":["R"]}});
    if let Some(t) = t {
        req["temporal"] = json!({"tx_as_of":t});
    }
    s.execute_query_ir_json(req).unwrap()["data"]
        .as_array()
        .unwrap()
        .clone()
}
#[test]
fn collection_crash_child() {
    let Ok(path) = std::env::var("GENESIS_WAVE_B_CHILD") else {
        return;
    };
    let s = open(Path::new(&path));
    for name in ["empty", "cos"] {
        s.create_collection(
            name.into(),
            "model-x".into(),
            2,
            Some("cosine".into()),
            Some("f16".into()),
            Some(123),
            Some(false),
        )
        .unwrap();
    }
    let mut n = node("vector");
    n.embedding = Some(vec![10., 0.]);
    n.collection = Some("cos".into());
    s.add_node(n).unwrap();
    std::process::exit(0);
}
#[test]
fn collection_definition_survives_exit_without_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "collection_crash_child", "--nocapture"])
        .env("GENESIS_WAVE_B_CHILD", dir.path())
        .status()
        .unwrap()
        .success());
    let s = open(dir.path());
    for name in ["empty", "cos"] {
        let c = s
            .list_collections()
            .into_iter()
            .find(|c| c.name == name)
            .expect("empty collection must also survive");
        assert_eq!(
            (c.model, c.dim, c.metric, c.quant, c.ef_search, c.rerank),
            (
                "model-x".into(),
                2,
                "Cosine".into(),
                "f16".into(),
                Some(123),
                false
            )
        );
    }
}
#[test]
fn invalid_definition_never_advances_frontier() {
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    let before = s.stable_frontier();
    for (dim, metric, quant) in [
        (0, "l2", "none"),
        (65537, "l2", "none"),
        (2, "bogus", "none"),
        (2, "l2", "bogus"),
    ] {
        assert!(s
            .create_collection(
                format!("bad{dim}{metric}{quant}"),
                "m".into(),
                dim,
                Some(metric.into()),
                Some(quant.into()),
                None,
                None
            )
            .is_err());
        assert_eq!(s.stable_frontier(), before);
    }
}
#[test]
fn rebind_current_and_multiple_historical_endpoints() {
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    for id in ["A", "B", "C", "D"] {
        s.add_node(node(id)).unwrap();
    }
    s.add_edge(edge("A", "B")).unwrap();
    let first = s.stable_frontier();
    s.add_edge(edge("C", "D")).unwrap();
    let second = s.stable_frontier();
    assert!(neighbors(&s, "A", None, "both").is_empty());
    assert!(neighbors(&s, "B", None, "both").is_empty());
    assert_eq!(neighbors(&s, "A", Some(first), "out")[0]["node"]["id"], "B");
    assert!(neighbors(&s, "C", Some(first), "out").is_empty());
    s.add_edge(edge("B", "A")).unwrap();
    assert_eq!(
        neighbors(&s, "C", Some(second), "out")[0]["node"]["id"],
        "D"
    );
    s.save_state().unwrap();
    drop(s);
    let s = open(dir.path());
    assert_eq!(neighbors(&s, "A", Some(first), "out")[0]["node"]["id"], "B");
    assert_eq!(
        neighbors(&s, "C", Some(second), "out")[0]["node"]["id"],
        "D"
    );
}

#[test]
fn repeated_rebind_retraction_recreation_and_projection_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    for id in ["A", "B", "C", "D"] {
        s.add_node(node(id)).unwrap();
    }
    s.add_edge(edge("A", "B")).unwrap();
    let first = s.stable_frontier();
    s.execute_batch(BatchInput {
        nodes: vec![],
        edges: vec![edge("C", "D"), edge("B", "C")],
    })
    .unwrap();
    let second = s.stable_frontier();
    assert!(neighbors(&s, "A", None, "both").is_empty());
    assert_eq!(
        neighbors(&s, "B", Some(second), "out")[0]["node"]["id"],
        "C"
    );
    s.retract_node("C").unwrap();
    assert!(neighbors(&s, "B", None, "out").is_empty());
    s.add_node(node("C")).unwrap();
    s.add_edge(edge("C", "A")).unwrap();
    assert_eq!(
        neighbors(&s, "B", Some(second), "out")[0]["node"]["id"],
        "C"
    );
    drop(s);
    std::fs::remove_file(dir.path().join("projection.sqlite")).unwrap();
    let s = open(dir.path());
    assert_eq!(neighbors(&s, "A", Some(first), "out")[0]["node"]["id"], "B");
    assert_eq!(
        neighbors(&s, "B", Some(second), "out")[0]["node"]["id"],
        "C"
    );
    s.add_edge(edge("A", "A")).unwrap();
    let key = Storage::edge_key("e");
    let a = s.get_u32("A").unwrap();
    assert!(s.out_idx.get(&a).unwrap().contains(&key));
    assert!(s.in_idx.get(&a).unwrap().contains(&key));
    assert!(!s
        .out_idx
        .get(&s.get_u32("C").unwrap())
        .unwrap()
        .contains(&key));
    s.compact().unwrap();
    assert!(s.execute_query_ir_json(json!({"contract_version":"query-ir.v1","request_id":"old","operation":{"kind":"traverse","seed_id":"A","depth":1,"direction":"out","relations":["R"]},"temporal":{"tx_as_of":first}})).is_err());
}

#[test]
fn losing_legacy_retraction_does_not_erase_edge_history() {
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    s.add_node(node("A")).unwrap();
    s.add_node(node("B")).unwrap();
    s.add_edge(edge("A", "B")).unwrap();
    s.add_node(node("A")).unwrap();
    // Model a historical raw frame accepted by an older writer.
    let frontier = s.stable_frontier() + 1;
    drop(s);
    let payload = serde_json::to_vec(&SignedEvent {
        event: Event::NodeRetract {
            id: "A".into(),
            clock: LogicalClock {
                time: 0,
                peer_id: "legacy".into(),
            },
            retracted_at: "2020-01-01T00:00:00Z".into(),
        },
        signature: vec![],
        signer_peer_id: "legacy".into(),
    })
    .unwrap();
    let path = dir.path().join("wal/active.gwal");
    let mut bytes = std::fs::read(&path).unwrap();
    let mut crc = frontier.to_le_bytes().to_vec();
    crc.extend_from_slice(&payload);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&frontier.to_le_bytes());
    bytes.extend_from_slice(&crc32c::crc32c(&crc).to_le_bytes());
    bytes.extend_from_slice(&payload);
    std::fs::write(path, bytes).unwrap();
    std::fs::remove_file(dir.path().join("projection.sqlite")).unwrap();
    let s = open(dir.path());
    assert_eq!(
        neighbors(&s, "A", Some(frontier), "out")[0]["node"]["id"],
        "B"
    );
}

#[test]
fn transaction_and_consensus_rebind_share_current_and_historical_endpoints() {
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    for id in ["A", "B", "C", "D"] {
        s.add_node(node(id)).unwrap();
    }
    s.add_edge(edge("A", "B")).unwrap();
    let first = s.stable_frontier();
    s.commit_transaction(GenesisTransaction {
        transaction_id: "wave-b-rebind".into(),
        expected_frontier: Some(0),
        relational: vec![],
        graph: BatchInput {
            nodes: vec![],
            edges: vec![edge("C", "D")],
        },
        vectors: vec![],
    })
    .unwrap();
    let second = s.stable_frontier();
    let mut e = s.edges.get(&Storage::edge_key("e")).unwrap().clone();
    e.from = "B".into();
    e.to = "A".into();
    e.clock.time += 1;
    let pid = s.propose_consensus(Event::Edge(e), vec![]).unwrap();
    let sig = s.sign_vote(pid.clone(), true);
    assert!(s
        .submit_vote(pid, s.local_peer_id.clone(), true, sig)
        .unwrap());
    assert!(neighbors(&s, "C", None, "out").is_empty());
    assert_eq!(neighbors(&s, "B", None, "out")[0]["node"]["id"], "A");
    assert_eq!(neighbors(&s, "A", Some(first), "out")[0]["node"]["id"], "B");
    assert_eq!(
        neighbors(&s, "C", Some(second), "out")[0]["node"]["id"],
        "D"
    );
    let mut stale = s.edges.get(&Storage::edge_key("e")).unwrap().clone();
    stale.from = "C".into();
    stale.to = "D".into();
    stale.clock.time = 0;
    let pid = s.propose_consensus(Event::Edge(stale), vec![]).unwrap();
    let sig = s.sign_vote(pid.clone(), true);
    assert!(s
        .submit_vote(pid, s.local_peer_id.clone(), true, sig)
        .unwrap());
    assert_eq!(neighbors(&s, "B", None, "out")[0]["node"]["id"], "A");
    assert_eq!(
        neighbors(&s, "B", Some(s.stable_frontier()), "out")[0]["node"]["id"],
        "A"
    );
    let pid = s
        .propose_consensus(
            Event::NodeRetract {
                id: "A".into(),
                clock: LogicalClock {
                    time: 0,
                    peer_id: "old".into(),
                },
                retracted_at: "2020-01-01T00:00:00Z".into(),
            },
            vec![],
        )
        .unwrap();
    let sig = s.sign_vote(pid.clone(), true);
    assert!(s
        .submit_vote(pid, s.local_peer_id.clone(), true, sig)
        .unwrap());
    assert_eq!(neighbors(&s, "B", None, "out")[0]["node"]["id"], "A");
}
