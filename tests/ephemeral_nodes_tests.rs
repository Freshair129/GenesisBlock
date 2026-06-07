use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use std::sync::Arc;
use tempfile::tempdir;
use chrono::Utc;

#[test]
fn test_ephemeral_nodes_ttl() {
    let dir = tempdir().unwrap();
    let path = dir.path().to_str().unwrap().to_string();
    let storage = Arc::new(Storage::open(OpenOptions { 
        path,
        page_cache_mb: Some(64),
        read_only: Some(false),
     vector_dim: None, }).unwrap());

    // 1. Add a node with 2-second TTL
    let node_res = storage.add_node(NodeInput {
        id: Some("ephemeral-1".to_string()),
        labels: vec!["TEMP".to_string()],
        props: None,
        embedding: None,
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
        ttl: Some(2),
    }).unwrap();

    assert!(node_res.expires_at.is_some());
    println!("Node expires at: {:?}", node_res.expires_at);

    // 2. Add a durable node
    storage.add_node(NodeInput {
        id: Some("durable-1".to_string()),
        labels: vec!["USER".to_string()],
        props: None,
        embedding: None,
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
        ttl: None,
    }).unwrap();

    // 3. Link them
    storage.add_edge(EdgeInput {
        id: None, from: "durable-1".to_string(), to: "ephemeral-1".to_string(), rel: "REFERENCES".to_string(),
        props: None, valid_from: None, supersede: None, impact: None, caused_by: None,
    }).unwrap();

    // Verify both exist
    assert!(storage.get_u32("ephemeral-1").is_some());
    assert!(storage.get_u32("durable-1").is_some());
    assert_eq!(storage.edges.len(), 1);

    // 4. Wait for expiration (3 seconds to be safe)
    println!("Waiting for TTL expiration...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // 5. Trigger autonomic maintenance
    storage.perform_autonomic_optimization().unwrap();

    // 6. Verify ephemeral node and its edge are gone
    assert!(storage.get_u32("ephemeral-1").is_none(), "Ephemeral node should be pruned");
    assert!(storage.get_u32("durable-1").is_some(), "Durable node should still exist");
    assert_eq!(storage.edges.len(), 0, "Edge linked to ephemeral node should be removed");
    
    println!("Step 4: Ephemeral Nodes verification successful.");
}
