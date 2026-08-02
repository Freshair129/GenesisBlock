//! Bitemporal model integration tests.
//!
//! Covers `supersede_node`, `retract_edge`, logical clock monotonicity,
//! `as_of` time-travel queries, and TTL / `expires_at`.

use genesis_block_native::{EdgeInput, NeighborInput, NodeInput, OpenOptions, QueryInput, Storage};
use serde_json::json;
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let db_path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&db_path).exists() {
        fs::remove_dir_all(&db_path).unwrap();
    }
    db_path
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

/// Helper: add a bare node with optional props.
fn add_node(s: &Storage, id: &str, props: Option<serde_json::Value>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

// -----------------------------------------------------------------------
// 1. supersede_node updates the current props
// -----------------------------------------------------------------------
#[test]
fn supersede_updates_current() {
    let p = fresh("bt_supersede_current");
    let s = open(&p);

    s.add_node(NodeInput {
        id: Some("s1".to_string()),
        labels: vec![],
        props: Some(json!({"name": "old"})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    let updated = s
        .supersede_node("s1".to_string(), Some(json!({"name": "new"})), None)
        .unwrap();
    assert_eq!(updated.props["name"], "new", "supersede must update props");

    // Also verify via the nodes map.
    let stored = s.node_view("s1").unwrap();
    assert_eq!(stored.props["name"], "new");
}

// -----------------------------------------------------------------------
// 2. supersede_node preserves labels
// -----------------------------------------------------------------------
#[test]
fn supersede_preserves_labels() {
    let p = fresh("bt_supersede_labels");
    let s = open(&p);

    s.add_node(NodeInput {
        id: Some("lab1".to_string()),
        labels: vec!["A".to_string(), "B".to_string()],
        props: Some(json!({"v": 1})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    let updated = s
        .supersede_node("lab1".to_string(), Some(json!({"v": 2})), None)
        .unwrap();
    assert_eq!(
        updated.labels,
        vec!["A".to_string(), "B".to_string()],
        "labels must survive supersede"
    );
}

// -----------------------------------------------------------------------
// 3. supersede_node sets caused_by
// -----------------------------------------------------------------------
#[test]
fn supersede_sets_caused_by() {
    let p = fresh("bt_supersede_caused");
    let s = open(&p);

    add_node(&s, "c1", Some(json!({"v": 1})));

    let updated = s
        .supersede_node(
            "c1".to_string(),
            Some(json!({"v": 2})),
            Some("migration-1".to_string()),
        )
        .unwrap();

    assert_eq!(
        updated.caused_by.as_deref(),
        Some("migration-1"),
        "caused_by must be set on the superseded node"
    );
}

// -----------------------------------------------------------------------
// 4. supersede_node on nonexistent node -> Err
// -----------------------------------------------------------------------
#[test]
fn supersede_nonexistent_node_errors() {
    let p = fresh("bt_supersede_ghost");
    let s = open(&p);

    let res = s.supersede_node("ghost".to_string(), Some(json!({"x": 1})), None);
    assert!(res.is_err(), "supersede on missing node must error");
}

// -----------------------------------------------------------------------
// 5. retract_edge sets valid_to
// -----------------------------------------------------------------------
#[test]
fn edge_retract_sets_valid_to() {
    let p = fresh("bt_retract_valid_to");
    let s = open(&p);

    add_node(&s, "A", None);
    add_node(&s, "B", None);
    let e = s
        .add_edge(EdgeInput {
            id: Some("E1".to_string()),
            from: "A".to_string(),
            to: "B".to_string(),
            rel: "LINK".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    assert!(e.valid_to.is_none(), "fresh edge has no valid_to");

    let retracted = s.retract_edge("E1".to_string(), None).unwrap();
    assert!(retracted.is_some(), "retract must return the edge");
    let retracted = retracted.unwrap();
    assert!(
        retracted.valid_to.is_some(),
        "retracted edge must have valid_to set"
    );

    // Cross-check via query with include_invalid.
    let edges = s
        .query(QueryInput {
            from: Some("A".to_string()),
            to: None,
            rel: None,
            as_of: None,
            include_invalid: Some(true),
            limit: None,
        })
        .unwrap();

    let e1 = edges.iter().find(|e| e.id == "E1").unwrap();
    assert!(
        e1.valid_to.is_some(),
        "queried retracted edge must have valid_to"
    );
}

// -----------------------------------------------------------------------
// 6. Logical clock is monotonically increasing across writes
// -----------------------------------------------------------------------
#[test]
fn logical_clock_monotonic() {
    let p = fresh("bt_clock_mono");
    let s = open(&p);

    let mut clocks = Vec::new();
    for i in 0..5 {
        let n = s
            .add_node(NodeInput {
                id: Some(format!("n{}", i)),
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
        clocks.push(n.clock.time);
    }

    for w in clocks.windows(2) {
        assert!(
            w[1] > w[0],
            "clock must be strictly increasing: {} -> {}",
            w[0],
            w[1]
        );
    }
}

// -----------------------------------------------------------------------
// 7. Logical clock peer_id is stable across writes
// -----------------------------------------------------------------------
#[test]
fn logical_clock_peer_id_stable() {
    let p = fresh("bt_clock_peer");
    let s = open(&p);

    let n1 = s
        .add_node(NodeInput {
            id: Some("p1".to_string()),
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

    let n2 = s
        .add_node(NodeInput {
            id: Some("p2".to_string()),
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

    assert_eq!(
        n1.clock.peer_id, n2.clock.peer_id,
        "peer_id must be stable within one Storage instance"
    );
    assert!(!n1.clock.peer_id.is_empty(), "peer_id must not be empty");
}

// -----------------------------------------------------------------------
// 8. as_of query filters edges by time
// -----------------------------------------------------------------------
#[test]
fn as_of_query_filters_by_time() {
    let p = fresh("bt_as_of_filter");
    let s = open(&p);

    add_node(&s, "X", None);
    add_node(&s, "Y", None);

    // Use an explicit valid_from far in the future so that an as_of query
    // set to the past will not see it.
    let future_time = "2099-01-01T00:00:00Z";
    s.add_edge(EdgeInput {
        id: Some("EF".to_string()),
        from: "X".to_string(),
        to: "Y".to_string(),
        rel: "LINK".to_string(),
        props: None,
        valid_from: Some(future_time.to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();

    // Neighbors with as_of in the past should not see the future edge.
    let past = "2020-01-01T00:00:00Z";
    let nbrs = s
        .neighbors(
            "X".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: Some(past.to_string()),
                include_invalid: Some(false),
                limit: None,
            },
            false,
        )
        .unwrap();

    assert!(
        nbrs.is_empty(),
        "as_of before edge valid_from must hide the edge"
    );

    // Without as_of (current view), the edge valid_from is in the future,
    // so whether it appears depends on the engine's interpretation. We just
    // verify the call doesn't crash.
    let nbrs_now = s
        .neighbors(
            "X".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: Some(false),
                limit: None,
            },
            false,
        )
        .unwrap();

    // Document: engine may or may not show future-valid_from edges in the
    // current view. This is engine-specific behaviour.
    println!(
        "current-view returned {} neighbors for future-dated edge",
        nbrs_now.len()
    );
}

// -----------------------------------------------------------------------
// 9. Edge temporal retract + as_of time-travel
// -----------------------------------------------------------------------
#[test]
fn edge_temporal_retract_as_of() {
    let p = fresh("bt_retract_as_of");
    let s = open(&p);

    // Nodes must have valid_from before the as_of query point,
    // otherwise the node-level time-travel check filters them out.
    let early = "2019-01-01T00:00:00Z";
    s.add_node(NodeInput {
        id: Some("M".to_string()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: Some(early.to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
    s.add_node(NodeInput {
        id: Some("N".to_string()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: Some(early.to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    let past = "2020-06-01T00:00:00Z";
    s.add_edge(EdgeInput {
        id: Some("ET".to_string()),
        from: "M".to_string(),
        to: "N".to_string(),
        rel: "REL".to_string(),
        props: None,
        valid_from: Some(past.to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();

    // Retract at a time after valid_from but before "now" (use an explicit
    // timestamp so the test is deterministic).
    let retract_at = "2024-01-01T00:00:00Z";
    s.retract_edge("ET".to_string(), Some(retract_at.to_string()))
        .unwrap();

    // as_of before the retraction (but after valid_from) should see the edge.
    let mid = "2022-01-01T00:00:00Z";
    let nbrs_mid = s
        .neighbors(
            "M".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: Some(mid.to_string()),
                include_invalid: Some(false),
                limit: None,
            },
            false,
        )
        .unwrap();
    assert_eq!(
        nbrs_mid.len(),
        1,
        "as_of between valid_from and valid_to must show the edge"
    );

    // Current view (after retraction) with include_invalid=false should NOT
    // see the retracted edge.
    let nbrs_current = s
        .neighbors(
            "M".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: Some(false),
                limit: None,
            },
            false,
        )
        .unwrap();
    assert!(
        nbrs_current.is_empty(),
        "current view with include_invalid=false must hide retracted edge"
    );
}

// -----------------------------------------------------------------------
// 10. TTL sets expires_at
// -----------------------------------------------------------------------
#[test]
fn ttl_sets_expires_at() {
    let p = fresh("bt_ttl_expires");
    let s = open(&p);

    let n = s
        .add_node(NodeInput {
            id: Some("ttl-node".to_string()),
            labels: vec![],
            props: None,
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: Some(3600), // 1 hour
            collection: None,
        })
        .unwrap();

    assert!(
        n.expires_at.is_some(),
        "node with ttl must have expires_at set"
    );

    // Verify it's a parseable RFC3339 timestamp.
    let expires = n.expires_at.as_ref().unwrap();
    assert!(
        expires.contains("T"),
        "expires_at must look like an RFC3339 timestamp, got: {}",
        expires
    );

    // Cross-check via nodes map.
    let uid = s.get_u32("ttl-node").unwrap();
    let stored = s.nodes.get(&uid).unwrap();
    assert!(
        stored.expires_at.is_some(),
        "stored node must also have expires_at"
    );
}
