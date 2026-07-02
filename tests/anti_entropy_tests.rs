// Gossip PullRequest anti-entropy (MARK XIV P5). The pull path was a no-op
// (`// TODO`). `events_since(from_clock)` is the source side: signed WAL events
// strictly newer than the requester's clock, sorted by logical time; a peer
// applies them with `reconcile_state` and converges. Tested at the Storage API
// level (the UDP loop just wires these two calls together).

use genesis_block_native::{
    EdgeInput, Event, HybridSearchInput, NeighborInput, NodeInput, OpenOptions, Storage, SyncPeer,
};
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
        vector_dim: None,
    })
    .unwrap()
}

fn node(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

fn edge(s: &Storage, id: &str, from: &str, to: &str) {
    s.add_edge(EdgeInput {
        id: Some(id.to_string()),
        from: from.to_string(),
        to: to.to_string(),
        rel: "REL".to_string(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
}

fn out_neighbors(s: &Storage, seed: &str) -> Vec<String> {
    s.neighbors(
        seed.to_string(),
        NeighborInput {
            depth: Some(1),
            rel: None,
            rels: None,
            direction: Some("out".to_string()),
            as_of: None,
            include_invalid: None,
            limit: None,
        },
        false,
    )
    .unwrap()
    .into_iter()
    .map(|n| n.node.id)
    .collect()
}

/// A peer behind another pulls its newer events and converges graph state: the
/// nodes and the edge between them become present and traversable.
#[test]
fn pull_delta_converges_graph_state() {
    let a = open(&fresh("test_ae_a"));
    node(&a, "X");
    node(&a, "Y");
    edge(&a, "E1", "X", "Y");

    let b = open(&fresh("test_ae_b"));
    // B must know A's verifying key to accept A's signed events.
    b.peers.insert(
        a.local_peer_id.clone(),
        SyncPeer {
            id: a.local_peer_id.clone(),
            addr: String::new(),
            last_seen: 0,
            verifying_key: a.verifying_key.to_bytes().to_vec(),
        },
    );

    let delta = a.events_since(0);
    assert!(!delta.is_empty(), "A has events newer than clock 0");
    b.reconcile_state(delta).unwrap();

    assert!(b.get_u32("X").is_some(), "node X synced to B");
    assert!(b.get_u32("Y").is_some(), "node Y synced to B");
    assert_eq!(
        out_neighbors(&b, "X"),
        vec!["Y".to_string()],
        "edge X->Y synced and traversable"
    );
}

/// `events_since` returns only events strictly newer than the given clock.
#[test]
fn events_since_filters_by_clock() {
    let a = open(&fresh("test_ae_filter"));
    node(&a, "N1");
    let mid = a.get_logical_clock(); // N1's time
    node(&a, "N2");

    let delta = a.events_since(mid);
    assert_eq!(delta.len(), 1, "only the post-`mid` event is returned");
    match &delta[0].event {
        Event::Node(n) => assert_eq!(n.id, "N2"),
        _ => panic!("expected the N2 node event"),
    }

    // Pulling from a clock at/after the latest event yields nothing.
    assert!(a.events_since(a.get_logical_clock()).is_empty());
}

/// Re-applying an already-seen delta is non-fatal and leaves state intact
/// (reconcile is LWW by clock).
#[test]
fn reapplying_delta_is_safe() {
    let a = open(&fresh("test_ae_a2"));
    node(&a, "Z");

    let b = open(&fresh("test_ae_b2"));
    b.peers.insert(
        a.local_peer_id.clone(),
        SyncPeer {
            id: a.local_peer_id.clone(),
            addr: String::new(),
            last_seen: 0,
            verifying_key: a.verifying_key.to_bytes().to_vec(),
        },
    );
    let delta = a.events_since(0);
    b.reconcile_state(delta.clone()).unwrap();
    b.reconcile_state(delta).unwrap(); // idempotent at the data level
    assert!(b.get_u32("Z").is_some());
}

/// Secondary embeddings (`add_vector`) now carry a clock, so they ride in pull
/// deltas (`events_since` used to drop them for lack of a clock). A peer that
/// lacks the collection auto-provisions it on receive, and the synced vector
/// becomes searchable there.
#[test]
fn pull_delta_syncs_secondary_vectors() {
    let a = open(&fresh("test_ae_vec_a"));
    a.create_collection(
        "code".to_string(),
        "m".to_string(),
        4,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    node(&a, "N");
    a.add_vector(
        "N".to_string(),
        "code".to_string(),
        vec![1.0, 0.0, 0.0, 0.0],
    )
    .unwrap();

    let b = open(&fresh("test_ae_vec_b")); // B has no "code" collection
    b.peers.insert(
        a.local_peer_id.clone(),
        SyncPeer {
            id: a.local_peer_id.clone(),
            addr: String::new(),
            last_seen: 0,
            verifying_key: a.verifying_key.to_bytes().to_vec(),
        },
    );

    let delta = a.events_since(0);
    assert!(
        delta.iter().any(|se| matches!(se.event, Event::Vector(_))),
        "the pull delta now includes the secondary vector (Event::Vector)"
    );

    b.reconcile_state(delta).unwrap();
    b.flush_index();
    let hits: Vec<String> = b
        .hybrid_search(HybridSearchInput {
            query_vector: vec![1.0, 0.0, 0.0, 0.0],
            k: 5,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: Some("code".to_string()),
            ef_search: None,
            oversample: None,
        })
        .unwrap()
        .into_iter()
        .map(|n| n.node.id)
        .collect();
    assert!(
        hits.contains(&"N".to_string()),
        "the secondary vector synced to B (collection auto-provisioned) and is searchable"
    );
}

/// `compact()` preserves live secondary vectors and writes readable SignedEvent
/// lines: after compaction + reload-from-WAL (no snapshot), the added vector
/// survives and stays searchable — secondary embeddings used to be dropped by
/// compaction, and compact's WAL output used to be unparseable on reload.
#[test]
fn compaction_preserves_secondary_vectors() {
    let path = fresh("test_ae_compact_vec");
    {
        let a = open(&path);
        a.create_collection(
            "code".to_string(),
            "m".to_string(),
            4,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        node(&a, "N");
        a.add_vector(
            "N".to_string(),
            "code".to_string(),
            vec![1.0, 0.0, 0.0, 0.0],
        )
        .unwrap();
        a.flush_index();
        a.compact().unwrap();
    }
    // Reopen WITHOUT a snapshot → replays the compacted WAL.
    let a = open(&path);
    a.flush_index();
    assert!(
        a.get_u32("N").is_some(),
        "node survived compaction + reload"
    );
    let hits: Vec<String> = a
        .hybrid_search(HybridSearchInput {
            query_vector: vec![1.0, 0.0, 0.0, 0.0],
            k: 5,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: Some("code".to_string()),
            ef_search: None,
            oversample: None,
        })
        .unwrap()
        .into_iter()
        .map(|n| n.node.id)
        .collect();
    assert!(
        hits.contains(&"N".to_string()),
        "secondary vector survived compaction + reload-from-WAL and is searchable"
    );
}
