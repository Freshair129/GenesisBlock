// Consensus commit correctness. Complements consensus_vote_sig_tests.rs (which
// covers vote authenticity). Here we pin the behaviour of the *commit* path once
// a proposal crosses quorum:
//   - a committed Edge is wired into the adjacency index (traversable now, not
//     just present in `edges` until the next reload)
//   - a committed Vector is staged + enqueued (searchable now)
//   - crossing quorum applies + persists the event exactly once; later approving
//     votes are short-circuited by the `committed` guard (no re-persist)
// See the consensus `submit_vote` block in src/lib.rs.

use genesis_block_native::{
    Storage, OpenOptions, NodeInput, NeighborInput, HybridSearchInput,
    Event, EdgeOutput, VectorEvent, NodeOutput, LogicalClock,
};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() { fs::remove_dir_all(&p).unwrap(); }
    p
}

fn open(path: &str, dim: u32) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(), page_cache_mb: Some(64), read_only: Some(false), vector_dim: Some(dim),
    }).unwrap()
}

fn node(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()), labels: vec![], props: None, embedding: None,
        lang: None, valid_from: None, caused_by: None, ttl: None, collection: None,
    }).unwrap();
}

fn mk_node(id: &str) -> NodeOutput {
    NodeOutput {
        id: id.to_string(), labels: vec!["USER".to_string()], props: serde_json::json!({}),
        impact: None, embedding: None, lang: None,
        valid_from: "2026-01-01T00:00:00Z".to_string(), valid_to: None, caused_by: None,
        expires_at: None, clock: LogicalClock { time: 1, peer_id: "p".to_string() }, collection: None,
    }
}

fn mk_edge(id: &str, from: &str, to: &str) -> EdgeOutput {
    EdgeOutput {
        id: id.to_string(), from: from.to_string(), to: to.to_string(), rel: "LINK".to_string(),
        props: serde_json::json!({}), valid_from: "2026-01-01T00:00:00Z".to_string(), valid_to: None,
        recorded_at: "2026-01-01T00:00:00Z".to_string(), superseded_by: None, impact: None,
        caused_by: None, clock: LogicalClock { time: 1, peer_id: "p".to_string() },
    }
}

fn hop1_out(s: &Storage, id: &str) -> usize {
    s.neighbors(id.to_string(), NeighborInput {
        depth: Some(1), rel: None, rels: None, direction: Some("out".to_string()),
        as_of: None, include_invalid: Some(false), limit: Some(1000),
    }, false).unwrap().len()
}

/// Self-sign + submit an approving vote, returning whether quorum was reached.
fn approve(s: &Storage, pid: &str) -> bool {
    let me = s.local_peer_id.clone();
    let sig = s.sign_vote(pid.to_string(), true);
    s.submit_vote(pid.to_string(), me, true, sig).unwrap()
}

fn wal_lines(path: &str) -> usize {
    let wal = format!("{}/genesis-graph.wal", path);
    match fs::read_to_string(&wal) {
        Ok(s) => s.lines().filter(|l| !l.trim().is_empty()).count(),
        Err(_) => 0,
    }
}

/// An Edge committed through consensus is indexed into the adjacency maps, so it
/// is traversable in this process — regression for the commit path that used to
/// insert into `edges` without calling `index_edge_internal`.
#[test]
fn consensus_committed_edge_is_traversable() {
    let s = open(&fresh("test_cc_edge"), 4);
    node(&s, "A");
    node(&s, "B");
    let pid = s.propose_consensus(Event::Edge(mk_edge("e-AB", "A", "B")), vec![0u8; 64]).unwrap();
    assert!(approve(&s, &pid), "self-vote reaches quorum");
    assert_eq!(hop1_out(&s, "A"), 1, "consensus-committed edge is in the adjacency index");
}

/// A Vector committed through consensus is staged + enqueued, so it is
/// searchable in this process — not merely persisted for a future replay.
#[test]
fn consensus_committed_vector_is_searchable() {
    let path = fresh("test_cc_vec");
    let s = open(&path, 4);
    node(&s, "N1"); // the vector attaches to an existing node
    let ev = Event::Vector(VectorEvent {
        node_id: "N1".to_string(), collection: None,
        embedding: vec![1.0, 0.0, 0.0, 0.0], lang: Some("en".to_string()),
        clock: LogicalClock { time: 1, peer_id: "p".to_string() },
    });
    let pid = s.propose_consensus(ev, vec![0u8; 64]).unwrap();
    assert!(approve(&s, &pid), "self-vote reaches quorum");

    s.flush_index();
    let hits: Vec<String> = s.hybrid_search(HybridSearchInput {
        query_vector: vec![1.0, 0.0, 0.0, 0.0], k: 5, alpha: Some(0.0),
        lang: None, as_of: None, collection: None, ef_search: None,
    }).unwrap().into_iter().map(|n| n.node.id).collect();
    assert!(hits.contains(&"N1".to_string()), "consensus-committed vector is searchable now");
}

/// Crossing quorum applies + persists exactly once. A second approving vote that
/// arrives after commit is short-circuited by the `committed` guard, so it does
/// not re-apply or re-persist the event (no WAL growth).
#[test]
fn recommit_after_quorum_does_not_repersist() {
    let path = fresh("test_cc_once");
    let s = open(&path, 4);
    // propose_consensus is in-memory only — nothing in the WAL yet.
    assert_eq!(wal_lines(&path), 0);

    let pid = s.propose_consensus(Event::Node(mk_node("N1")), vec![0u8; 64]).unwrap();
    assert!(approve(&s, &pid), "first authentic vote crosses quorum and commits");
    let after_commit = wal_lines(&path);
    assert_eq!(after_commit, 1, "commit persists the event once");

    // Re-vote (same peer, same authentic signature). The guard returns Ok(true)
    // without re-applying — the WAL must not grow.
    assert!(approve(&s, &pid), "a post-quorum vote still reports quorum reached");
    assert_eq!(wal_lines(&path), after_commit, "no re-persist after the proposal is committed");
}
