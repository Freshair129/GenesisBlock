use ed25519_dalek::{Signature, Verifier};
use genesis_block_native::{
    Event, GossipMessage, NodeInput, OpenOptions, SignedEvent, Storage, SyncPeer, SCHEMA_VERSION,
};
use serde_json::json;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

fn open(dir: &TempDir) -> Storage {
    Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn add_node(storage: &Storage, id: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: vec!["P6".into()],
            props: Some(json!({"id": id})),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn p6_event(storage: &Storage) -> SignedEvent {
    let generation = storage.publish_generation().unwrap();
    let event = serde_json::from_value(json!({
        "GenerationPublished": {
            "version": 1,
            "generation": generation,
        }
    }))
    .unwrap();
    SignedEvent {
        event,
        signature: vec![0],
        signer_peer_id: "foreign-peer".into(),
    }
}

fn is_p6(event: &Event) -> bool {
    serde_json::to_value(event)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .is_some_and(|object| {
            object.contains_key("GenerationPublished") || object.contains_key("AccessPolicyChanged")
        })
}

fn assert_error_prefix<T, E: Display>(result: Result<T, E>, prefix: &str) {
    let error = match result {
        Ok(_) => panic!("expected an error"),
        Err(error) => error,
    };
    assert!(
        error.to_string().starts_with(prefix),
        "expected {prefix}, got {}",
        error
    );
}

#[test]
fn peer_ingress_rejects_direct_p6_before_wal_side_effects() {
    let source = open(&TempDir::new().unwrap());
    let event = p6_event(&source);
    let destination_dir = TempDir::new().unwrap();
    let destination = open(&destination_dir);
    let before = destination.stable_frontier();

    assert_error_prefix(destination.reconcile_state(vec![event]), "P6_LOCAL_ONLY");
    assert_eq!(destination.stable_frontier(), before);
}

#[test]
fn peer_ingress_rejects_nested_p6_atomically_before_signature_or_siblings() {
    let source = open(&TempDir::new().unwrap());
    let event = p6_event(&source);
    let destination_dir = TempDir::new().unwrap();
    let destination = open(&destination_dir);
    let nested = SignedEvent {
        event: Event::Batch(vec![Event::Batch(vec![event.event])]),
        signature: vec![0],
        signer_peer_id: "forged-peer".into(),
    };

    assert_error_prefix(destination.reconcile_state(vec![nested]), "P6_LOCAL_ONLY");
    assert_eq!(destination.stable_frontier(), 0);
    assert!(destination.node_view("sibling").is_none());
}

#[test]
fn consensus_rejects_p6_control_events_before_proposal_creation() {
    let source = open(&TempDir::new().unwrap());
    let event = p6_event(&source);
    let destination = open(&TempDir::new().unwrap());

    assert_error_prefix(
        destination.propose_consensus(event.event, Vec::new()),
        "P6_LOCAL_ONLY",
    );
}

#[test]
fn outbound_sequence_sync_excludes_local_p6_control_events() {
    let source = open(&TempDir::new().unwrap());
    add_node(&source, "before");
    p6_event(&source);
    add_node(&source, "after");

    let events = source.events_since_seq(0);
    assert!(events.iter().all(|event| !is_p6(&event.event)));
    assert!(events
        .iter()
        .any(|event| { matches!(&event.event, Event::Node(node) if node.id == "before") }));
    assert!(events
        .iter()
        .any(|event| { matches!(&event.event, Event::Node(node) if node.id == "after") }));
}

#[tokio::test]
async fn filtered_sequence_progress_carries_a_signed_receipt() {
    let source_dir = TempDir::new().unwrap();
    let source = Arc::new(open(&source_dir));
    let destination_dir = TempDir::new().unwrap();
    let destination = open(&destination_dir);
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();

    source.peers.insert(
        destination.local_peer_id.clone(),
        SyncPeer {
            id: destination.local_peer_id.clone(),
            addr: socket.local_addr().unwrap().to_string(),
            last_seen: 0,
            verifying_key: destination.verifying_key.to_bytes().to_vec(),
        },
    );
    source.publish_generation().unwrap();
    Storage::start_gossip_manager(source.clone());
    tokio::time::timeout(Duration::from_secs(3), async {
        while source.gossip_port.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    let request = GossipMessage::PullRequest {
        schema_version: SCHEMA_VERSION,
        from_clock: 0,
        target_peer_id: destination.local_peer_id.clone(),
        from_commit_seq: Some(0),
        request_nonce: Some("receipt-test-nonce".into()),
    };
    socket
        .send_to(
            &serde_json::to_vec(&request).unwrap(),
            format!(
                "127.0.0.1:{}",
                source.gossip_port.load(std::sync::atomic::Ordering::SeqCst)
            ),
        )
        .await
        .unwrap();

    let mut buf = [0u8; 65536];
    let (len, _) = tokio::time::timeout(Duration::from_secs(3), socket.recv_from(&mut buf))
        .await
        .unwrap()
        .unwrap();
    let response = serde_json::from_slice::<GossipMessage>(&buf[..len]).unwrap();
    match response {
        GossipMessage::PushDelta {
            events,
            source_peer_id,
            through_seq: Some(through_seq),
            progress_receipt: Some(receipt),
        } => {
            assert!(events.iter().all(|event| !is_p6(&event.event)));
            assert_eq!(source_peer_id, source.local_peer_id);
            assert_eq!(receipt.responder_peer_id, source.local_peer_id);
            assert_eq!(receipt.requester_peer_id, destination.local_peer_id);
            assert_eq!(receipt.request_nonce, "receipt-test-nonce");
            assert_eq!(receipt.from_seq, 0);
            assert_eq!(receipt.through_seq, through_seq);
            assert_eq!(receipt.signature.len(), 64);
            let payload = serde_json::to_vec(&(
                receipt.version,
                receipt.responder_peer_id.as_str(),
                receipt.requester_peer_id.as_str(),
                receipt.request_nonce.as_str(),
                receipt.from_seq,
                receipt.through_seq,
            ))
            .unwrap();
            let signature = Signature::from_slice(&receipt.signature).unwrap();
            source.verifying_key.verify(&payload, &signature).unwrap();
        }
        other => panic!("expected signed PushDelta receipt, got {other:?}"),
    }
}
