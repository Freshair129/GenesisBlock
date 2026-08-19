// Bitemporal time-travel over TRAVERSE ... AS OF. Exercises the storage core
// directly (Storage, sync) so it builds and runs with or without the napi
// bindings (core/napi split, #161) — the async GenesisDatabase wrapper is just a
// thin offload over these same Storage methods.
use genesis_block_native::{EdgeInput, NeighborOutput, NodeInput, OpenOptions, Storage};
use serde_json::{from_value, json};

#[test]
fn test_temporal_time_travel() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_temporal_db");
    let opts = OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(10),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    };
    let _ = std::fs::remove_dir_all(path);
    let db = Storage::open(opts).unwrap();

    // 1. Create a node in the past (Year 2024)
    let _node_v1 = db
        .add_node(NodeInput {
            id: Some("Concept-A".to_string()),
            labels: vec!["Idea".to_string()],
            props: Some(json!({"desc": "Version 1"})),
            embedding: Some(vec![0.1; 1536]),
            lang: Some("en".to_string()),
            valid_from: Some("2024-01-01T00:00:00Z".to_string()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    // 2. Create another node in the past
    let _node_b = db
        .add_node(NodeInput {
            id: Some("Concept-B".to_string()),
            labels: vec!["Idea".to_string()],
            props: Some(json!({"desc": "Original B"})),
            embedding: Some(vec![0.2; 1536]),
            lang: Some("en".to_string()),
            valid_from: Some("2024-01-01T00:00:00Z".to_string()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    // 3. Connect them in the past
    let _edge_v1 = db
        .add_edge(EdgeInput {
            id: Some("E1".to_string()),
            from: "Concept-A".to_string(),
            to: "Concept-B".to_string(),
            rel: "RELATES_TO".to_string(),
            props: None,
            valid_from: Some("2024-02-01T00:00:00Z".to_string()),
            supersede: None,
            impact: Some(0.8),
            caused_by: None,
        })
        .unwrap();

    // --- TIME TRAVEL QUERY 1: AS OF 2024 ---
    let hql_2024 = "TRAVERSE FROM \"Concept-A\" DEPTH 1 REL ANY AS OF \"2024-06-01T00:00:00Z\"";
    let res_2024 = db.execute_hql(hql_2024).unwrap();
    let results_2024: Vec<NeighborOutput> = from_value(res_2024).unwrap();
    assert_eq!(results_2024.len(), 1, "Should find 1 neighbor in 2024");
    assert_eq!(results_2024[0].node.id, "Concept-B");

    // 4. Future node
    let _node_c_future = db
        .add_node(NodeInput {
            id: Some("Concept-C".to_string()),
            labels: vec!["FutureIdea".to_string()],
            props: Some(json!({"desc": "Born in 2026"})),
            embedding: Some(vec![0.3; 1536]),
            lang: Some("en".to_string()),
            valid_from: Some("2026-01-01T00:00:00Z".to_string()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    let _edge_v2 = db
        .add_edge(EdgeInput {
            id: Some("E2".to_string()),
            from: "Concept-A".to_string(),
            to: "Concept-C".to_string(),
            rel: "RELATES_TO".to_string(),
            props: None,
            valid_from: Some("2026-02-01T00:00:00Z".to_string()),
            supersede: None,
            impact: Some(0.9),
            caused_by: None,
        })
        .unwrap();

    // --- TIME TRAVEL QUERY 2: AS OF 2025 ---
    let hql_2025 = "TRAVERSE FROM \"Concept-A\" DEPTH 1 REL ANY AS OF \"2025-01-01T00:00:00Z\"";
    let res_2025 = db.execute_hql(hql_2025).unwrap();
    let results_2025: Vec<NeighborOutput> = from_value(res_2025).unwrap();
    assert_eq!(
        results_2025.len(),
        1,
        "Should still only find Concept-B in 2025"
    );
    assert_eq!(results_2025[0].node.id, "Concept-B");

    // --- PRESENT DAY QUERY: AS OF 2027 ---
    let hql_2027 = "TRAVERSE FROM \"Concept-A\" DEPTH 1 REL ANY AS OF \"2027-01-01T00:00:00Z\"";
    let res_2027 = db.execute_hql(hql_2027).unwrap();
    let results_2027: Vec<NeighborOutput> = from_value(res_2027).unwrap();
    assert_eq!(results_2027.len(), 2, "Should find both B and C in 2027");

    let _ = std::fs::remove_dir_all(path);
}

/// `supersede_node` advances the in-memory node's `valid_from` to now and
/// clears `valid_to`. A time-travel query issued before the supersession
/// timestamp can no longer see the node (its new valid_from is in the future
/// relative to the query), while queries in the far future find the updated
/// version. The current (no-AS-OF) view always returns the latest version.
#[test]
fn test_supersede_node_bitemporal_as_of() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_supersede_asof");
    let _ = std::fs::remove_dir_all(path);
    let db = Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(10),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();

    // Hub + target, both created in the distant past.
    db.add_node(NodeInput {
        id: Some("hub".to_string()),
        labels: vec!["Hub".to_string()],
        props: None,
        embedding: None,
        lang: None,
        valid_from: Some("2020-01-01T00:00:00Z".to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    db.add_node(NodeInput {
        id: Some("target".to_string()),
        labels: vec!["Target".to_string()],
        props: Some(json!({"version": 1})),
        embedding: None,
        lang: None,
        valid_from: Some("2020-01-01T00:00:00Z".to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    db.add_edge(EdgeInput {
        id: Some("hub-to-target".to_string()),
        from: "hub".to_string(),
        to: "target".to_string(),
        rel: "LINKS".to_string(),
        props: None,
        valid_from: Some("2020-01-01T00:00:00Z".to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();

    // Pre-supersession: target is visible in a past-time query.
    let pre: Vec<NeighborOutput> = from_value(
        db.execute_hql("TRAVERSE FROM hub DEPTH 1 REL ANY AS OF \"2022-01-01T00:00:00Z\"")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(pre.len(), 1, "target must be visible before supersession");
    assert_eq!(
        pre[0].node.props["version"], 1,
        "pre-supersession props must be v1"
    );

    // Supersede: valid_from advances to now (~2026), valid_to cleared.
    let new_ver = db
        .supersede_node("target".to_string(), Some(json!({"version": 2})), None)
        .unwrap();
    assert_eq!(
        new_ver.props["version"], 2,
        "supersede must return updated props"
    );

    // Current view (no AS OF): always sees the latest version.
    let cur: Vec<NeighborOutput> =
        from_value(db.execute_hql("TRAVERSE FROM hub DEPTH 1 REL ANY").unwrap()).unwrap();
    assert_eq!(cur.len(), 1, "target still reachable in current view");
    assert_eq!(cur[0].node.props["version"], 2, "current view has v2 props");

    // AS OF 2022 after supersession: in-memory node has valid_from ~now (2026)
    // which is after 2022, so it is filtered out.
    let past: Vec<NeighborOutput> = from_value(
        db.execute_hql("TRAVERSE FROM hub DEPTH 1 REL ANY AS OF \"2022-01-01T00:00:00Z\"")
            .unwrap(),
    )
    .unwrap();
    assert!(
        past.is_empty(),
        "superseded node has valid_from=now; must not appear in a 2022 AS OF query"
    );

    // AS OF far future: valid_from ~now < 2099, valid_to = None → visible.
    let future: Vec<NeighborOutput> = from_value(
        db.execute_hql("TRAVERSE FROM hub DEPTH 1 REL ANY AS OF \"2099-01-01T00:00:00Z\"")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        future.len(),
        1,
        "target must be visible in far-future AS OF query"
    );
    assert_eq!(
        future[0].node.props["version"], 2,
        "far-future query returns v2"
    );

    let _ = std::fs::remove_dir_all(path);
}
