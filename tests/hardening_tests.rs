use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_bi_directional_edge_cleanup() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    }).unwrap();

    // 1. Add two nodes
    storage.add_node(NodeInput {
        id: Some("A".to_string()), labels: vec!["USER".to_string()], props: None,
        embedding: None, lang: None, valid_from: None, caused_by: None, ttl: None,
    }).unwrap();

    storage.add_node(NodeInput {
        id: Some("B".to_string()), labels: vec!["USER".to_string()], props: None,
        embedding: None, lang: None, valid_from: None, caused_by: None, ttl: None,
    }).unwrap();

    // 2. Link them A -> B
    storage.add_edge(EdgeInput {
        id: Some("E1".to_string()), from: "A".to_string(), to: "B".to_string(), rel: "KNOWS".to_string(),
        props: None, valid_from: None, supersede: None, impact: None, caused_by: None,
    }).unwrap();

    let u32_a = storage.get_u32("A").unwrap();
    let u32_b = storage.get_u32("B").unwrap();

    // Verify indices exist
    assert!(storage.out_idx.get(&u32_a).unwrap().contains(&storage.get_u32("E1").unwrap()));
    assert!(storage.in_idx.get(&u32_b).unwrap().contains(&storage.get_u32("E1").unwrap()));

    // 3. Retract node A
    storage.retract_node("A").unwrap();

    // 4. Verify Edge E1 is gone from main edges map
    assert!(storage.edges.get(&storage.get_u32("E1").unwrap_or(9999)).is_none());

    // 5. CRITICAL: Verify node B's in-index is cleaned up (Bi-directional cleanup)
    if let Some(in_set) = storage.in_idx.get(&u32_b) {
        assert!(!in_set.contains(&storage.get_u32("E1").unwrap_or(9999)), "Node B's in-index should not contain the deleted edge");
    }
    
    println!("Bi-directional edge cleanup verified.");
}

#[test]
fn test_configurable_vector_dim() {
    let dir = tempdir().unwrap();
    // Test with a non-standard dimension (e.g., 768 for small models)
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(768),
    }).unwrap();

    assert_eq!(storage.vector_dim, 768);

    // Add node with 768-dim vector
    let v_768 = vec![0.5; 768];
    storage.add_node(NodeInput {
        id: Some("node-768".to_string()),
        labels: vec!["USER".to_string()],
        props: None,
        embedding: Some(v_768.clone()),
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
        ttl: None,
    }).unwrap();

    // Verify metadata reflects the correct dimension
    let meta_arena = storage.metadata_arena.read();
    assert_eq!(meta_arena[0].vector_dim, 768);
    
    println!("Configurable vector dimension verified.");
}
