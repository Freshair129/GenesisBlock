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
    b.peers.insert(
        a.local_peer_id.clone(),
        SyncPeer {
            id: a.local_peer_id.clone(),
            addr: String::new(),
            last_seen: 0,
            verifying_key: a.verifying_key.to_bytes().to_vec(),
        },
    );
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
            .find(|e| matches!(e.event, Event::Batch(_)))
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
    use std::{sync::Arc, time::Duration};
    let dir = tempfile::tempdir().unwrap();
    let s = Arc::new(open(dir.path()));
    Storage::start_gossip_manager(s.clone());
    tokio::time::timeout(Duration::from_secs(3), async {
        while s.gossip_port.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = format!(
        "127.0.0.1:{}",
        s.gossip_port.load(std::sync::atomic::Ordering::SeqCst)
    );
    let request = GossipMessage::PullRequest {
        schema_version: 3,
        from_clock: 0,
        target_peer_id: "old".into(),
        from_commit_seq: Some(0),
    };
    socket
        .send_to(&serde_json::to_vec(&request).unwrap(), &addr)
        .await
        .unwrap();
    let mut buf = [0u8; 65536];
    let (len, _) = tokio::time::timeout(Duration::from_secs(3), socket.recv_from(&mut buf))
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        serde_json::from_slice::<GossipMessage>(&buf[..len]).unwrap(),
        GossipMessage::UpgradeRequired { schema_version: 4 }
    ));
    s.peers.insert(
        "new".into(),
        SyncPeer {
            id: "new".into(),
            addr: socket.local_addr().unwrap().to_string(),
            last_seen: 0,
            verifying_key: vec![],
        },
    );
    let request = GossipMessage::PullRequest {
        schema_version: 4,
        from_clock: 0,
        target_peer_id: "new".into(),
        from_commit_seq: Some(0),
    };
    socket
        .send_to(&serde_json::to_vec(&request).unwrap(), &addr)
        .await
        .unwrap();
    let (len, _) = tokio::time::timeout(Duration::from_secs(3), socket.recv_from(&mut buf))
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        serde_json::from_slice::<GossipMessage>(&buf[..len]).unwrap(),
        GossipMessage::PushDelta {
            through_seq: Some(0),
            ..
        }
    ));
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
    socket
        .send_to(&serde_json::to_vec(&request).unwrap(), &addr)
        .await
        .unwrap();
    let (len, _) = tokio::time::timeout(Duration::from_secs(3), socket.recv_from(&mut buf))
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(serde_json::from_slice::<GossipMessage>(&buf[..len]).unwrap(),
        GossipMessage::BootstrapRequired { reason } if reason.contains("SYNC_BOOTSTRAP_REQUIRED"))
    );
}
