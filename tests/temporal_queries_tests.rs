use genesis_block_native::{OpenOptions, NodeInput, EdgeInput, GenesisDatabase};
use serde_json::json;

#[tokio::test]
async fn test_temporal_time_travel() {
    let opts = OpenOptions {
        path: ".brain/test_temporal_db".to_string(),
        page_cache_mb: Some(10),
        read_only: Some(false),
    };
    let _ = std::fs::remove_dir_all(&opts.path);
    let db = GenesisDatabase::open(opts.clone()).unwrap();

    // 1. Create a node in the past (Year 2024)
    let _node_v1 = db.add_node(NodeInput {
        id: Some("Concept-A".to_string()),
        labels: vec!["Idea".to_string()],
        props: Some(json!({"desc": "Version 1"})),
        embedding: Some(vec![0.1; 1536]),
        lang: Some("en".to_string()),
        valid_from: Some("2024-01-01T00:00:00Z".to_string()),
    }).await.unwrap();

    // 2. Create another node in the past
    let _node_b = db.add_node(NodeInput {
        id: Some("Concept-B".to_string()),
        labels: vec!["Idea".to_string()],
        props: Some(json!({"desc": "Original B"})),
        embedding: Some(vec![0.2; 1536]),
        lang: Some("en".to_string()),
        valid_from: Some("2024-01-01T00:00:00Z".to_string()),
    }).await.unwrap();

    // 3. Connect them in the past
    let _edge_v1 = db.add_edge(EdgeInput {
        id: Some("E1".to_string()),
        from: "Concept-A".to_string(),
        to: "Concept-B".to_string(),
        rel: "RELATES_TO".to_string(),
        props: None,
        valid_from: Some("2024-02-01T00:00:00Z".to_string()),
        supersede: None,
        impact: Some(0.8),
    }).await.unwrap();

    // --- TIME TRAVEL QUERY 1: AS OF 2024 ---
    // Should see Concept A and Concept B connected.
    let hql_2024 = "TRAVERSE FROM \"Concept-A\" DEPTH 1 REL ANY AS OF \"2024-06-01T00:00:00Z\"";
    let results_2024 = db.execute_hql(hql_2024.to_string()).await.unwrap();
    assert_eq!(results_2024.len(), 1, "Should find 1 neighbor in 2024");
    assert_eq!(results_2024[0].node.id, "Concept-B");

    // 4. "Retract" the edge in 2025 by simulating a supersede (For now, we'll manually test valid_to filtering by retract_edge if implemented, or just simulate a future node that didn't exist in 2024)
    let _node_c_future = db.add_node(NodeInput {
        id: Some("Concept-C".to_string()),
        labels: vec!["FutureIdea".to_string()],
        props: Some(json!({"desc": "Born in 2026"})),
        embedding: Some(vec![0.3; 1536]),
        lang: Some("en".to_string()),
        valid_from: Some("2026-01-01T00:00:00Z".to_string()),
    }).await.unwrap();

    let _edge_v2 = db.add_edge(EdgeInput {
        id: Some("E2".to_string()),
        from: "Concept-A".to_string(),
        to: "Concept-C".to_string(),
        rel: "RELATES_TO".to_string(),
        props: None,
        valid_from: Some("2026-02-01T00:00:00Z".to_string()),
        supersede: None,
        impact: Some(0.9),
    }).await.unwrap();

    // --- TIME TRAVEL QUERY 2: AS OF 2025 ---
    // Concept-C did not exist yet! Traversal from Concept-A should ONLY yield Concept-B.
    let hql_2025 = "TRAVERSE FROM \"Concept-A\" DEPTH 1 REL ANY AS OF \"2025-01-01T00:00:00Z\"";
    let results_2025 = db.execute_hql(hql_2025.to_string()).await.unwrap();
    assert_eq!(results_2025.len(), 1, "Should still only find Concept-B in 2025");
    assert_eq!(results_2025[0].node.id, "Concept-B");

    // --- PRESENT DAY QUERY: AS OF 2027 ---
    // Should see BOTH Concept-B and Concept-C
    let hql_2027 = "TRAVERSE FROM \"Concept-A\" DEPTH 1 REL ANY AS OF \"2027-01-01T00:00:00Z\"";
    let results_2027 = db.execute_hql(hql_2027.to_string()).await.unwrap();
    assert_eq!(results_2027.len(), 2, "Should find both B and C in 2027");

    let _ = std::fs::remove_dir_all(&opts.path);
}
