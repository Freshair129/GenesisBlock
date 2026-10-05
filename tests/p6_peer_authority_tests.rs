use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use genesis_block_native::{
    Event, GossipMessage, NodeInput, OpenOptions, SignedEvent, Storage, SyncPeer,
    SyncProgressReceipt, SCHEMA_VERSION,
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

fn deterministic_signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn peer_id_for(seed: u8) -> String {
    format!("test-peer-{seed:02x}")
}

const GOSSIP_TEST_TIMEOUT: Duration = Duration::from_secs(3);

async fn start_requester(
    dir: &TempDir,
    peer_socket: &tokio::net::UdpSocket,
    peer_id: &str,
    prepopulated_peer_key: Option<&SigningKey>,
) -> Arc<Storage> {
    let requester = Arc::new(open(dir));
    if let Some(key) = prepopulated_peer_key {
        requester.peers.insert(
            peer_id.to_string(),
            SyncPeer {
                id: peer_id.to_string(),
                addr: peer_socket.local_addr().unwrap().to_string(),
                last_seen: 0,
                verifying_key: key.verifying_key().to_bytes().to_vec(),
            },
        );
    }

    Storage::start_gossip_manager(requester.clone());
    tokio::time::timeout(GOSSIP_TEST_TIMEOUT, async {
        while requester
            .gossip_port
            .load(std::sync::atomic::Ordering::SeqCst)
            == 0
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("requester gossip manager did not bind");
    requester
}

async fn send_heartbeat(
    peer_socket: &tokio::net::UdpSocket,
    requester: &Storage,
    peer_id: &str,
    signing_key: &SigningKey,
    merkle_root: &str,
) {
    let message = GossipMessage::Heartbeat {
        schema_version: SCHEMA_VERSION,
        peer_id: peer_id.to_string(),
        merkle_root: merkle_root.to_string(),
        logical_time: 0,
        port: peer_socket.local_addr().unwrap().port(),
        verifying_key: signing_key.verifying_key().to_bytes().to_vec(),
    };
    let address = format!(
        "127.0.0.1:{}",
        requester
            .gossip_port
            .load(std::sync::atomic::Ordering::SeqCst)
    );
    peer_socket
        .send_to(&serde_json::to_vec(&message).unwrap(), address)
        .await
        .unwrap();
}

async fn next_pull_request(
    peer_socket: &tokio::net::UdpSocket,
    requester_peer_id: &str,
) -> (String, u64) {
    tokio::time::timeout(GOSSIP_TEST_TIMEOUT, async {
        let mut buf = [0u8; 65535];
        loop {
            let (len, _) = peer_socket.recv_from(&mut buf).await.unwrap();
            if let Ok(GossipMessage::PullRequest {
                target_peer_id,
                from_commit_seq,
                request_nonce,
                ..
            }) = serde_json::from_slice::<GossipMessage>(&buf[..len])
            {
                if target_peer_id == requester_peer_id {
                    return (
                        request_nonce.expect("requester PullRequest has a nonce"),
                        from_commit_seq.expect("requester PullRequest has a commit cursor"),
                    );
                }
            }
        }
    })
    .await
    .expect("timed out waiting for requester PullRequest")
}

fn signed_receipt(
    signing_key: &SigningKey,
    responder_peer_id: &str,
    requester_peer_id: &str,
    request_nonce: &str,
    from_seq: u64,
    through_seq: u64,
) -> SyncProgressReceipt {
    let version = 1;
    let payload = serde_json::to_vec(&(
        version,
        responder_peer_id,
        requester_peer_id,
        request_nonce,
        from_seq,
        through_seq,
    ))
    .unwrap();
    SyncProgressReceipt {
        version,
        responder_peer_id: responder_peer_id.to_string(),
        requester_peer_id: requester_peer_id.to_string(),
        request_nonce: request_nonce.to_string(),
        from_seq,
        through_seq,
        signature: signing_key.sign(&payload).to_bytes().to_vec(),
    }
}

struct ReceiptRange {
    from_seq: u64,
    signed_through_seq: u64,
    outer_through_seq: u64,
}

async fn send_receipted_push_delta(
    peer_socket: &tokio::net::UdpSocket,
    requester: &Storage,
    peer_id: &str,
    receipt_signer: &SigningKey,
    request_nonce: &str,
    range: ReceiptRange,
) {
    let ReceiptRange {
        from_seq,
        signed_through_seq,
        outer_through_seq,
    } = range;
    let message = GossipMessage::PushDelta {
        events: Vec::new(),
        source_peer_id: peer_id.to_string(),
        through_seq: Some(outer_through_seq),
        progress_receipt: Some(signed_receipt(
            receipt_signer,
            peer_id,
            &requester.local_peer_id,
            request_nonce,
            from_seq,
            signed_through_seq,
        )),
    };
    let address = format!(
        "127.0.0.1:{}",
        requester
            .gossip_port
            .load(std::sync::atomic::Ordering::SeqCst)
    );
    peer_socket
        .send_to(&serde_json::to_vec(&message).unwrap(), address)
        .await
        .unwrap();
}

async fn observe_cursor_after_heartbeat(
    peer_socket: &tokio::net::UdpSocket,
    requester: &Storage,
    peer_id: &str,
    heartbeat_key: &SigningKey,
) -> u64 {
    send_heartbeat(
        peer_socket,
        requester,
        peer_id,
        heartbeat_key,
        "peer-root-that-differs",
    )
    .await;
    next_pull_request(peer_socket, &requester.local_peer_id)
        .await
        .1
}

#[test]
fn peer_ingress_rejects_direct_p6_before_wal_side_effects() {
    let source_dir = TempDir::new().unwrap();
    let source = open(&source_dir);
    let event = p6_event(&source);
    let destination_dir = TempDir::new().unwrap();
    let destination = open(&destination_dir);
    let before = destination.stable_frontier();

    assert_error_prefix(destination.reconcile_state(vec![event]), "P6_LOCAL_ONLY");
    assert_eq!(destination.stable_frontier(), before);
}

#[test]
fn peer_ingress_rejects_nested_p6_atomically_before_signature_or_siblings() {
    let source_dir = TempDir::new().unwrap();
    let source = open(&source_dir);
    let event = p6_event(&source);
    let destination_dir = TempDir::new().unwrap();
    let destination = open(&destination_dir);
    let nested = SignedEvent {
        event: Event::Batch(vec![Event::Batch(vec![event.event])]),
        signature: vec![0],
        signer_peer_id: "forged-peer".into(),
    };

    let before = destination.stable_frontier();
    assert_error_prefix(destination.reconcile_state(vec![nested]), "P6_LOCAL_ONLY");
    assert_eq!(destination.stable_frontier(), before);
    assert!(destination.node_view("sibling").is_none());
}

#[test]
fn consensus_rejects_p6_control_events_before_proposal_creation() {
    let source_dir = TempDir::new().unwrap();
    let source = open(&source_dir);
    let event = p6_event(&source);
    let destination_dir = TempDir::new().unwrap();
    let destination = open(&destination_dir);

    assert_error_prefix(
        destination.propose_consensus(event.event, Vec::new()),
        "P6_LOCAL_ONLY",
    );
}

#[test]
fn outbound_sequence_sync_excludes_local_p6_control_events() {
    let source_dir = TempDir::new().unwrap();
    let source = open(&source_dir);
    add_node(&source, "before");
    p6_event(&source);
    add_node(&source, "after");

    let events = source.events_since_seq(0);
    assert!(events.iter().all(|event| !is_p6(&event.event)));
    for expected_id in ["before", "after"] {
        assert!(events.iter().any(|event| match &event.event {
            Event::Node(node) => node.id == expected_id,
            Event::Transaction(transaction) =>
                transaction.nodes.iter().any(|node| node.id == expected_id),
            _ => false,
        }));
    }
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

#[tokio::test]
async fn requester_rejects_progress_authorized_only_by_unauthenticated_heartbeat() {
    let peer_key = deterministic_signing_key(0x41);
    let peer_id = peer_id_for(0x41);
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let requester_dir = TempDir::new().unwrap();
    let requester = start_requester(&requester_dir, &peer_socket, &peer_id, None).await;

    send_heartbeat(
        &peer_socket,
        &requester,
        &peer_id,
        &peer_key,
        "peer-root-that-differs",
    )
    .await;
    let (nonce, from_seq) = next_pull_request(&peer_socket, &requester.local_peer_id).await;
    assert_eq!(from_seq, 0);

    send_receipted_push_delta(
        &peer_socket,
        &requester,
        &peer_id,
        &peer_key,
        &nonce,
        ReceiptRange {
            from_seq,
            signed_through_seq: 17,
            outer_through_seq: 17,
        },
    )
    .await;

    let observed_cursor =
        observe_cursor_after_heartbeat(&peer_socket, &requester, &peer_id, &peer_key).await;
    assert_eq!(observed_cursor, 0, "heartbeat-only key advanced the cursor");
}

#[tokio::test]
async fn requester_heartbeat_key_replacement_does_not_authorize_cursor_progress() {
    let prepopulated_key = deterministic_signing_key(0x52);
    let attacker_key = deterministic_signing_key(0x63);
    let peer_id = peer_id_for(0x52);
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let requester_dir = TempDir::new().unwrap();
    let requester = start_requester(
        &requester_dir,
        &peer_socket,
        &peer_id,
        Some(&prepopulated_key),
    )
    .await;

    send_heartbeat(
        &peer_socket,
        &requester,
        &peer_id,
        &prepopulated_key,
        "peer-root-that-differs",
    )
    .await;
    let (nonce, from_seq) = next_pull_request(&peer_socket, &requester.local_peer_id).await;
    assert_eq!(from_seq, 0);

    // The matching root avoids replacing the outstanding request nonce while
    // the heartbeat attempts to replace the pre-existing peer-map key.
    let matching_root = requester.get_merkle_root();
    send_heartbeat(
        &peer_socket,
        &requester,
        &peer_id,
        &attacker_key,
        &matching_root,
    )
    .await;
    send_receipted_push_delta(
        &peer_socket,
        &requester,
        &peer_id,
        &attacker_key,
        &nonce,
        ReceiptRange {
            from_seq,
            signed_through_seq: 23,
            outer_through_seq: 23,
        },
    )
    .await;

    let observed_cursor =
        observe_cursor_after_heartbeat(&peer_socket, &requester, &peer_id, &prepopulated_key).await;
    assert_eq!(
        observed_cursor, 0,
        "heartbeat key replacement authorized cursor progress"
    );
}

#[tokio::test]
async fn requester_rejects_receipt_through_seq_mismatch_with_outer_cursor() {
    let prepopulated_key = deterministic_signing_key(0x74);
    let peer_id = peer_id_for(0x74);
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let requester_dir = TempDir::new().unwrap();
    let requester = start_requester(
        &requester_dir,
        &peer_socket,
        &peer_id,
        Some(&prepopulated_key),
    )
    .await;

    send_heartbeat(
        &peer_socket,
        &requester,
        &peer_id,
        &prepopulated_key,
        "peer-root-that-differs",
    )
    .await;
    let (nonce, from_seq) = next_pull_request(&peer_socket, &requester.local_peer_id).await;
    assert_eq!(from_seq, 0);
    send_receipted_push_delta(
        &peer_socket,
        &requester,
        &peer_id,
        &prepopulated_key,
        &nonce,
        ReceiptRange {
            from_seq,
            signed_through_seq: 7,
            outer_through_seq: 19,
        },
    )
    .await;

    let observed_cursor =
        observe_cursor_after_heartbeat(&peer_socket, &requester, &peer_id, &prepopulated_key).await;
    assert_eq!(
        observed_cursor, 0,
        "outer cursor advanced beyond the signed receipt"
    );
}

#[tokio::test]
async fn requester_does_not_advance_for_preconfigured_peer_entry_even_when_receipt_matches() {
    let prepopulated_key = deterministic_signing_key(0x85);
    let peer_id = peer_id_for(0x85);
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let requester_dir = TempDir::new().unwrap();
    let requester = start_requester(
        &requester_dir,
        &peer_socket,
        &peer_id,
        Some(&prepopulated_key),
    )
    .await;

    send_heartbeat(
        &peer_socket,
        &requester,
        &peer_id,
        &prepopulated_key,
        "peer-root-that-differs",
    )
    .await;
    let (nonce, from_seq) = next_pull_request(&peer_socket, &requester.local_peer_id).await;
    assert_eq!(from_seq, 0);
    send_receipted_push_delta(
        &peer_socket,
        &requester,
        &peer_id,
        &prepopulated_key,
        &nonce,
        ReceiptRange {
            from_seq,
            signed_through_seq: 19,
            outer_through_seq: 19,
        },
    )
    .await;

    let observed_cursor =
        observe_cursor_after_heartbeat(&peer_socket, &requester, &peer_id, &prepopulated_key).await;
    assert_eq!(
        observed_cursor, 0,
        "preconfigured Storage.peers entry was treated as receipt trust"
    );
}
