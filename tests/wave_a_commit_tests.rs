use genesis_block_native::*;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

fn open(path: &std::path::Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn node(id: &str) -> NodeInput {
    NodeInput {
        id: Some(id.into()),
        labels: vec!["TEST".into()],
        props: Some(json!({"v":1})),
        embedding: Some(vec![1.0, 0.0]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }
}

fn schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "audit".into(),
        schema_version: 1,
        previous_version: None,
        package_id: "00000000-0000-4000-8000-000000000002".into(),
        schema_hash: String::new(),
        named_queries: vec![],
        tables: vec![RelationalTable {
            name: "items".into(),
            columns: vec![
                RelationalColumn::required("id", RelationalColumnType::Text),
                RelationalColumn::required("name", RelationalColumnType::Text),
            ],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![RelationalIndex {
                name: "unique_name".into(),
                columns: vec!["name".into()],
                unique: true,
            }],
        }],
    }
}

fn txn(id: &str, rows: Vec<Value>) -> GenesisTransaction {
    GenesisTransaction {
        transaction_id: id.into(),
        expected_frontier: None,
        relational: vec![RelationalMutationGroup {
            namespace: "audit".into(),
            mutations: rows
                .into_iter()
                .map(|values| RelationalRowMutation {
                    table: "items".into(),
                    kind: RelationalMutationKind::Insert,
                    values,
                    key: None,
                })
                .collect(),
        }],
        graph: BatchInput {
            nodes: vec![node("mixed")],
            edges: vec![],
        },
        vectors: vec![],
    }
}

fn rows(s: &Storage) -> Vec<Value> {
    s.query_relational(RelationalQuery {
        namespace: "audit".into(),
        table: "items".into(),
        columns: vec!["items.id".into()],
        joins: vec![],
        filters: vec![],
        limit: None,
        offset: None,
    })
    .unwrap()
}

fn broken_wal<T>(s: &mut Storage, f: impl FnOnce(&Storage) -> T) -> T {
    let (tx, rx) = crossbeam_channel::unbounded();
    drop(rx);
    let original = std::mem::replace(&mut s.wal_sender, tx);
    let result = f(s);
    s.wal_sender = original;
    result
}

#[test]
fn rejected_constraints_leave_no_wal_and_database_reopens() {
    for bad in [
        vec![json!({"id":"a","name":"x"}), json!({"id":"a","name":"y"})],
        vec![json!({"id":"a","name":"x"}), json!({"id":"b","name":"x"})],
        vec![json!({"id":"a","name":null})],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let s = open(dir.path());
        s.register_relational_schema(schema()).unwrap();
        let before = s.stable_frontier();
        assert!(s.commit_transaction(txn("retry", bad)).is_err());
        assert_eq!(
            s.stable_frontier(),
            before,
            "rejection must not append a poison frame"
        );
        assert!(s.node_view("mixed").is_none());
        assert!(rows(&s).is_empty());
        drop(s);
        let s = open(dir.path());
        assert!(rows(&s).is_empty());
        let retry = txn("retry", vec![json!({"id":"a","name":"corrected"})]);
        let committed = s.commit_transaction(retry.clone()).unwrap();
        assert_eq!(
            s.commit_transaction(retry).unwrap().commit_sequence,
            committed.commit_sequence
        );
    }
}

#[test]
fn failed_node_upsert_preserves_old_graph_row_and_vector() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = open(dir.path());
    s.add_node(node("old")).unwrap();
    s.flush_index();
    let before = s.stable_frontier();
    let mut replacement = node("old");
    replacement.labels = vec!["CHANGED".into()];
    replacement.props = Some(json!({"v":2}));
    assert!(broken_wal(&mut s, |s| s.add_node(replacement)).is_err());
    assert_eq!(s.node_view("old").unwrap().labels, vec!["TEST"]);
    assert_eq!(s.node_view("old").unwrap().props, json!({"v":1}));
    assert!(broken_wal(&mut s, |s| s.add_node(node("new"))).is_err());
    assert!(s.node_view("new").is_none());
    assert!(s.get_u32("new").is_none());
    assert_eq!(s.stable_frontier(), before);
    drop(s);
    assert!(open(dir.path()).node_view("new").is_none());
}

