use genesis_block_native::*;
use serde_json::json;

fn open(p: &std::path::Path) -> Storage {
    Storage::open(OpenOptions {
        path: p.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}
fn peer(a: &Storage, b: &Storage) {
    let identity = a.local_identity();
    b.configure_peer_membership(
        vec![AuthorizedPeerConfig {
            peer_id: identity.peer_id,
            public_key_hex: identity.public_key_hex,
            gossip_addr: "127.0.0.1:32202".into(),
            master_authority: false,
        }],
        "127.0.0.1:32201",
        true,
    )
    .unwrap();
}
#[test]
fn empty_collection_and_dependent_delta_sync_and_reopen() {
    let a_dir = tempfile::tempdir().unwrap();
    let b_dir = tempfile::tempdir().unwrap();
    let a = open(a_dir.path());
    let b = open(b_dir.path());
    peer(&a, &b);
    a.create_collection(
        "empty".into(),
        "m".into(),
        2,
        Some("cosine".into()),
        None,
        Some(321),
        None,
    )
    .unwrap();
    b.reconcile_state(a.events_since(0)).unwrap();
    assert!(b
        .list_collections()
        .iter()
        .any(|c| c.name == "empty" && c.ef_search == Some(321)));
    a.create_collection(
        "vectors".into(),
        "model-v".into(),
        2,
        Some("cosine".into()),
        Some("f16".into()),
        None,
        None,
    )
    .unwrap();
    let cursor = a.stable_frontier();
    a.add_node(NodeInput {
        id: Some("n".into()),
        labels: vec![],
        props: Some(json!({"v":1})),
        embedding: Some(vec![10., 0.]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: Some("vectors".into()),
    })
    .unwrap();
    b.reconcile_state(a.events_since_seq(cursor)).unwrap();
    assert_eq!(
        b.list_collections()
            .iter()
            .find(|c| c.name == "vectors")
            .unwrap()
            .metric,
        "Cosine"
    );
    drop(b);
    let b = open(b_dir.path());
    assert!(b.node_view("n").is_some());
}
#[test]
fn conflicting_definition_rejected_before_append() {
    let a_dir = tempfile::tempdir().unwrap();
    let b_dir = tempfile::tempdir().unwrap();
    let a = open(a_dir.path());
    let b = open(b_dir.path());
    peer(&a, &b);
    a.create_collection("x".into(), "a".into(), 2, None, None, None, None)
        .unwrap();
    b.create_collection("x".into(), "b".into(), 2, None, None, None, None)
        .unwrap();
    let before = b.stable_frontier();
    assert!(b.reconcile_state(a.events_since(0)).is_err());
    assert_eq!(b.stable_frontier(), before);
    assert_eq!(
        b.list_collections()
            .iter()
            .find(|c| c.name == "x")
            .unwrap()
            .model,
        "b"
    );
}

#[test]
fn signed_batch_rebind_is_applied_as_one_original_frame() {
    let ad = tempfile::tempdir().unwrap();
    let bd = tempfile::tempdir().unwrap();
    let a = open(ad.path());
    let b = open(bd.path());
    peer(&a, &b);
    let node = |id: &str| NodeInput {
        id: Some(id.into()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    };
    let edge = |from: &str, to: &str| EdgeInput {
        id: Some("e".into()),
        from: from.into(),
        to: to.into(),
        rel: "R".into(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    };
    a.execute_batch(BatchInput {
        nodes: ["A", "B", "C", "D"].iter().map(|id| node(id)).collect(),
        edges: vec![edge("A", "B")],
    })
    .unwrap();
    b.reconcile_state(a.events_since_seq(0)).unwrap();
    assert!(b.node_view("A").is_some());
    let old = b.stable_frontier();
    let cursor = a.stable_frontier();
    a.execute_batch(BatchInput {
        nodes: vec![],
        edges: vec![edge("C", "D")],
    })
    .unwrap();
    let delta = a.events_since_seq(cursor);
    let original = serde_json::to_value(
        delta
            .iter()
            .find(|e| matches!(e.event, Event::Transaction(_)))
            .unwrap(),
    )
    .unwrap();
    b.reconcile_state(delta).unwrap();
    assert_eq!(b.edges.get(&Storage::edge_key("e")).unwrap().from, "C");
    assert!(b
        .events_since_seq(old)
        .iter()
        .any(|e| serde_json::to_value(e).unwrap() == original));
}

#[tokio::test]
async fn gossip_rejects_old_schema_before_sending_delta() {
    use std::{net::SocketAddr, sync::Arc, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    let s = Arc::new(open(dir.path()));
    let initial_frontier = s.stable_frontier();
    let peer_dir = tempfile::tempdir().unwrap();
    let peer_storage = open(peer_dir.path());
    let peer_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let peer_addr = peer_socket.local_addr().unwrap();
    let reserve = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let local_addr: SocketAddr = reserve.local_addr().unwrap();
    drop(reserve);
    let peer_identity = peer_storage.local_identity();
    let local_identity = s.local_identity();
    s.configure_peer_membership(
        vec![AuthorizedPeerConfig {
            peer_id: peer_identity.peer_id.clone(),
            public_key_hex: peer_identity.public_key_hex.clone(),
            gossip_addr: peer_addr.to_string(),
            master_authority: false,
        }],
        &local_addr.to_string(),
        true,
    )
    .unwrap();
    peer_storage
        .configure_peer_membership(
            vec![AuthorizedPeerConfig {
                peer_id: local_identity.peer_id,
                public_key_hex: local_identity.public_key_hex,
                gossip_addr: local_addr.to_string(),
                master_authority: true,
            }],
            &peer_addr.to_string(),
            false,
        )
        .unwrap();
    Storage::start_gossip_manager(s.clone());
    tokio::time::timeout(Duration::from_secs(3), async {
        while s.gossip_port.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let mut buf = [0u8; 65536];
    let heartbeat = peer_storage
        .encode_gossip_message(GossipMessage::Heartbeat {
            schema_version: GOSSIP_PROTOCOL_VERSION,
            storage_schema_version: 5,
            peer_id: peer_storage.local_peer_id.clone(),
            merkle_root: s.get_merkle_root(),
            logical_time: 0,
            port: peer_addr.port(),
            verifying_key: peer_storage.verifying_key.to_bytes().to_vec(),
        })
        .unwrap();
    peer_socket.send_to(&heartbeat, local_addr).await.unwrap();
    let heartbeat_mismatch = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let (len, _) = peer_socket.recv_from(&mut buf).await.unwrap();
            let packet = serde_json::from_slice::<AuthenticatedGossipPacket>(&buf[..len]).unwrap();
            if matches!(packet.message, GossipMessage::StorageSchemaMismatch { .. }) {
                break packet.message;
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        heartbeat_mismatch,
        GossipMessage::StorageSchemaMismatch {
            local_storage_schema_version: SCHEMA_VERSION,
            peer_storage_schema_version: 5
        }
    ));
    assert!(
        s.peers.is_empty(),
        "schema-incompatible peers are not admitted"
    );

    let request = peer_storage
        .encode_gossip_message(GossipMessage::PullRequest {
            // Peers on the previous wire protocol must negotiate v8 before synchronization.
            schema_version: GOSSIP_PROTOCOL_VERSION - 1,
            storage_schema_version: SCHEMA_VERSION,
            from_clock: 0,
            target_peer_id: peer_storage.local_peer_id.clone(),
            from_commit_seq: Some(0),
            request_nonce: None,
        })
        .unwrap();
    peer_socket.send_to(&request, local_addr).await.unwrap();
    let old_schema_response = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let (len, _) = peer_socket.recv_from(&mut buf).await.unwrap();
            let packet = serde_json::from_slice::<AuthenticatedGossipPacket>(&buf[..len]).unwrap();
            if matches!(packet.message, GossipMessage::UpgradeRequired { .. }) {
                break packet.message;
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        old_schema_response,
        GossipMessage::UpgradeRequired {
            schema_version: GOSSIP_PROTOCOL_VERSION
        }
    ));
    let request = peer_storage
        .encode_gossip_message(GossipMessage::PullRequest {
            schema_version: GOSSIP_PROTOCOL_VERSION,
            storage_schema_version: 5,
            from_clock: 0,
            target_peer_id: peer_storage.local_peer_id.clone(),
            from_commit_seq: Some(0),
            request_nonce: None,
        })
        .unwrap();
    peer_socket.send_to(&request, local_addr).await.unwrap();
    let storage_schema_response = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let (len, _) = peer_socket.recv_from(&mut buf).await.unwrap();
            let packet = serde_json::from_slice::<AuthenticatedGossipPacket>(&buf[..len]).unwrap();
            if matches!(packet.message, GossipMessage::StorageSchemaMismatch { .. }) {
                break packet.message;
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        storage_schema_response,
        GossipMessage::StorageSchemaMismatch {
            local_storage_schema_version: SCHEMA_VERSION,
            peer_storage_schema_version: 5
        }
    ));
    let request = peer_storage
        .encode_gossip_message(GossipMessage::PullRequest {
            schema_version: GOSSIP_PROTOCOL_VERSION,
            storage_schema_version: SCHEMA_VERSION,
            from_clock: 0,
            target_peer_id: peer_storage.local_peer_id.clone(),
            from_commit_seq: Some(0),
            request_nonce: None,
        })
        .unwrap();
    peer_socket.send_to(&request, local_addr).await.unwrap();
    let delta_response = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let (len, _) = peer_socket.recv_from(&mut buf).await.unwrap();
            let packet = serde_json::from_slice::<AuthenticatedGossipPacket>(&buf[..len]).unwrap();
            if matches!(packet.message, GossipMessage::PushDelta { .. }) {
                break packet.message;
            }
        }
    })
    .await
    .unwrap();
    assert!(
        matches!(delta_response, GossipMessage::PushDelta { through_seq: Some(seq), .. } if seq == initial_frontier)
    );
    s.add_node(NodeInput {
        id: Some("oversized".into()),
        labels: vec![],
        props: Some(json!({"payload":"x".repeat(50_000)})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
    let request = peer_storage
        .encode_gossip_message(GossipMessage::PullRequest {
            schema_version: GOSSIP_PROTOCOL_VERSION,
            storage_schema_version: SCHEMA_VERSION,
            from_clock: 0,
            target_peer_id: peer_storage.local_peer_id.clone(),
            from_commit_seq: Some(0),
            request_nonce: None,
        })
        .unwrap();
    peer_socket.send_to(&request, local_addr).await.unwrap();
    let oversized_response = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let (len, _) = peer_socket.recv_from(&mut buf).await.unwrap();
            let packet = serde_json::from_slice::<AuthenticatedGossipPacket>(&buf[..len]).unwrap();
            if matches!(packet.message, GossipMessage::BootstrapRequired { .. }) {
                break packet.message;
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(oversized_response,
        GossipMessage::BootstrapRequired { reason } if reason.contains("SYNC_BOOTSTRAP_REQUIRED")));
}
