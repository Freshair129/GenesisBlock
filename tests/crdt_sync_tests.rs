use genesis_block_native::{
    AuthorizedPeerConfig, Event, LogicalClock, NodeInput, OpenOptions, SignedEvent, Storage,
};
use serde_json::json;
use std::{fs, path::Path, sync::Arc};
use tempfile::tempdir;

fn mark_schema5(path: &Path) {
    fs::write(path.join("state.json"), r#"{"schema_version":5}"#).unwrap();
}

fn authorize(destination: &Storage, remote: &Storage, local_port: u16, remote_port: u16) {
    let identity = remote.local_identity();
    destination
        .configure_peer_membership(
            vec![AuthorizedPeerConfig {
                peer_id: identity.peer_id,
                public_key_hex: identity.public_key_hex,
                gossip_addr: format!("127.0.0.1:{remote_port}"),
                master_authority: false,
            }],
            &format!("127.0.0.1:{local_port}"),
            true,
        )
        .unwrap();
}

#[test]
fn test_crdt_conflict_resolution() {
    let dir_a = tempdir().unwrap();
    let dir_b = tempdir().unwrap();
    mark_schema5(dir_a.path());
    mark_schema5(dir_b.path());

    let storage_a = Arc::new(
        Storage::open(OpenOptions {
            path: dir_a.path().to_str().unwrap().to_string(),
            page_cache_mb: Some(64),
            read_only: Some(false),
            vector_dim: None,
            retention: None,
        })
        .unwrap(),
    );

    let storage_b = Arc::new(
        Storage::open(OpenOptions {
            path: dir_b.path().to_str().unwrap().to_string(),
            page_cache_mb: Some(64),
            read_only: Some(false),
            vector_dim: None,
            retention: None,
        })
        .unwrap(),
    );

    authorize(&storage_a, &storage_b, 30101, 30102);
    authorize(&storage_b, &storage_a, 30102, 30101);

    // 1. Agent A creates a node (automatically signs and persists)
    let node_a = storage_a
        .add_node(NodeInput {
            id: Some("node-1".to_string()),
            labels: vec!["USER".to_string()],
            props: Some(json!({"name": "Initial"})),
            embedding: None,
            lang: Some("en".to_string()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    let valid_event_a = storage_a.sign_event(&Event::Node(node_a.clone()));

    storage_b
        .reconcile_state(vec![valid_event_a.clone()])
        .unwrap();
    assert_eq!(
        storage_b.node_view("node-1").unwrap().props["name"],
        "Initial"
    );

    // 3. Concurrent Edits
    let node_a_v2 = storage_a
        .supersede_node(
            "node-1".to_string(),
            Some(json!({"name": "Version A"})),
            None,
        )
        .unwrap();

    let mut node_b_v2_inner = node_a.clone();
    node_b_v2_inner.props = json!({"name": "Version B"});
    node_b_v2_inner.clock = LogicalClock {
        time: 10,
        peer_id: storage_b.local_peer_id.clone(),
    };

    let signed_b = storage_b.sign_event(&Event::Node(node_b_v2_inner));

    // 4. B applies its own Version B locally first
    storage_b.reconcile_state(vec![signed_b.clone()]).unwrap();
    assert_eq!(
        storage_b.node_view("node-1").unwrap().props["name"],
        "Version B"
    );

    // 5. A receives B's update (10 > 2)
    storage_a.reconcile_state(vec![signed_b.clone()]).unwrap();
    assert_eq!(
        storage_a.node_view("node-1").unwrap().props["name"],
        "Version B"
    );

    // 6. B receives A's update (2 < 10)
    let signed_a2 = storage_a.sign_event(&Event::Node(node_a_v2));
    storage_b.reconcile_state(vec![signed_a2]).unwrap();
    assert_eq!(
        storage_b.node_view("node-1").unwrap().props["name"],
        "Version B",
        "B should reject A's older update"
    );
}

#[test]
fn test_logical_clock_convergence() {
    let dir = tempdir().unwrap();
    mark_schema5(dir.path());
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();

    let n1 = storage
        .add_node(NodeInput {
            id: Some("n1".to_string()),
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
    assert_eq!(n1.clock.time, 1);

    // Reconcile a remote event with time 100
    let mut node_inner = n1.clone();
    node_inner.id = "remote".to_string();
    node_inner.clock = LogicalClock {
        time: 100,
        peer_id: "remote-agent".to_string(),
    };

    let event = Event::Node(node_inner);
    let signed = storage.sign_event(&event);

    storage.reconcile_state(vec![signed]).unwrap();
    assert_eq!(storage.get_logical_clock(), 100);
}

#[test]
fn test_reconcile_batch_remains_reentrant_under_lifecycle_barrier() {
    let dir = tempdir().unwrap();
    mark_schema5(dir.path());
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();

    let mut nested = storage
        .add_node(NodeInput {
            id: Some("nested-event".to_string()),
            labels: vec!["SYNC".to_string()],
            props: Some(json!({"source": "batch"})),
            embedding: None,
            lang: Some("en".to_string()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    storage.retract_node("nested-event").unwrap();

    // Slice-1 tombstones: a re-push carrying the ORIGINAL (pre-retraction)
    // clock now correctly loses LWW to the retraction and must NOT resurrect.
    // What this test pins is the Batch arm's reentrancy under the lifecycle
    // barrier, so re-create with a clock NEWER than the retraction's — a
    // legitimate re-create that clears the tombstone on apply.
    nested.clock = LogicalClock {
        time: nested.clock.time + 2, // retraction consumed +1
        peer_id: storage.local_peer_id.clone(),
    };
    storage
        .reconcile_state(vec![
            storage.sign_event(&Event::Batch(vec![Event::Node(nested.clone())]))
        ])
        .unwrap();

    assert!(storage.node_view("nested-event").is_some());

    // And the inverse: the SAME batch shape with the stale pre-retraction
    // clock stays dead after a fresh retraction (tombstone wins LWW).
    storage.retract_node("nested-event").unwrap();
    let stale = storage.sign_event(&Event::Batch(vec![Event::Node(nested)]));
    storage.reconcile_state(vec![stale]).unwrap();
    assert!(
        storage.node_view("nested-event").is_none(),
        "stale batched re-push must not resurrect a retracted node"
    );
}

#[test]
fn test_cryptographic_forgery_rejection() {
    let dir_a = tempdir().unwrap();
    let dir_b = tempdir().unwrap();
    mark_schema5(dir_a.path());
    mark_schema5(dir_b.path());

    let storage_a = Storage::open(OpenOptions {
        path: dir_a.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();

    let storage_b = Storage::open(OpenOptions {
        path: dir_b.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .unwrap();

    let node_a = storage_a
        .add_node(NodeInput {
            id: Some("A".to_string()),
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

    authorize(&storage_b, &storage_a, 30202, 30201);

    let forged_event = SignedEvent {
        event: Event::Node(node_a),
        signature: vec![0u8; 64],
        signer_peer_id: storage_a.local_peer_id.clone(),
    };

    assert!(storage_b.reconcile_state(vec![forged_event]).is_err());
    assert!(
        storage_b.get_u32("A").is_none(),
        "B should have rejected forged event"
    );
    println!("Mark X: Cryptographic forgery rejection verified.");
}