#[test]
fn failed_edge_write_does_not_publish_adjacency() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = open(dir.path());
    s.add_node(node("a")).unwrap();
    s.add_node(node("b")).unwrap();
    let result = broken_wal(&mut s, |s| {
        s.add_edge(EdgeInput {
            id: Some("e".into()),
            from: "a".into(),
            to: "b".into(),
            rel: "LINK".into(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
    });
    assert!(result.is_err());
    assert!(!s.edges.contains_key(&Storage::edge_key("e")));
    assert!(!s.out_idx.contains_key(&s.get_u32("a").unwrap()));
}

#[test]
fn failed_unified_wal_rolls_back_preflight_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = open(dir.path());
    s.register_relational_schema(schema()).unwrap();
    let request = txn("retry", vec![json!({"id":"a","name":"x"})]);
    assert!(broken_wal(&mut s, |s| s.commit_transaction(request.clone())).is_err());
    assert!(rows(&s).is_empty());
    assert!(s.node_view("mixed").is_none());
    s.commit_transaction(request).unwrap();
}

#[test]
fn readers_wait_for_a_write_ack_before_observing_node() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = open(dir.path());
    let (proxy, pending) = crossbeam_channel::unbounded();
    let original = std::mem::replace(&mut s.wal_sender, proxy);
    let s = Arc::new(s);
    let writer_s = s.clone();
    let writer = std::thread::spawn(move || writer_s.add_node(node("inflight")));
    let message = pending.recv_timeout(Duration::from_secs(10)).unwrap();
    let (ready, started) = crossbeam_channel::bounded(1);
    let (answer, result) = crossbeam_channel::bounded(1);
    let reader_s = s.clone();
    let reader = std::thread::spawn(move || {
        ready.send(()).unwrap();
        answer.send(reader_s.node_view("inflight")).unwrap();
    });
    started.recv().unwrap();
    let premature = result.recv_timeout(Duration::from_millis(100));
    original.send(message).unwrap();
    let write = writer.join().unwrap();
    let value = match &premature {
        Ok(value) => value.clone(),
        Err(_) => result.recv_timeout(Duration::from_secs(10)).unwrap(),
    };
    reader.join().unwrap();
    let mut s = Arc::try_unwrap(s).ok().unwrap();
    s.wal_sender = original;
    assert!(write.is_ok());
    assert!(
        premature.is_err(),
        "read escaped the commit publication boundary"
    );
    assert!(value.is_some());
}

#[test]
fn cross_group_and_foreign_key_constraints_are_checked_before_wal() {
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    let mut package = schema();
    package.tables.push(RelationalTable {
        name: "children".into(),
        columns: vec![RelationalColumn::required("id", RelationalColumnType::Text)],
        primary_key: vec!["id".into()],
        indexes: vec![],
        foreign_keys: vec![RelationalForeignKey {
            columns: vec!["id".into()],
            referenced_table: "items".into(),
            referenced_columns: vec!["id".into()],
        }],
    });
    s.register_relational_schema(package).unwrap();
    let before = s.stable_frontier();
    let mut request = txn("groups", vec![json!({"id":"a","name":"x"})]);
    request.relational.push(request.relational[0].clone());
    assert!(s.commit_transaction(request).is_err());
    assert_eq!(s.stable_frontier(), before);
    assert!(rows(&s).is_empty());
    let mut request = txn("fk", vec![json!({"id":"missing"})]);
    request.relational[0].mutations[0].table = "children".into();
    assert!(s.commit_transaction(request).is_err());
    assert_eq!(s.stable_frontier(), before);
    assert!(s.node_view("mixed").is_none());
}

#[test]
fn durable_apply_failure_blocks_reads_writes_and_checkpoint_until_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    let conn = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_props BEFORE INSERT ON props BEGIN SELECT RAISE(ABORT, 'injected apply failure'); END;").unwrap();
    let error = s.add_node(node("durable")).unwrap_err().to_string();
    assert!(error.contains("DURABLE_COMMIT_APPLY_FAILED"), "{error}");
    assert_eq!(s.stable_frontier(), 1);
    assert!(s
        .query_sql("SELECT * FROM props", vec![])
        .unwrap_err()
        .to_string()
        .contains("RECOVERY_REQUIRED"));
    assert!(s.add_node(node("later")).is_err());
    assert!(s.save_state().is_err());
    assert!(s.perform_index_compaction().is_err());
    assert!(s.perform_index_compaction().is_err());
    conn.execute_batch("DROP TRIGGER fail_props").unwrap();
    drop(conn);
    drop(s);
    let s = open(dir.path());
    assert_eq!(s.node_view("durable").unwrap().props, json!({"v":1}));
    assert!(s.node_view("later").is_none());
    s.save_state().unwrap();
}

#[test]
fn second_supersede_write_failure_keeps_durable_closing_frame_for_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = open(dir.path());
    s.add_node(node("old")).unwrap();
    let (proxy, pending) = crossbeam_channel::unbounded();
    let original = std::mem::replace(&mut s.wal_sender, proxy);
    let s = Arc::new(s);
    let writer_s = s.clone();
    let writer = std::thread::spawn(move || {
        writer_s.supersede_node("old".into(), Some(json!({"v":2})), None)
    });
    original
        .send(pending.recv_timeout(Duration::from_secs(10)).unwrap())
        .unwrap();
    // Drop the second request's ack sender. The first frame has already fsynced.
    drop(pending.recv_timeout(Duration::from_secs(10)).unwrap());
    let result = writer.join().unwrap();
    let mut s = Arc::try_unwrap(s).ok().unwrap();
    s.wal_sender = original;
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("DURABLE_COMMIT_APPLY_FAILED"));
    assert!(s.save_state().is_err());
    drop(s);
    let s = open(dir.path());
    let old = s.node_view("old").unwrap();
    assert_eq!(old.props, json!({"v":1}));
    assert!(old.valid_to.is_some());
}

