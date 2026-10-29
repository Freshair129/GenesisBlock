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

fn h1_node(id: &str, padding: usize) -> Event {
    serde_json::from_value(json!({"Node": {
        "id":id, "labels":["P6"], "props":{"id":id,"padding":"x".repeat(padding)},
        "impact":null, "lang":"en", "valid_from":"2026-01-01T00:00:00Z",
        "valid_to":null, "caused_by":null, "expires_at":null,
        "clock":{"time":1,"peer_id":"h1-fixture"}, "collection":null
    }}))
    .unwrap()
}

fn h1_policy() -> Event {
    serde_json::from_value(json!({"AccessPolicyChanged": {
        "version":1,"expected_revision":0,
        "policy":{"revision":1,"mode":"Disabled","grants":[]},
        "actor":{"principal":"h1-owner","namespace":"default"},"folded":false
    }}))
    .unwrap()
}

fn h1_edge(id: &str) -> Event {
    serde_json::from_value(json!({"Edge": {
        "id":id,"from":"before-control","to":"after-control","rel":"LINK",
        "props":{"source":"h1-nested"},"valid_from":"2026-01-01T00:00:00Z",
        "valid_to":null,"recorded_at":"2026-01-01T00:00:00Z","superseded_by":null,
        "impact":null,"caused_by":null,"clock":{"time":1,"peer_id":"h1-fixture"}
    }}))
    .unwrap()
}

