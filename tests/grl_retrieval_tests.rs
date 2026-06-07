use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput, ContextPackage};
use std::sync::Arc;
use tempfile::tempdir;
use serde_json::json;

#[test]
fn test_grl_context_retrieval_tiered() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    }).unwrap();

    // 1. Setup a small graph: A -> B -> C
    storage.add_node(NodeInput {
        id: Some("A".to_string()), labels: vec!["USER".to_string()], props: Some(json!({"text": "Node A content"})),
        embedding: None, lang: None, valid_from: None, caused_by: None, ttl: None,
    }).unwrap();

    storage.add_node(NodeInput {
        id: Some("B".to_string()), labels: vec!["USER".to_string()], props: Some(json!({"text": "Node B content"})),
        embedding: None, lang: None, valid_from: None, caused_by: None, ttl: None,
    }).unwrap();

    storage.add_node(NodeInput {
        id: Some("C".to_string()), labels: vec!["USER".to_string()], props: Some(json!({"text": "Node C content"})),
        embedding: None, lang: None, valid_from: None, caused_by: None, ttl: None,
    }).unwrap();

    storage.add_edge(EdgeInput {
        id: None, from: "A".to_string(), to: "B".to_string(), rel: "knows".to_string(),
        props: None, valid_from: None, supersede: None, impact: None, caused_by: None,
    }).unwrap();

    storage.add_edge(EdgeInput {
        id: None, from: "B".to_string(), to: "C".to_string(), rel: "knows".to_string(),
        props: None, valid_from: None, supersede: None, impact: None, caused_by: None,
    }).unwrap();

    // 2. Test TIER H0 (Self Only)
    let ctx_h0 = storage.retrieve_context("A", "H0", None, false).unwrap();
    assert_eq!(ctx_h0.nodes.len(), 1);
    assert_eq!(ctx_h0.nodes[0].id, "A");
    assert_eq!(ctx_h0.edges.len(), 0);

    // 3. Test TIER H1 (Neighbors)
    let ctx_h1 = storage.retrieve_context("A", "H1", None, false).unwrap();
    // Should include A and B (neighbor)
    assert!(ctx_h1.nodes.iter().any(|n| n.id == "A"));
    assert!(ctx_h1.nodes.iter().any(|n| n.id == "B"));
    assert!(!ctx_h1.nodes.iter().any(|n| n.id == "C"));

    // 4. Test TIER H2 (Feature)
    let ctx_h2 = storage.retrieve_context("A", "H2", None, false).unwrap();
    // Should include A, B, and C
    assert_eq!(ctx_h2.nodes.len(), 3);
}

#[test]
fn test_grl_budget_compression() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    }).unwrap();

    // Add some nodes and metadata to allow SuperNode generation
    storage.add_node(NodeInput {
        id: Some("A".to_string()), labels: vec!["USER".to_string()], props: Some(json!({"large": "x".repeat(100)})),
        embedding: Some(vec![0.1; 1536]), lang: None, valid_from: None, caused_by: None, ttl: None,
    }).unwrap();

    storage.detect_communities().unwrap();
    storage.generate_meta_graph().unwrap();

    // Test with low budget (triggering compression)
    let ctx_low = storage.retrieve_context("A", "H2", Some(10), false).unwrap();
    assert!(ctx_low.nodes.is_empty(), "Nodes should be pruned due to budget");
    assert!(!ctx_low.super_nodes.is_empty(), "SuperNodes should be returned as fallback");
    println!("Budget fallback triggered correctly.");
}

#[test]
fn test_hql_context_command() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    }).unwrap();

    storage.add_node(NodeInput {
        id: Some("Target".to_string()), labels: vec!["USER".to_string()], props: None,
        embedding: None, lang: None, valid_from: None, caused_by: None, ttl: None,
    }).unwrap();

    // Test HQL CONTEXT
    let hql_res = storage.execute_hql("CONTEXT FOR Target TIER H1 BUDGET 5000").unwrap();
    let ctx: ContextPackage = serde_json::from_value(hql_res).unwrap();
    
    assert_eq!(ctx.nodes.len(), 1);
    assert_eq!(ctx.nodes[0].id, "Target");
    assert!(ctx.reasoning_path.contains("H1"));
    
    println!("HQL CONTEXT command verified.");
}
