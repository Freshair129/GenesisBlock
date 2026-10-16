use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use genesis_block_native::{
    AuthenticatedGossipPacket, AuthorizedPeerConfig, Event, GossipMessage, NodeInput, OpenOptions,
    SignedEvent, Storage, SyncProgressReceipt, GOSSIP_PROTOCOL_VERSION, SCHEMA_VERSION,
};
use serde_json::json;
use sha2::Digest;
use std::fmt::Display;
use std::fs;
use std::net::SocketAddr;
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

fn open_with_identity(dir: &TempDir, key: &SigningKey) -> Storage {
    fs::write(dir.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    fs::write(dir.path().join("identity.bin"), key.to_bytes()).unwrap();
    open(dir)
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
    let key = deterministic_signing_key(seed).verifying_key();
    hex::encode(sha2::Sha256::digest(key.as_bytes()))[..16].to_string()
}

const GOSSIP_TEST_TIMEOUT: Duration = Duration::from_secs(3);

async fn available_addr() -> SocketAddr {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    address
}

fn authorized_peer(
    storage: &Storage,
    address: SocketAddr,
    master_authority: bool,
) -> AuthorizedPeerConfig {
    let identity = storage.local_identity();
    AuthorizedPeerConfig {
        peer_id: identity.peer_id,
        public_key_hex: identity.public_key_hex,
        gossip_addr: address.to_string(),
        master_authority,
    }
}

struct RequesterHarness {
    requester: Arc<Storage>,
    _remote_dir: TempDir,
    remote: Storage,
    requester_addr: SocketAddr,
}

async fn start_requester(
    dir: &TempDir,
    peer_socket: &tokio::net::UdpSocket,
    peer_key: &SigningKey,
) -> RequesterHarness {
    let requester = Arc::new(open(dir));
    let remote_dir = TempDir::new().unwrap();
    let remote = open_with_identity(&remote_dir, peer_key);
    let requester_addr = available_addr().await;
    let remote_addr = peer_socket.local_addr().unwrap();
    requester
        .configure_peer_membership(
            vec![authorized_peer(&remote, remote_addr, false)],
            &requester_addr.to_string(),
            true,
        )
        .unwrap();
    remote
        .configure_peer_membership(
            vec![authorized_peer(&requester, requester_addr, true)],
            &remote_addr.to_string(),
            false,
        )
        .unwrap();

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
    RequesterHarness {
        requester,
        _remote_dir: remote_dir,
        remote,
        requester_addr,
    }
}

async fn send_heartbeat(
    peer_socket: &tokio::net::UdpSocket,
    harness: &RequesterHarness,
    merkle_root: &str,
) {
    let message = GossipMessage::Heartbeat {
        schema_version: GOSSIP_PROTOCOL_VERSION,
        storage_schema_version: SCHEMA_VERSION,
        peer_id: harness.remote.local_peer_id.clone(),
        merkle_root: merkle_root.to_string(),
        logical_time: 0,
        port: peer_socket.local_addr().unwrap().port(),
        verifying_key: harness.remote.verifying_key.to_bytes().to_vec(),
    };
    let data = harness.remote.encode_gossip_message(message).unwrap();
    peer_socket
        .send_to(&data, harness.requester_addr)
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
            if let Ok(AuthenticatedGossipPacket {
                message:
                    GossipMessage::PullRequest {
                        target_peer_id,
                        from_commit_seq,
                        request_nonce,
                        ..
                    },
                ..
            }) = serde_json::from_slice::<AuthenticatedGossipPacket>(&buf[..len])
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
    harness: &RequesterHarness,
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
        source_peer_id: harness.remote.local_peer_id.clone(),
        through_seq: Some(outer_through_seq),
        progress_receipt: Some(signed_receipt(
            receipt_signer,
            &harness.remote.local_peer_id,
            &harness.requester.local_peer_id,
            request_nonce,
            from_seq,
            signed_through_seq,
        )),
    };
    let data = harness.remote.encode_gossip_message(message).unwrap();
    peer_socket
        .send_to(&data, harness.requester_addr)
        .await
        .unwrap();
}