fn h1_assert_graph(storage: &Storage, dir: &TempDir) {
    for id in ["before-control", "after-control"] {
        assert_eq!(
            storage.node_view(id).unwrap().props,
            json!({"id":id,"padding":""})
        );
    }
    let conn = rusqlite::Connection::open_with_flags(
        dir.path().join("projection.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    for id in ["h1-link", "h1-nested-link"] {
        let payload: String = conn
            .query_row(
                "SELECT payload FROM edge_versions WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(value["props"], json!({"source":"h1-nested"}));
    }
}

fn h1_files(root: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fn visit(
        root: &std::path::Path,
        dir: &std::path::Path,
        out: &mut std::collections::BTreeMap<String, Vec<u8>>,
    ) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, out);
                continue;
            }
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path
                .file_name()
                .is_some_and(|n| n == "genesis.lock" || n == "projection.sqlite-shm")
            {
                out.insert(
                    format!("{name}:size"),
                    fs::metadata(path).unwrap().len().to_le_bytes().to_vec(),
                );
            } else {
                out.insert(name, fs::read(path).unwrap());
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    visit(root, root, &mut out);
    out
}

fn h1_verify_event(source: &Storage, signed: &SignedEvent) {
    assert_eq!(signed.signer_peer_id, source.local_peer_id);
    source
        .verifying_key
        .verify(
            &serde_json::to_vec(&signed.event).unwrap(),
            &Signature::from_slice(&signed.signature).unwrap(),
        )
        .unwrap();
}

#[test]
fn peer_nested_p6_with_real_siblings_is_atomic_and_survives_reopen() {
    let source_dir = TempDir::new().unwrap();
    let source = open_with_identity(&source_dir, &deterministic_signing_key(0x81));
    let generation = source.publish_generation().unwrap();
    let generation_event: Event = serde_json::from_value(json!({
        "GenerationPublished":{"version":1,"generation":generation}
    }))
    .unwrap();
    for control in [generation_event, h1_policy()] {
        for depth in 0..=2 {
            let dir = TempDir::new().unwrap();
            let destination = open_with_identity(&dir, &deterministic_signing_key(0x82));
            destination
                .configure_peer_membership(
                    vec![authorized_peer(
                        &source,
                        "127.0.0.1:31001".parse().unwrap(),
                        true,
                    )],
                    "127.0.0.1:31002",
                    true,
                )
                .unwrap();
            add_node(&destination, "existing");
            let lease = destination
                .pin_generation(
                    genesis_block_native::AccessContext {
                        principal: "h1-owner".into(),
                        namespace: "default".into(),
                    },
                    genesis_block_native::TemporalRead {
                        as_of: None,
                        tx_as_of: None,
                    },
                    Duration::from_secs(30),
                )
                .unwrap();
            destination.save_state().unwrap();
            let frontier = destination.stable_frontier();
            let before = h1_files(dir.path());
            let mut nested = control.clone();
            for _ in 0..depth {
                nested = Event::Batch(vec![nested]);
            }
            let signed = source.sign_event(&Event::Batch(vec![
                h1_node("before-control", 0),
                nested,
                h1_node("after-control", 0),
            ]));
            h1_verify_event(&source, &signed);
            let original = serde_json::to_vec(&signed).unwrap();
            assert_error_prefix(
                destination.reconcile_state(vec![signed.clone()]),
                "P6_LOCAL_ONLY",
            );
            assert_eq!(serde_json::to_vec(&signed).unwrap(), original);
            assert_eq!(destination.stable_frontier(), frontier);
            destination.validate_lease(&lease).unwrap();
            assert!(destination.node_view("existing").is_some());
            for id in ["before-control", "after-control"] {
                assert!(destination.node_view(id).is_none());
            }
            assert_eq!(h1_files(dir.path()), before);
            drop(destination);
            let reopened = open(&dir);
            assert_eq!(reopened.stable_frontier(), frontier);
            assert!(reopened.node_view("existing").is_some());
            for id in ["before-control", "after-control"] {
                assert!(reopened.node_view(id).is_none());
            }
            assert_error_prefix(reopened.validate_lease(&lease), "LEASE_OWNER_MISMATCH");
            eprintln!("H1 local-only control rejected and reopened: depth={depth}");
        }
    }
    // The same correctly signed siblings are admissible without a local-only child.
    for depth in [0, 1, 3] {
        let dir = TempDir::new().unwrap();
        let destination = open_with_identity(&dir, &deterministic_signing_key(0x83));
        destination
            .configure_peer_membership(
                vec![authorized_peer(
                    &source,
                    "127.0.0.1:31003".parse().unwrap(),
                    true,
                )],
                "127.0.0.1:31004",
                true,
            )
            .unwrap();
        let after = vec![h1_node("after-control", 0), h1_edge("h1-nested-link")];
        let mut children = vec![h1_node("before-control", 0)];
        if depth == 0 {
            children.extend(after);
        } else {
            let mut after = Event::Batch(after);
            for _ in 1..depth {
                after = Event::Batch(vec![after]);
            }
            children.push(after);
        }
        children.push(h1_edge("h1-link"));
        let ordinary = source.sign_event(&Event::Batch(children));
        h1_verify_event(&source, &ordinary);
        let original = serde_json::to_vec(&ordinary).unwrap();
        let before = destination.stable_frontier();
        destination.reconcile_state(vec![ordinary]).unwrap();
        assert_eq!(destination.stable_frontier(), before + 1);
        assert_eq!(
            destination
                .events_since_seq(before)
                .iter()
                .filter(|signed| serde_json::to_vec(signed).unwrap() == original)
                .count(),
            1
        );
        h1_assert_graph(&destination, &dir);
        drop(destination);
        if depth == 3 {
            // Force journal projection replay independently of a reusable cache.
            for name in [
                "projection.sqlite",
                "projection.sqlite-wal",
                "projection.sqlite-shm",
            ] {
                let path = dir.path().join(name);
                if path.exists() {
                    fs::remove_file(path).unwrap();
                }
            }
        }
        let reopened = Storage::open(OpenOptions {
            path: dir.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(2),
            retention: Some("full".into()),
        })
        .unwrap_or_else(|error| {
            panic!(
                "H1 ordinary depth={depth} reopen failed: {error}; fixture at {}",
                dir.path().display()
            );
        });
        h1_assert_graph(&reopened, &dir);
        eprintln!("H1 ordinary depth={depth} accepted and reopened");
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

// Boundary fixtures deliberately install signed, CRC-valid journal images while
// the acknowledged writer is idle. They exercise the real export/wire reader,
// not recovery or ordinary public mutation admission for these images.
fn h1_install_frames(dir: &TempDir, source: &Storage, frames: &[(u64, SignedEvent)]) {
    let last = frames.iter().map(|(seq, _)| *seq).max().unwrap();
    assert!(last > 0);
    while source.stable_frontier() < last {
        source.persist(&Event::Batch(vec![])).unwrap();
    }
    assert_eq!(source.stable_frontier(), last);
    // The image supplies the complete framed input. Remove the initial
    // genesis-base definition; the export path still prepends its bootstrap
    // dependency, which counts toward the delta budget.
    for entry in fs::read_dir(dir.path().join("journal")).unwrap() {
        let path = entry.unwrap().path();
        assert!(path.file_name().unwrap().to_string_lossy().starts_with('B'));
        fs::remove_file(path).unwrap();
    }
    let path = dir.path().join("wal/active.gwal");
    let mut bytes = fs::read(&path).unwrap()[..14].to_vec();
    let mut previous = 0;
    for (seq, signed) in frames {
        assert!(*seq >= previous);
        previous = *seq;
        h1_verify_event(source, signed);
        let payload = serde_json::to_vec(signed).unwrap();
        let mut crc_input = seq.to_le_bytes().to_vec();
        crc_input.extend_from_slice(&payload);
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&seq.to_le_bytes());
        bytes.extend_from_slice(&crc32c::crc32c(&crc_input).to_le_bytes());
        bytes.extend_from_slice(&payload);
    }
    fs::write(path, bytes).unwrap();
}

struct H1Export {
    _dir: TempDir,
    requester: Storage,
    socket: tokio::net::UdpSocket,
    source_addr: SocketAddr,
}

// Diagnostic control, not an in-budget protocol acceptance test: the original
// required prefix-budget response must still arrive in the test below.
#[tokio::test]
async fn gossip_datagram_default_capacity_probe() {
    let sender = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let receiver = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    eprintln!(
        "H1-T default probe: sndbuf={:?} rcvbuf={:?}",
        socket2::SockRef::from(&sender).send_buffer_size(),
        socket2::SockRef::from(&receiver).recv_buffer_size(),
    );
    for bytes in [128, 30_000, 55_000, 65_507] {
        let data = vec![0x5a; bytes];
        match sender.send_to(&data, receiver.local_addr().unwrap()).await {
            Ok(sent) => {
                eprintln!("H1-T default probe: bytes={bytes} sent={sent}");
                assert_eq!(sent, bytes);
                let mut buf = vec![0; 65_535];
                let (received, addr) =
                    tokio::time::timeout(GOSSIP_TEST_TIMEOUT, receiver.recv_from(&mut buf))
                        .await
                        .expect("successful loopback send must arrive within the unchanged guard")
                        .unwrap();
                assert_eq!(addr, sender.local_addr().unwrap());
                assert_eq!(&buf[..received], data.as_slice());
            }
            Err(error) => {
                eprintln!(
                    "H1-T default probe: bytes={bytes} send_error_kind={:?} raw_os_error={:?}",
                    error.kind(),
                    error.raw_os_error(),
                );
                assert_ne!(bytes, 128, "small control send must succeed");
            }
        }
    }
}

async fn h1_export(source: &Arc<Storage>) -> H1Export {
    let dir = TempDir::new().unwrap();
    let requester = open_with_identity(&dir, &deterministic_signing_key(0x91));
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let source_addr = available_addr().await;
    source
        .configure_peer_membership(
            vec![authorized_peer(
                &requester,
                socket.local_addr().unwrap(),
                false,
            )],
            &source_addr.to_string(),
            true,
        )
        .unwrap();
    requester
        .configure_peer_membership(
            vec![authorized_peer(source, source_addr, true)],
            &socket.local_addr().unwrap().to_string(),
            false,
        )
        .unwrap();
    Storage::start_gossip_manager(source.clone());
    tokio::time::timeout(GOSSIP_TEST_TIMEOUT, async {
        while source.gossip_port.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    H1Export {
        _dir: dir,
        requester,
        socket,
        source_addr,
    }
}

async fn h1_pull(harness: &H1Export, from: u64, nonce: &str) -> Option<GossipMessage> {
    eprintln!("H1 pull: from={from} nonce={nonce:.64}");
    let request = harness
        .requester
        .encode_gossip_message(GossipMessage::PullRequest {
            schema_version: GOSSIP_PROTOCOL_VERSION,
            storage_schema_version: 5,
            from_clock: 0,
            target_peer_id: harness.requester.local_peer_id.clone(),
            from_commit_seq: Some(from),
            request_nonce: Some(nonce.into()),
        })
        .unwrap();
    harness
        .socket
        .send_to(&request, harness.source_addr)
        .await
        .unwrap();
    tokio::time::timeout(GOSSIP_TEST_TIMEOUT, async {
        let mut buf = [0u8; 65536];
        loop {
            let (len, addr) = harness.socket.recv_from(&mut buf).await.unwrap();
            let packet: AuthenticatedGossipPacket = serde_json::from_slice(&buf[..len]).unwrap();
            if matches!(
                packet.message,
                GossipMessage::PushDelta { .. } | GossipMessage::BootstrapRequired { .. }
            ) {
                assert_eq!(addr, harness.source_addr);
                assert!(harness.requester.verify_gossip_packet(&packet, addr));
                return packet.message;
            }
        }
    })
    .await
    .ok()
}

fn h1_delta(
    source: &Storage,
    harness: &H1Export,
    message: GossipMessage,
    from: u64,
    through: u64,
    nonce: &str,
    expected: &[SignedEvent],
) {
    let GossipMessage::PushDelta {
        events,
        source_peer_id,
        through_seq,
        progress_receipt: Some(receipt),
    } = message
    else {
        panic!("expected a receipted delta, got {message:?}")
    };
    if expected.is_empty() {
        assert!(events.is_empty());
    } else {
        let Event::CollectionDefinition(definition) = &events[0].event else {
            panic!("bootstrap definition must precede data")
        };
        assert_eq!(definition.name, "default");
        assert_eq!(definition.provenance, "bootstrap");
        assert_eq!(
            events
                .iter()
                .filter(|signed| matches!(signed.event, Event::CollectionDefinition(_)))
                .count(),
            1
        );
    }
    assert_eq!(source_peer_id, source.local_peer_id);
    assert_eq!(through_seq, Some(through));
    assert_eq!(receipt.version, 1);
    assert_eq!(receipt.responder_peer_id, source.local_peer_id);
    assert_eq!(receipt.requester_peer_id, harness.requester.local_peer_id);
    assert_eq!(receipt.request_nonce, nonce);
    assert_eq!(receipt.from_seq, from);
    assert_eq!(receipt.through_seq, through);
    assert_eq!(through_seq, Some(receipt.through_seq));
    let payload = serde_json::to_vec(&(
        receipt.version,
        receipt.responder_peer_id.as_str(),
        receipt.requester_peer_id.as_str(),
        receipt.request_nonce.as_str(),
        receipt.from_seq,
        receipt.through_seq,
    ))
    .unwrap();
    source
        .verifying_key
        .verify(
            &payload,
            &Signature::from_slice(&receipt.signature).unwrap(),
        )
        .unwrap();
    assert!(events.iter().all(|signed| !is_p6(&signed.event)));
    let ordinary: Vec<_> = events
        .iter()
        .filter(|signed| !matches!(&signed.event, Event::CollectionDefinition(_)))
        .cloned()
        .collect();
    assert_eq!(
        serde_json::to_value(&ordinary).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    for signed in &events {
        h1_verify_event(source, signed);
    }
}

fn h1_sized(source: &Storage, id: &str, bytes: usize) -> SignedEvent {
    let make = |padding, salt| {
        let Event::Node(mut node) = h1_node(id, padding) else {
            unreachable!()
        };
        node.props["fixture_salt"] = json!(format!("{salt:04x}"));
        Event::Node(node)
    };
    let base = serde_json::to_vec(&source.sign_event(&make(0, 0)))
        .unwrap()
        .len();
    let center = bytes.checked_sub(base).unwrap();
    // JSON signature-array width changes with the signed payload. A second
    // fixed-width input salt avoids relying on a one-dimensional size search.
    for salt in 0..64 {
        for padding in center.saturating_sub(64)..center + 64 {
            let signed = source.sign_event(&make(padding, salt));
            if serde_json::to_vec(&signed).unwrap().len() == bytes {
                return signed;
            }
        }
    }
    panic!("could not construct exact {bytes}-byte signed event");
}

#[tokio::test]
async fn peer_mixed_batch_export_blocks_without_crossing_sequence() {
    for control in [h1_policy(), Event::Batch(vec![h1_policy()])] {
        let dir = TempDir::new().unwrap();
        let source = Arc::new(open_with_identity(&dir, &deterministic_signing_key(0x92)));
        let mixed = source.sign_event(&Event::Batch(vec![
            h1_node("before-control", 0),
            control,
            h1_node("after-control", 0),
        ]));
        let later = source.sign_event(&h1_node("later", 0));
        h1_install_frames(&dir, &source, &[(1, mixed.clone()), (2, later)]);
        let before = fs::read(dir.path().join("wal/active.gwal")).unwrap();
        assert!(source.events_since_seq(0).is_empty());
        assert!(source.events_since(0).is_empty());
        let harness = h1_export(&source).await;
        for nonce in ["mixed-first", "mixed-retry"] {
            let Some(GossipMessage::BootstrapRequired { reason }) =
                h1_pull(&harness, 0, nonce).await
            else {
                panic!("mixed batch escaped or gave no fail-closed error")
            };
            assert_eq!(reason, "P6_MIXED_BATCH_UNEXPORTABLE");
            assert_eq!(
                fs::read(dir.path().join("wal/active.gwal")).unwrap(),
                before
            );
            h1_verify_event(&source, &mixed);
        }
    }
}

#[tokio::test]
async fn peer_complete_prefix_budget_and_retry_boundaries() {
    for case in [
        "filtered-only",
        "trailing",
        "same-sequence",
        "prefix-budget",
        "event-at",
        "event-over",
        "delta-at",
        "delta-over",
        "blocked-group",
    ] {
        eprintln!("H1 export boundary: {case}");
        let dir = TempDir::new().unwrap();
        let source = Arc::new(open_with_identity(&dir, &deterministic_signing_key(0x93)));
        let control = source.sign_event(&Event::Batch(vec![h1_policy()]));
        let a = source.sign_event(&h1_node("a", 0));
        let b = source.sign_event(&h1_node("b", 0));
        let (frames, expected, through, error) = match case {
            "filtered-only" => (
                vec![(1, control.clone()), (2, control.clone())],
                vec![],
                2,
                None,
            ),
            "trailing" => (
                vec![(1, a.clone()), (2, control.clone())],
                vec![a.clone()],
                2,
                None,
            ),
            "same-sequence" => (
                vec![
                    (1, a.clone()),
                    (1, control.clone()),
                    (1, b.clone()),
                    (2, control.clone()),
                ],
                vec![a.clone(), b.clone()],
                2,
                None,
            ),
            "prefix-budget" => {
                let a = h1_sized(&source, "large-a", 30000);
                let b = h1_sized(&source, "large-b", 30000);
                (vec![(1, a.clone()), (2, b)], vec![a], 1, None)
            }
            "event-at" => {
                let a = h1_sized(&source, "at", 54000);
                (vec![(1, a.clone())], vec![a], 1, None)
            }
            "event-over" => (
                vec![(1, h1_sized(&source, "over", 54001))],
                vec![],
                0,
                Some("SYNC_BOOTSTRAP_REQUIRED: frame exceeds datagram budget"),
            ),
            "delta-at" | "delta-over" => {
                // Derive only input-definition byte overhead, then freeze a literal
                // 55,000/55,001-byte complete serialized vector before wire execution.
                source.persist(&Event::Batch(vec![])).unwrap();
                let definition = source
                    .events_since_seq(0)
                    .into_iter()
                    .find(|s| matches!(s.event, Event::CollectionDefinition(_)))
                    .unwrap();
                let overhead = serde_json::to_vec(&definition).unwrap().len() + 4;
                let a = h1_sized(&source, "delta-a", 27000);
                let target = if case == "delta-at" { 55000 } else { 55001 };
                let b = h1_sized(&source, "delta-b", target - overhead - 27000);
                assert_eq!(
                    serde_json::to_vec(&vec![definition, a.clone(), b.clone()])
                        .unwrap()
                        .len(),
                    target
                );
                // install_frames allocates remaining acknowledged fixture sequences.
                (
                    vec![(2, a.clone()), (2, b.clone())],
                    vec![a, b],
                    2,
                    if case == "delta-over" {
                        Some("SYNC_BOOTSTRAP_REQUIRED: base/dependencies exceed datagram budget")
                    } else {
                        None
                    },
                )
            }
            "blocked-group" => (
                vec![
                    (1, h1_sized(&source, "blocked-a", 30000)),
                    (1, h1_sized(&source, "blocked-b", 30000)),
                    (2, b),
                ],
                vec![],
                0,
                None,
            ),
            _ => unreachable!(),
        };
        h1_install_frames(&dir, &source, &frames);
        let before = fs::read(dir.path().join("wal/active.gwal")).unwrap();
        let harness = h1_export(&source).await;
        for nonce in ["boundary-first", "boundary-retry"] {
            let message = h1_pull(&harness, 0, nonce).await;
            if case == "blocked-group" {
                assert!(
                    message.is_none(),
                    "oversized indivisible group advanced or changed reply behavior"
                );
            } else if let Some(expected_error) = error {
                let Some(GossipMessage::BootstrapRequired { reason }) = message else {
                    panic!("{case}: expected bootstrap error")
                };
                assert_eq!(reason, expected_error, "{case}");
            } else {
                h1_delta(
                    &source,
                    &harness,
                    message.unwrap(),
                    0,
                    through,
                    nonce,
                    &expected,
                );
            }
            assert_eq!(
                fs::read(dir.path().join("wal/active.gwal")).unwrap(),
                before,
                "{case}"
            );
        }
        if case == "prefix-budget" {
            h1_delta(
                &source,
                &harness,
                h1_pull(&harness, 1, "prefix-next").await.unwrap(),
                1,
                2,
                "prefix-next",
                &[frames[1].1.clone()],
            );
        }
    }
}

#[tokio::test]
async fn peer_real_folded_base_zero_is_complete() {
    for padding in [0, 30000] {
        let dir = TempDir::new().unwrap();
        let signer_dir = TempDir::new().unwrap();
        let signer = open_with_identity(&signer_dir, &deterministic_signing_key(0x94));
        let mut legacy = Vec::new();
        let definition = signer
            .events_since_seq(0)
            .into_iter()
            .find(|signed| matches!(signed.event, Event::CollectionDefinition(_)))
            .unwrap();
        legacy.extend_from_slice(&serde_json::to_vec(&definition).unwrap());
        legacy.push(b'\n');
        for id in ["fold-a", "fold-b"] {
            legacy.extend_from_slice(
                &serde_json::to_vec(&signer.sign_event(&h1_node(id, padding))).unwrap(),
            );
            legacy.push(b'\n');
        }
        fs::write(dir.path().join("genesis-graph.wal"), legacy).unwrap();
        let source = Arc::new(open_with_identity(&dir, &deterministic_signing_key(0x94)));
        assert_eq!(source.stable_frontier(), 0);
        for id in ["fold-a", "fold-b"] {
            assert_eq!(
                source.node_view(id).unwrap().props,
                json!({"id":id,"padding":"x".repeat(padding)})
            );
        }
        source.compact().unwrap();
        assert!(fs::read_dir(dir.path().join("journal"))
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with('B')));
        let expected: Vec<_> = source
            .events_since_seq(0)
            .into_iter()
            .filter(|signed| !matches!(signed.event, Event::CollectionDefinition(_)))
            .collect();
        assert_eq!(expected.len(), 3);
        let mut ids: Vec<_> = expected
            .iter()
            .filter_map(|signed| match &signed.event {
                Event::Node(node) => Some(node.id.as_str()),
                _ => None,
            })
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["fold-a", "fold-b"]);
        assert_eq!(
            expected
                .iter()
                .filter(|signed| matches!(signed.event, Event::CollectionMaterialization(_)))
                .count(),
            1
        );
        for signed in &expected {
            h1_verify_event(&source, signed);
            if let Event::Node(node) = &signed.event {
                assert_eq!(
                    node.props,
                    json!({"id":node.id.as_str(),"padding":"x".repeat(padding)})
                );
            }
            assert!(serde_json::to_vec(signed).unwrap().len() <= 54000);
        }
        if padding > 0 {
            assert!(serde_json::to_vec(&expected).unwrap().len() > 55000);
        }
        let harness = h1_export(&source).await;
        for nonce in ["base-first", "base-retry"] {
            let message = h1_pull(&harness, 0, nonce).await.unwrap();
            if padding > 0 {
                let GossipMessage::BootstrapRequired { reason } = message else {
                    panic!("oversized base0 must fail without a cursor")
                };
                assert_eq!(
                    reason,
                    "SYNC_BOOTSTRAP_REQUIRED: base/dependencies exceed datagram budget"
                );
            } else {
                h1_delta(&source, &harness, message, 0, 0, nonce, &expected);
            }
        }
    }
}
