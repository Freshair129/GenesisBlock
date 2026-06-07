use genesis_block_native::{Storage, OpenOptions, NodeInput, Event, LogicalClock};
use std::sync::Arc;
use tempfile::tempdir;
use serde_json::json;

#[test]
fn test_crdt_conflict_resolution() {
    let dir_a = tempdir().unwrap();
    let dir_b = tempdir().unwrap();
    
    let storage_a = Arc::new(Storage::open(OpenOptions { 
        path: dir_a.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
     vector_dim: None, }).unwrap());

    let storage_b = Arc::new(Storage::open(OpenOptions { 
        path: dir_b.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
     vector_dim: None, }).unwrap());

    // 1. Agent A creates a node
    let node_a = storage_a.add_node(NodeInput {
        id: Some("node-1".to_string()),
        labels: vec!["USER".to_string()],
        props: Some(json!({"name": "Initial"})),
        embedding: None,
        lang: Some("en".to_string()),
        valid_from: None,
        caused_by: None,
        ttl: None,
    }).unwrap();

    // 2. Replicate to B
    storage_b.reconcile_state(vec![Event::Node(node_a.clone())]).unwrap();
    assert_eq!(storage_b.nodes.get(&0).unwrap().props["name"], "Initial");

    // 3. Concurrent Edits
    // A updates locally: Initial -> Version A (Clock 2)
    let node_a_v2 = storage_a.supersede_node("node-1".to_string(), Some(json!({"name": "Version A"})), None).unwrap();
    
    // B updates locally (simulated): Initial -> Version B (Clock 10)
    let mut node_b_v2 = node_a.clone();
    node_b_v2.props = json!({"name": "Version B"});
    node_b_v2.clock = LogicalClock { time: 10, peer_id: "agent-b".to_string() };

    // 4. A receives B's update (Clock 10 > Clock 2)
    storage_a.reconcile_state(vec![Event::Node(node_b_v2.clone())]).unwrap();
    assert_eq!(storage_a.nodes.get(&0).unwrap().props["name"], "Version B", "A should apply B's update (10 > 2)");

    // 5. B receives A's update (Clock 2 < Clock 1) 
    // Wait, B's LOCAL state in the real world would have advanced to 10 if it made that update.
    // Let's set B's local state to 10.
    storage_b.reconcile_state(vec![Event::Node(node_b_v2.clone())]).unwrap();
    
    // Now B receives A's version 2 (Clock 2)
    storage_b.reconcile_state(vec![Event::Node(node_a_v2.clone())]).unwrap();
    assert_eq!(storage_b.nodes.get(&0).unwrap().props["name"], "Version B", "B should reject A's older update (2 < 10)");
}

#[test]
fn test_logical_clock_convergence() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions { 
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
     vector_dim: None, }).unwrap();

    // Initial clock is 0 (or first mutation makes it 1)
    let n1 = storage.add_node(NodeInput {
        id: Some("n1".to_string()),
        labels: vec!["USER".to_string()],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
    }).unwrap();
    assert_eq!(n1.clock.time, 1);

    // Reconcile a remote event with time 100
    let mut node = storage.add_node(NodeInput {
        id: Some("remote".to_string()),
        labels: vec!["USER".to_string()],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
    }).unwrap();
    
    node.clock = LogicalClock { time: 100, peer_id: "remote-agent".to_string() };
    storage.reconcile_state(vec![Event::Node(node)]).unwrap();

    // Local logical_clock atomic should be 100
    assert_eq!(storage.get_logical_clock(), 100);

    // Next local mutation should be 101
    let next_node = storage.add_node(NodeInput {
        id: Some("local-next".to_string()),
        labels: vec!["USER".to_string()],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
    }).unwrap();

    assert_eq!(next_node.clock.time, 101);
}
