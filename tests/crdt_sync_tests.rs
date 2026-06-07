use genesis_block_native::{Storage, OpenOptions, NodeInput, Event, LogicalClock, SignedEvent};
use ed25519_dalek::Signer;
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
        vector_dim: None, 
    }).unwrap());

    let storage_b = Arc::new(Storage::open(OpenOptions { 
        path: dir_b.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None, 
    }).unwrap());

    // 1. Agent A creates a node (automatically signs and persists)
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

    // 2. Mock a SignedEvent for replication
    let signed_event_a = SignedEvent {
        event: Event::Node(node_a.clone()),
        signature: vec![0; 64], // We'll bypass local verification in this test by matching local_peer_id if needed, 
                               // but better to use real signatures or register peer.
        signer_peer_id: storage_a.local_peer_id.clone(),
    };
    
    // Register Peer A at B
    storage_b.peers.insert(storage_a.local_peer_id.clone(), genesis_block_native::SyncPeer {
        id: storage_a.local_peer_id.clone(), addr: "".to_string(), last_seen: 0,
        verifying_key: storage_a.verifying_key.to_bytes().to_vec(),
    });

    // We need a real signature for B to accept it
    let event_data = serde_json::to_vec(&signed_event_a.event).unwrap();
    let real_signature = storage_a.signing_key.sign(&event_data).to_bytes().to_vec();
    let valid_event_a = SignedEvent {
        event: signed_event_a.event,
        signature: real_signature,
        signer_peer_id: storage_a.local_peer_id.clone(),
    };

    storage_b.reconcile_state(vec![valid_event_a.clone()]).unwrap();
    assert_eq!(storage_b.nodes.get(&0).unwrap().props["name"], "Initial");

    // 3. Concurrent Edits
    let node_a_v2 = storage_a.supersede_node("node-1".to_string(), Some(json!({"name": "Version A"})), None).unwrap();
    
    let mut node_b_v2_inner = node_a.clone();
    node_b_v2_inner.props = json!({"name": "Version B"});
    node_b_v2_inner.clock = LogicalClock { time: 10, peer_id: storage_b.local_peer_id.clone() };

    let event_b = Event::Node(node_b_v2_inner);
    let sig_b = storage_b.signing_key.sign(&serde_json::to_vec(&event_b).unwrap()).to_bytes().to_vec();
    let signed_b = SignedEvent {
        event: event_b,
        signature: sig_b,
        signer_peer_id: storage_b.local_peer_id.clone(),
    };

    // Register Peer B at A
    storage_a.peers.insert(storage_b.local_peer_id.clone(), genesis_block_native::SyncPeer {
        id: storage_b.local_peer_id.clone(), addr: "".to_string(), last_seen: 0,
        verifying_key: storage_b.verifying_key.to_bytes().to_vec(),
    });

    // 4. B applies its own Version B locally first
    storage_b.reconcile_state(vec![signed_b.clone()]).unwrap();
    assert_eq!(storage_b.nodes.get(&0).unwrap().props["name"], "Version B");

    // 5. A receives B's update (10 > 2)
    storage_a.reconcile_state(vec![signed_b.clone()]).unwrap();
    assert_eq!(storage_a.nodes.get(&0).unwrap().props["name"], "Version B");

    // 6. B receives A's update (2 < 10)
    let sig_a2 = storage_a.signing_key.sign(&serde_json::to_vec(&Event::Node(node_a_v2.clone())).unwrap()).to_bytes().to_vec();
    let signed_a2 = SignedEvent {
        event: Event::Node(node_a_v2),
        signature: sig_a2,
        signer_peer_id: storage_a.local_peer_id.clone(),
    };
    storage_b.reconcile_state(vec![signed_a2]).unwrap();
    assert_eq!(storage_b.nodes.get(&0).unwrap().props["name"], "Version B", "B should reject A's older update");
}

#[test]
fn test_logical_clock_convergence() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions { 
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None, 
    }).unwrap();

    let n1 = storage.add_node(NodeInput {
        id: Some("n1".to_string()), labels: vec![], props: None, embedding: None, lang: None,
        valid_from: None, caused_by: None, ttl: None,
    }).unwrap();
    assert_eq!(n1.clock.time, 1);

    // Reconcile a remote event with time 100
    let mut node_inner = n1.clone();
    node_inner.id = "remote".to_string();
    node_inner.clock = LogicalClock { time: 100, peer_id: "remote-agent".to_string() };
    
    let event = Event::Node(node_inner);
    // We'll bypass verification in this test by not registering the peer, 
    // but reconcile_state skips verification if signer_id == local_peer_id.
    // Let's just use local_peer_id for simplicity in this specific test.
    let signed = SignedEvent {
        event,
        signature: vec![0; 64],
        signer_peer_id: storage.local_peer_id.clone(),
    };

    storage.reconcile_state(vec![signed]).unwrap();
    assert_eq!(storage.get_logical_clock(), 100);
}

#[test]
fn test_cryptographic_forgery_rejection() {
    let dir_a = tempdir().unwrap();
    let dir_b = tempdir().unwrap();
    
    let storage_a = Storage::open(OpenOptions {
        path: dir_a.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
    }).unwrap();

    let storage_b = Storage::open(OpenOptions {
        path: dir_b.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
    }).unwrap();

    let node_a = storage_a.add_node(NodeInput {
        id: Some("A".to_string()), labels: vec![], props: None, embedding: None, lang: None,
        valid_from: None, caused_by: None, ttl: None,
    }).unwrap();

    storage_b.peers.insert(storage_a.local_peer_id.clone(), genesis_block_native::SyncPeer {
        id: storage_a.local_peer_id.clone(),
        addr: "127.0.0.1:0".to_string(),
        last_seen: 0,
        verifying_key: storage_a.verifying_key.to_bytes().to_vec(),
    });

    let forged_event = SignedEvent {
        event: Event::Node(node_a),
        signature: vec![0u8; 64], 
        signer_peer_id: storage_a.local_peer_id.clone(),
    };

    storage_b.reconcile_state(vec![forged_event]).unwrap();
    assert!(storage_b.get_u32("A").is_none(), "B should have rejected forged event");
    println!("Mark X: Cryptographic forgery rejection verified.");
}