#[tokio::test]
async fn rest_rejection_preserves_frontier_and_corrected_retry_succeeds() {
    use axum::{body::Body, http::Request};
    use genesis_block_native::router::{build_router, AppState};
    use parking_lot::RwLock;
    use tower::ServiceExt;
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    s.register_relational_schema(schema()).unwrap();
    let before = s.stable_frontier();
    let storage = Arc::new(RwLock::new(s));
    let app = build_router(AppState {
        storage: storage.clone(),
        api_key: None,
    });
    let invalid = txn("rest-retry", vec![json!({"id":"a","name":"x"}); 2]);
    let valid = txn("rest-retry", vec![json!({"id":"a","name":"x"})]);
    for (request, expected) in [(invalid, 409), (valid, 200)] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/v1/transaction/commit")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), expected);
        if expected == 409 {
            assert_eq!(storage.read().stable_frontier(), before);
            assert!(rows(&storage.read()).is_empty());
        }
    }
    assert_eq!(storage.read().stable_frontier(), before + 1);
}

#[test]
fn failed_retraction_and_first_supersede_frame_preserve_live_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = open(dir.path());
    s.add_node(node("a")).unwrap();
    s.add_node(node("b")).unwrap();
    s.add_edge(EdgeInput {
        id: Some("e".into()),
        from: "a".into(),
        to: "b".into(),
        rel: "LINK".into(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
    let before = s.stable_frontier();
    assert!(broken_wal(&mut s, |s| s.retract_edge("e".into(), None)).is_err());
    assert!(s
        .edges
        .get(&Storage::edge_key("e"))
        .unwrap()
        .valid_to
        .is_none());
    assert!(broken_wal(&mut s, |s| s.supersede_node(
        "a".into(),
        Some(json!({"v":2})),
        None
    ))
    .is_err());
    assert!(s.node_view("a").unwrap().valid_to.is_none());
    assert_eq!(s.node_view("a").unwrap().props, json!({"v":1}));
    assert_eq!(s.stable_frontier(), before);
}

#[test]
fn failed_consensus_and_sync_writes_do_not_publish_graph() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = open(dir.path());
    let mut proposed = s.add_node(node("seed")).unwrap();
    proposed.id = "proposed".into();
    proposed.embedding = None;
    let proposal = s
        .propose_consensus(Event::Node(proposed.clone()), vec![])
        .unwrap();
    let signature = s.sign_vote(proposal.clone(), true);
    let peer = s.local_peer_id.clone();
    let before = s.stable_frontier();
    assert!(broken_wal(&mut s, |s| s.submit_vote(proposal, peer, true, signature)).is_err());
    assert!(s.node_view("proposed").is_none());
    assert!(s.get_u32("proposed").is_none());
    let signed = SignedEvent {
        event: Event::Node(proposed),
        signature: vec![],
        signer_peer_id: s.local_peer_id.clone(),
    };
    assert!(broken_wal(&mut s, |s| s.reconcile_state(vec![signed])).is_err());
    assert!(s.node_view("proposed").is_none());
    assert!(s.get_u32("proposed").is_none());
    assert_eq!(s.stable_frontier(), before);
}

#[test]
fn lost_ack_after_fsync_is_unknown_and_reopen_recovers_the_write() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = open(dir.path());
    let (proxy, pending) = crossbeam_channel::unbounded();
    let original = std::mem::replace(&mut s.wal_sender, proxy);
    let s = Arc::new(s);
    let writer_s = s.clone();
    let writer = std::thread::spawn(move || writer_s.add_node(node("acked-to-proxy")));
    let WalMsg::Append(event, client_ack) = pending.recv_timeout(Duration::from_secs(10)).unwrap()
    else {
        panic!("expected append")
    };
    let (ack, durable) = crossbeam_channel::bounded(1);
    original.send(WalMsg::Append(event, ack)).unwrap();
    assert_eq!(
        durable.recv_timeout(Duration::from_secs(10)).unwrap(),
        Some(1)
    );
    drop(client_ack);
    let result = writer.join().unwrap();
    let mut s = Arc::try_unwrap(s).ok().unwrap();
    s.wal_sender = original;
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("COMMIT_OUTCOME_UNKNOWN"));
    assert!(s.save_state().is_err());
    drop(s);
    let s = open(dir.path());
    assert!(s.node_view("acked-to-proxy").is_some());
    assert_eq!(s.stable_frontier(), 1);
}
