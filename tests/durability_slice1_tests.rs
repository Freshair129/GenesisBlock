// Slice-1 tombstone retention (follow-up to RCA--SLICE0-DURABILITY defect 2).
//
// Slice 0 made node retraction durable (Event::NodeRetract), but the frame
// died at the next fold: build_compacted_wal emitted live state only, so
// (a) an anti-entropy peer that never pulled the retract frame re-pushed the
//     node and reconcile resurrected it (no local copy left to win LWW), and
// (b) retracted edges vanished from the journal at every fold, silently
//     breaking retract_edge's documented time-travel contract whenever the
//     snapshot was the only surviving copy.
//
// Slice 1 adds a tombstone registry (Storage.tombstones) that survives folds
// (NodeRetract frames re-emitted into the fold payload) and snapshots
// (state.json), gates CRDT Node upserts by clock LWW, and retains retracted
// edges in the fold within TOMBSTONE_RETENTION_SECS (30 days interim; policy
// moves to WP-1.3 retention profiles).

use genesis_block_native::{
    Event, LogicalClock, NeighborInput, NodeInput, NodeTombstone, OpenOptions, SignedEvent, Storage,
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
        vector_dim: Some(4),
        retention: None,
    })
    .unwrap()
}

fn add_node(s: &Storage) -> genesis_block_native::NodeOutput {
    s.add_node(NodeInput {
        id: Some("victim".to_string()),
        labels: vec![],
        props: None,
        embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
        lang: Some("en".to_string()),
        valid_from: Some("2024-01-01T00:00:00Z".to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap()
}

fn node_exists(s: &Storage, id: &str) -> bool {
    s.nodes.iter().any(|e| e.value().id == id)
}

/// A stale re-push of `node` as this store's own peer id — reconcile_state
/// self-trusts local events, so no real signature is needed. This is exactly
/// the disk image of "a peer still holding the pre-retraction node".
fn stale_push(node: &genesis_block_native::NodeOutput, s: &Storage) -> SignedEvent {
    SignedEvent {
        event: Event::Node(node.clone()),
        signature: vec![0; 64],
        signer_peer_id: s.local_peer_id.clone(),
    }
}

/// The core LWW claim: a retraction (newer clock) beats a stale peer re-push
/// even though the node no longer exists locally to compare against.
#[test]
fn stale_peer_push_does_not_resurrect_retracted_node() {
    let path = fresh("slice1_stale_push");
    let s = open(&path);
    let pre = add_node(&s);
    s.retract_node("victim").unwrap();

    s.reconcile_state(vec![stale_push(&pre, &s)]).unwrap();
    assert!(
        !node_exists(&s, "victim"),
        "stale peer re-push resurrected a retracted node (tombstone lost LWW)"
    );
}

/// The tombstone must survive BOTH persistence paths: the snapshot instant
/// load (state.json registry) and journal-only recovery (NodeRetract frames
/// re-emitted into the fold payload).
#[test]
fn tombstone_survives_fold_snapshot_and_journal_only_reopen() {
    let path = fresh("slice1_survive");
    let pre = {
        let s = open(&path);
        let pre = add_node(&s);
        s.retract_node("victim").unwrap();
        s.save_state().unwrap(); // fold + snapshot
        pre
    }; // Drop folds again.

    // (a) snapshot instant-load path
    {
        let s = open(&path);
        assert!(
            s.tombstones.contains_key("victim"),
            "tombstone registry empty after snapshot reopen"
        );
        s.reconcile_state(vec![stale_push(&pre, &s)]).unwrap();
        assert!(
            !node_exists(&s, "victim"),
            "resurrected after snapshot reopen"
        );
    }

    // (b) journal-only path: the base segment must carry the retraction.
    for f in ["state.json", "nodes.bin", "edges.bin"] {
        let p = Path::new(&path).join(f);
        if p.exists() {
            fs::remove_file(p).unwrap();
        }
    }
    let s = open(&path);
    assert!(
        s.tombstones.contains_key("victim"),
        "tombstone registry empty after journal-only recovery — the fold did not carry the NodeRetract frame"
    );
    s.reconcile_state(vec![stale_push(&pre, &s)]).unwrap();
    assert!(
        !node_exists(&s, "victim"),
        "resurrected after journal-only reopen"
    );
}

/// A genuinely newer upsert is a legitimate re-create: it clears the
/// tombstone, and the node stays alive across a reopen.
#[test]
fn newer_upsert_clears_tombstone() {
    let path = fresh("slice1_recreate");
    {
        let s = open(&path);
        add_node(&s);
        s.retract_node("victim").unwrap();
        add_node(&s); // fresh clock > tombstone clock
        assert!(
            !s.tombstones.contains_key("victim"),
            "legitimate re-create left the tombstone in place"
        );
        assert!(node_exists(&s, "victim"));
    }
    let s = open(&path);
    assert!(node_exists(&s, "victim"), "re-created node lost on reopen");
    assert!(!s.tombstones.contains_key("victim"));
}

/// retract_edge's documented time-travel contract must survive a fold even
/// when the journal is the only surviving copy: hidden from the current view,
/// visible with as_of before the retraction and with include_invalid.
#[test]
fn retracted_edge_time_travel_survives_fold() {
    let path = fresh("slice1_edge_ttravel");
    {
        let s = open(&path);
        for id in ["a", "b"] {
            s.add_node(NodeInput {
                id: Some(id.to_string()),
                labels: vec![],
                props: None,
                embedding: Some(vec![1.0, 0.0, 0.0, 0.0]),
                lang: Some("en".to_string()),
                valid_from: Some("2024-01-01T00:00:00Z".to_string()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
        }
        s.add_edge(genesis_block_native::EdgeInput {
            id: Some("e1".to_string()),
            from: "a".to_string(),
            to: "b".to_string(),
            rel: "REL".to_string(),
            props: None,
            valid_from: Some("2024-02-01T00:00:00Z".to_string()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
        s.retract_edge("e1".to_string(), None).unwrap();
        s.save_state().unwrap();
    }
    // Journal-only recovery: pre-Slice-1 the fold dropped every retracted
    // edge, so this reopen lost e1 entirely (not merely un-retracted it).
    for f in ["state.json", "nodes.bin", "edges.bin"] {
        let p = Path::new(&path).join(f);
        if p.exists() {
            fs::remove_file(p).unwrap();
        }
    }
    let s = open(&path);
    let nb = |as_of: Option<&str>, include_invalid: Option<bool>| {
        s.neighbors(
            "a".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: as_of.map(|x| x.to_string()),
                include_invalid,
                limit: None,
            },
            false,
        )
        .unwrap()
    };
    assert!(
        nb(None, None).iter().all(|n| n.node.id != "b"),
        "retracted edge visible in the current view"
    );
    assert!(
        nb(Some("2025-06-01T00:00:00Z"), None)
            .iter()
            .any(|n| n.node.id == "b"),
        "as_of before the retraction must see the edge (time-travel contract) — the fold dropped it"
    );
    assert!(
        nb(None, Some(true)).iter().any(|n| n.node.id == "b"),
        "include_invalid must see the retracted edge after journal-only recovery"
    );
}

/// The retention window is the fold's GC boundary: an expired tombstone is
/// dropped from the registry, the fold payload, and the snapshot.
#[test]
fn expired_tombstone_is_gcd_at_fold() {
    let path = fresh("slice1_gc");
    {
        let s = open(&path);
        // A tombstone 31 days in the past — beyond the 30-day interim window.
        s.tombstones.insert(
            "ancient".to_string(),
            NodeTombstone {
                clock: LogicalClock {
                    time: 1,
                    peer_id: s.local_peer_id.clone(),
                },
                retracted_at: (chrono::Utc::now() - chrono::Duration::days(31)).to_rfc3339(),
            },
        );
        // And a fresh one that must survive the same fold.
        add_node(&s);
        s.retract_node("victim").unwrap();
        s.save_state().unwrap();
        assert!(
            !s.tombstones.contains_key("ancient"),
            "expired tombstone survived the fold GC"
        );
        assert!(s.tombstones.contains_key("victim"));
    }
    let s = open(&path);
    assert!(!s.tombstones.contains_key("ancient"));
    assert!(s.tombstones.contains_key("victim"));
}