async fn observe_cursor_after_heartbeat(
    peer_socket: &tokio::net::UdpSocket,
    harness: &RequesterHarness,
) -> (String, u64) {
    tokio::time::sleep(Duration::from_millis(1100)).await;
    send_heartbeat(peer_socket, harness, "peer-root-that-differs").await;
    next_pull_request(peer_socket, &harness.requester.local_peer_id).await
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
    let source_addr = available_addr().await;
    source
        .configure_peer_membership(
            vec![authorized_peer(
                &destination,
                socket.local_addr().unwrap(),
                false,
            )],
            &source_addr.to_string(),
            true,
        )
        .unwrap();
    destination
        .configure_peer_membership(
            vec![authorized_peer(&source, source_addr, true)],
            &socket.local_addr().unwrap().to_string(),
            false,
        )
        .unwrap();
    source.publish_generation().unwrap();
    Storage::start_gossip_manager(source.clone());
    tokio::time::timeout(Duration::from_secs(3), async {
        while source.gossip_port.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    let request = destination
        .encode_gossip_message(GossipMessage::PullRequest {
            schema_version: GOSSIP_PROTOCOL_VERSION,
            storage_schema_version: SCHEMA_VERSION,
            from_clock: 0,
            target_peer_id: destination.local_peer_id.clone(),
            from_commit_seq: Some(0),
            request_nonce: Some("receipt-test-nonce".into()),
        })
        .unwrap();
    socket.send_to(&request, source_addr).await.unwrap();

    let mut buf = [0u8; 65536];
    let response = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let (len, _) = socket.recv_from(&mut buf).await.unwrap();
            let packet = serde_json::from_slice::<AuthenticatedGossipPacket>(&buf[..len]).unwrap();
            if matches!(packet.message, GossipMessage::PushDelta { .. }) {
                break packet;
            }
        }
    })
    .await
    .unwrap();
    assert!(destination.verify_gossip_packet(&response, source_addr));
    let response = response.message;
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
async fn requester_rejects_unframed_heartbeat_without_roster_authentication() {
    let peer_key = deterministic_signing_key(0x41);
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let requester_dir = TempDir::new().unwrap();
    let harness = start_requester(&requester_dir, &peer_socket, &peer_key).await;
    let message = GossipMessage::Heartbeat {
        schema_version: GOSSIP_PROTOCOL_VERSION,
        storage_schema_version: SCHEMA_VERSION,
        peer_id: peer_id_for(0x41),
        merkle_root: "attacker-root".into(),
        logical_time: 0,
        port: peer_socket.local_addr().unwrap().port(),
        verifying_key: peer_key.verifying_key().to_bytes().to_vec(),
    };
    peer_socket
        .send_to(
            &serde_json::to_vec(&message).unwrap(),
            harness.requester_addr,
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(harness.requester.peers.is_empty());
}

#[tokio::test]
async fn receipt_signature_must_match_the_roster_key() {
    let prepopulated_key = deterministic_signing_key(0x52);
    let attacker_key = deterministic_signing_key(0x63);
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let requester_dir = TempDir::new().unwrap();
    let harness = start_requester(&requester_dir, &peer_socket, &prepopulated_key).await;

    send_heartbeat(&peer_socket, &harness, "peer-root-that-differs").await;
    let (nonce, from_seq) = next_pull_request(&peer_socket, &harness.requester.local_peer_id).await;
    assert_eq!(from_seq, 0);

    // The outer packet is authenticated by the roster member, but the signed
    // cursor receipt is deliberately signed by a different key.
    send_receipted_push_delta(
        &peer_socket,
        &harness,
        &attacker_key,
        &nonce,
        ReceiptRange {
            from_seq,
            signed_through_seq: 23,
            outer_through_seq: 23,
        },
    )
    .await;

    let (retry_nonce, observed_cursor) =
        observe_cursor_after_heartbeat(&peer_socket, &harness).await;
    assert_eq!(
        retry_nonce, nonce,
        "retry must reuse the pending request nonce"
    );
    assert_eq!(
        observed_cursor, 0,
        "receipt signed by a non-roster key advanced the cursor"
    );
}

#[tokio::test]
async fn gossip_admits_only_roster_members_and_expires_live_peer_cache() {
    const GOSSIP_UNTRUSTED_FLOOD: usize = 300;
    let requester_dir = TempDir::new().unwrap();
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let peer_key = deterministic_signing_key(0x27);
    let harness = start_requester(&requester_dir, &peer_socket, &peer_key).await;
    let peer_id = harness.remote.local_peer_id.clone();
    send_heartbeat(&peer_socket, &harness, "remote-root").await;
    let _ = next_pull_request(&peer_socket, &harness.requester.local_peer_id).await;
    assert_eq!(harness.requester.peers.len(), 1);
    assert!(harness.requester.peers.contains_key(&peer_id));

    // A flood of legacy/unframed identities cannot create roster entries.
    for index in 0..GOSSIP_UNTRUSTED_FLOOD {
        let heartbeat = GossipMessage::Heartbeat {
            schema_version: GOSSIP_PROTOCOL_VERSION,
            storage_schema_version: SCHEMA_VERSION,
            peer_id: format!("untrusted-{index}"),
            merkle_root: "remote-root".into(),
            logical_time: 0,
            port: peer_socket.local_addr().unwrap().port(),
            verifying_key: peer_key.verifying_key().to_bytes().to_vec(),
        };
        peer_socket
            .send_to(
                &serde_json::to_vec(&heartbeat).unwrap(),
                harness.requester_addr,
            )
            .await
            .unwrap();
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(harness.requester.peers.len(), 1);
    assert!(!harness.requester.peers.contains_key("untrusted-0"));

    harness
        .requester
        .peers
        .get_mut(&peer_id)
        .expect("old peer should still be present")
        .last_seen = 0;
    send_heartbeat(&peer_socket, &harness, "remote-root").await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while !harness.requester.peers.contains_key(&peer_id) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("configured peer should return to the live endpoint cache");
    assert_eq!(harness.requester.peers.len(), 1);
}

#[tokio::test]
async fn requester_rejects_receipt_through_seq_mismatch_with_outer_cursor() {
    let prepopulated_key = deterministic_signing_key(0x74);
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let requester_dir = TempDir::new().unwrap();
    let harness = start_requester(&requester_dir, &peer_socket, &prepopulated_key).await;

    send_heartbeat(&peer_socket, &harness, "peer-root-that-differs").await;
    let (nonce, from_seq) = next_pull_request(&peer_socket, &harness.requester.local_peer_id).await;
    assert_eq!(from_seq, 0);
    send_receipted_push_delta(
        &peer_socket,
        &harness,
        &prepopulated_key,
        &nonce,
        ReceiptRange {
            from_seq,
            signed_through_seq: 7,
            outer_through_seq: 19,
        },
    )
    .await;

    let (retry_nonce, observed_cursor) =
        observe_cursor_after_heartbeat(&peer_socket, &harness).await;
    assert_eq!(
        retry_nonce, nonce,
        "retry must reuse the pending request nonce"
    );
    assert_eq!(
        observed_cursor, 0,
        "outer cursor advanced beyond the signed receipt"
    );
}

#[tokio::test]
async fn roster_member_with_matching_receipt_advances_the_sync_cursor() {
    let prepopulated_key = deterministic_signing_key(0x85);
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let requester_dir = TempDir::new().unwrap();
    let harness = start_requester(&requester_dir, &peer_socket, &prepopulated_key).await;

    send_heartbeat(&peer_socket, &harness, "peer-root-that-differs").await;
    let (nonce, from_seq) = next_pull_request(&peer_socket, &harness.requester.local_peer_id).await;
    assert_eq!(from_seq, 0);
    send_receipted_push_delta(
        &peer_socket,
        &harness,
        &prepopulated_key,
        &nonce,
        ReceiptRange {
            from_seq,
            signed_through_seq: 19,
            outer_through_seq: 19,
        },
    )
    .await;

    let (_retry_nonce, observed_cursor) =
        observe_cursor_after_heartbeat(&peer_socket, &harness).await;
    assert_eq!(
        observed_cursor, 19,
        "authenticated roster member receipt should advance the cursor"
    );
    assert_eq!(from_seq, 0);
    assert!(!nonce.is_empty());
}
