use genesis_block_native::{
    query::hql2::QueryOutcomeV2,
    uee_v2::{RecordKindV2, RecordRefV2, RecordRevisionTransactionV1},
    AccessContext, BatchInput, EdgeInput, Event, GenesisTransaction, GenesisTransactionEvent,
    NodeInput, OpenOptions, RelationalColumn, RelationalColumnType, RelationalMutationBatch,
    RelationalMutationKind, RelationalRowMutation, RelationalSchemaPackage, RelationalTable,
    SignedEvent, Storage,
};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn record_ref() -> Value {
    json!({
        "database_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "namespace": "default",
        "kind": "node",
        "id": "node-1",
        "revision": "550e8400-e29b-41d4-a716-446655440000"
    })
}

fn database_id(storage: &Storage) -> String {
    let request = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"database-id",
        "namespace":"default",
        "ir":{"contract_version":"query-ir.v2","nodes":[{"id":"v","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}}],"root":"v","parameter_types":{"xs":"List<I64>"}},
        "params":{"xs":{"type":"List<I64>","value":[]}}
    }))
    .unwrap();
    let outcome = storage
        .query_v2(
            AccessContext {
                principal: "durable-revision-test".into(),
                namespace: "default".into(),
            },
            request,
        )
        .unwrap();
    let QueryOutcomeV2::Rows(result) = outcome else {
        panic!("values request must return rows")
    };
    result.snapshot.database_id
}

fn relational_schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "durable_test".into(),
        schema_version: 1,
        previous_version: None,
        package_id: "550e8400-e29b-41d4-a716-446655440010".into(),
        schema_hash: String::new(),
        tables: vec![
            RelationalTable {
                name: "records".into(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::Text),
                    RelationalColumn {
                        name: "value".into(),
                        column_type: RelationalColumnType::Text,
                        nullable: true,
                        default: None,
                    },
                ],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            },
            RelationalTable {
                name: "nullable_keys".into(),
                columns: vec![
                    RelationalColumn {
                        name: "id".into(),
                        column_type: RelationalColumnType::Text,
                        nullable: true,
                        default: None,
                    },
                    RelationalColumn {
                        name: "value".into(),
                        column_type: RelationalColumnType::Text,
                        nullable: true,
                        default: None,
                    },
                ],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            },
        ],
        named_queries: vec![],
    }
}

fn oversized_key_batch(value: String) -> RelationalMutationBatch {
    RelationalMutationBatch {
        mutation_id: "550e8400-e29b-41d4-a716-446655440011".into(),
        namespace: "durable_test".into(),
        schema_version: 1,
        operations: vec![RelationalRowMutation {
            table: "records".into(),
            kind: RelationalMutationKind::Insert,
            values: json!({"id": value}),
            key: None,
        }],
    }
}

fn relational_batch(
    mutation_id: &str,
    operation: RelationalRowMutation,
) -> RelationalMutationBatch {
    RelationalMutationBatch {
        mutation_id: mutation_id.into(),
        namespace: "durable_test".into(),
        schema_version: 1,
        operations: vec![operation],
    }
}

fn row_operation(kind: RelationalMutationKind, value: Option<&str>) -> RelationalRowMutation {
    match kind {
        RelationalMutationKind::Delete => RelationalRowMutation {
            table: "records".into(),
            kind,
            values: Value::Null,
            key: Some(json!({"id": "row-1"})),
        },
        RelationalMutationKind::Update => RelationalRowMutation {
            table: "records".into(),
            kind,
            values: json!({"value": value.unwrap()}),
            key: Some(json!({"id": "row-1"})),
        },
        _ => RelationalRowMutation {
            table: "records".into(),
            kind,
            values: json!({"id": "row-1", "value": value}),
            key: None,
        },
    }
}

fn relational_revisions(storage: &Storage, mutation_id: &str) -> RecordRevisionTransactionV1 {
    storage
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::RelationalRows {
                mutation_id: event_id,
                record_revision_transaction,
                ..
            } if event_id == mutation_id => record_revision_transaction,
            _ => None,
        })
        .expect("schema-v6 relational writes must carry a durable revision envelope")
}

fn graph_transaction(transaction_id: &str, title: &str) -> GenesisTransaction {
    GenesisTransaction {
        transaction_id: transaction_id.into(),
        expected_frontier: None,
        relational: vec![],
        graph: BatchInput {
            nodes: vec![NodeInput {
                id: Some("revision-node".into()),
                labels: vec!["Note".into()],
                props: Some(json!({"title": title})),
                embedding: None,
                lang: Some("en".into()),
                valid_from: Some("2026-09-28T00:00:00Z".into()),
                caused_by: None,
                ttl: None,
                collection: None,
            }],
            edges: vec![EdgeInput {
                id: Some("revision-edge".into()),
                from: "revision-node".into(),
                to: "revision-node".into(),
                rel: "LINK".into(),
                props: Some(json!({"weight": 1})),
                valid_from: Some("2026-09-28T00:00:00Z".into()),
                supersede: None,
                impact: None,
                caused_by: None,
            }],
        },
        vectors: vec![],
    }
}

#[test]
fn record_reference_requires_database_id_and_canonical_uuid_v4_revision() {
    let valid = record_ref();
    let decoded = RecordRefV2::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), valid);

    let mut missing_database = record_ref();
    missing_database
        .as_object_mut()
        .unwrap()
        .remove("database_id");
    assert!(RecordRefV2::from_value(missing_database).is_err());

    let mut missing_revision = record_ref();
    missing_revision.as_object_mut().unwrap().remove("revision");
    assert!(RecordRefV2::from_value(missing_revision).is_err());

    let mut unknown_field = record_ref();
    unknown_field["fallback_to_latest"] = json!(true);
    assert!(RecordRefV2::from_value(unknown_field).is_err());

    let mut invalid_namespace = record_ref();
    invalid_namespace["namespace"] = json!("Default");
    assert!(RecordRefV2::from_value(invalid_namespace).is_err());

    let mut empty_id = record_ref();
    empty_id["id"] = json!("");
    assert!(RecordRefV2::from_value(empty_id).is_err());

    for bad_database_id in ["A".repeat(64), "a".repeat(63), "g".repeat(64)] {
        let mut value = record_ref();
        value["database_id"] = json!(bad_database_id);
        assert!(RecordRefV2::from_value(value).is_err());
    }

    for bad_revision in [
        "550e8400-e29b-11d4-a716-446655440000",
        "550e8400-e29b-41d4-3716-446655440000",
        "550E8400-E29B-41D4-A716-446655440000",
        "550e8400e29b41d4a716446655440000",
        "not-a-uuid",
    ] {
        let mut value = record_ref();
        value["revision"] = json!(bad_revision);
        assert!(RecordRefV2::from_value(value).is_err());
    }
}

#[test]
fn revision_transaction_validates_lineage_cas_and_nonempty_validity() {
    let valid = json!({
        "transaction_id": "550e8400-e29b-41d4-a716-446655440001",
        "origin_database_id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "mutations": [{
            "namespace": "default",
            "kind": "node",
            "id": "node-1",
            "revision_id": "550e8400-e29b-41d4-a716-446655440002",
            "operation": "upsert",
            "valid_from": "2026-09-28T00:00:00Z",
            "valid_to": null,
            "payload": {"labels":["Person"],"props":{"name":"Ada"}}
        }]
    });
    let decoded = RecordRevisionTransactionV1::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), valid);

    let mut stale_predecessor = valid.clone();
    stale_predecessor["mutations"][0]["expected_revision_id"] =
        json!("550e8400-e29b-41d4-a716-446655440003");
    stale_predecessor["mutations"][0]["predecessor_revision_id"] =
        json!("550e8400-e29b-41d4-a716-446655440004");
    assert!(RecordRevisionTransactionV1::from_value(stale_predecessor).is_err());

    let mut reused_revision = valid.clone();
    reused_revision["mutations"][0]["expected_revision_id"] =
        json!("550e8400-e29b-41d4-a716-446655440002");
    reused_revision["mutations"][0]["predecessor_revision_id"] =
        json!("550e8400-e29b-41d4-a716-446655440002");
    assert!(RecordRevisionTransactionV1::from_value(reused_revision).is_err());

    let mut empty_interval = valid.clone();
    empty_interval["mutations"][0]["valid_to"] = json!("2026-09-28T00:00:00Z");
    assert!(RecordRevisionTransactionV1::from_value(empty_interval).is_err());

    let mut wrong_origin = valid.clone();
    wrong_origin["origin_database_id"] = json!("B".repeat(64));
    assert!(RecordRevisionTransactionV1::from_value(wrong_origin).is_err());

    let row = json!({
        "transaction_id": "550e8400-e29b-41d4-a716-446655440005",
        "origin_database_id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "mutations": [{
            "namespace": "durable_test",
            "kind": "row",
            "id": "550e8400-e29b-41d4-a716-446655440006",
            "revision_id": "550e8400-e29b-41d4-a716-446655440007",
            "operation": "upsert",
            "valid_from": "2026-09-28T00:00:00Z",
            "schema_ref": "records",
            "schema_version": 1,
            "key_codec_version": 1,
            "key_bytes": [72, 81, 76, 50],
            "payload": {"after_image":{"id":"row-1"}}
        }]
    });
    assert!(RecordRevisionTransactionV1::from_value(row.clone()).is_ok());

    let mut invalid_row_id = row.clone();
    invalid_row_id["mutations"][0]["id"] = json!("not-a-row-uuid");
    assert!(RecordRevisionTransactionV1::from_value(invalid_row_id).is_err());

    let mut missing_row_codec = row;
    missing_row_codec["mutations"][0]
        .as_object_mut()
        .unwrap()
        .remove("key_bytes");
    assert!(RecordRevisionTransactionV1::from_value(missing_row_codec).is_err());
}

#[test]
fn schema5_transaction_event_serialization_omits_revision_extension() {
    let event = GenesisTransactionEvent {
        transaction_id: "legacy-tx".into(),
        origin_commit_seq: 0,
        local_frame_seq: None,
        payload_hash: "legacy-hash".into(),
        relational: vec![],
        nodes: vec![],
        edges: vec![],
        vectors: vec![],
        node_retractions: vec![],
        advances_txn_frontier: true,
        record_revision_transaction: None,
    };
    let value = serde_json::to_value(&event).unwrap();
    assert!(value.get("record_revision_transaction").is_none());
    assert!(value.get("advances_txn_frontier").is_none());
    assert_eq!(
        serde_json::from_value::<GenesisTransactionEvent>(value)
            .unwrap()
            .transaction_id,
        "legacy-tx"
    );

    let mut graph_event = event;
    graph_event.advances_txn_frontier = false;
    let value = serde_json::to_value(&graph_event).unwrap();
    assert_eq!(value.get("advances_txn_frontier"), Some(&json!(false)));
    assert!(
        !serde_json::from_value::<GenesisTransactionEvent>(value)
            .unwrap()
            .advances_txn_frontier
    );
}

#[test]
fn database_id_is_stable_for_a_persisted_key_and_distinct_for_new_lineages() {
    let first = TempDir::new().unwrap();
    let initial_id = {
        let storage = open(first.path());
        database_id(&storage)
    };
    let reopened_id = {
        let storage = open(first.path());
        database_id(&storage)
    };
    assert_eq!(initial_id, reopened_id);
    assert_eq!(initial_id.len(), 64);
    assert!(initial_id
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));

    let second = TempDir::new().unwrap();
    let other_id = database_id(&open(second.path()));
    assert_ne!(initial_id, other_id);
}

#[test]
fn schema6_relational_mutations_enforce_the_runtime_key_codec_limit_before_append() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .register_relational_schema(relational_schema())
        .unwrap();
    let frontier = storage.stable_frontier();

    let error = storage
        .apply_relational_batch(oversized_key_batch("x".repeat(16 * 1024)))
        .unwrap_err()
        .to_string();

    assert!(error.contains("key_codec_value_too_large"), "{error}");
    assert_eq!(storage.stable_frontier(), frontier);
}

#[test]
fn schema6_direct_relational_events_cannot_bypass_key_codec_preflight() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .register_relational_schema(relational_schema())
        .unwrap();
    let frontier = storage.stable_frontier();
    let event = Event::RelationalRows {
        namespace: "durable_test".into(),
        schema_version: 1,
        mutation_id: String::new(),
        payload_hash: String::new(),
        affected_rows: 0,
        mutations: oversized_key_batch("x".repeat(16 * 1024)).operations,
        record_revision_transaction: None,
    };

    let error = storage.persist(&event).unwrap_err().to_string();

    assert!(error.contains("key_codec_value_too_large"), "{error}");
    assert_eq!(storage.stable_frontier(), frontier);
}

#[test]
fn schema5_relational_writes_keep_legacy_key_size_behavior() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    let storage = open(dir.path());
    storage
        .register_relational_schema(relational_schema())
        .unwrap();

    let result = storage
        .apply_relational_batch(oversized_key_batch("x".repeat(16 * 1024)))
        .unwrap();

    assert_eq!(result.affected_rows, 1);
    assert_eq!(storage.query_ir_capabilities()["storage_schema_version"], 5);
}

#[test]
fn schema6_relational_row_ids_revisions_and_key_codec_survive_reinsert_and_rebuild() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .register_relational_schema(relational_schema())
        .unwrap();

    storage
        .apply_relational_batch(relational_batch(
            "550e8400-e29b-41d4-a716-446655440020",
            row_operation(RelationalMutationKind::Insert, Some("first")),
        ))
        .unwrap();
    let first = relational_revisions(&storage, "550e8400-e29b-41d4-a716-446655440020");
    let first_row = &first.mutations[0];
    assert_eq!(first_row.kind, RecordKindV2::Row);
    assert_eq!(first_row.namespace, "durable_test");
    assert_eq!(first_row.schema_ref.as_deref(), Some("records"));
    assert_eq!(first_row.key_codec_version, Some(1));
    assert!(first_row
        .key_bytes
        .as_deref()
        .is_some_and(|bytes| bytes.starts_with(b"HQL2RK1")));
    assert_eq!(
        first_row.key_bytes.as_deref(),
        Some(b"HQL2RK1\x01\x01\x00\x00\x00\x05row-1".as_slice())
    );
    let row_id = first_row.id.clone();
    let first_revision = first_row.revision_id.clone();

    storage
        .apply_relational_batch(relational_batch(
            "550e8400-e29b-41d4-a716-446655440021",
            row_operation(RelationalMutationKind::Upsert, Some("updated")),
        ))
        .unwrap();
    let updated = relational_revisions(&storage, "550e8400-e29b-41d4-a716-446655440021");
    assert_eq!(updated.mutations[0].id, row_id);
    assert_eq!(
        updated.mutations[0].expected_revision_id.as_deref(),
        Some(first_revision.as_str())
    );

    storage
        .apply_relational_batch(relational_batch(
            "550e8400-e29b-41d4-a716-446655440022",
            row_operation(RelationalMutationKind::Delete, None),
        ))
        .unwrap();
    let deleted = relational_revisions(&storage, "550e8400-e29b-41d4-a716-446655440022");
    assert_eq!(deleted.mutations[0].id, row_id);
    assert_eq!(
        deleted.mutations[0].operation,
        genesis_block_native::uee_v2::RevisionOperationV1::Retract
    );

    storage
        .apply_relational_batch(relational_batch(
            "550e8400-e29b-41d4-a716-446655440023",
            row_operation(RelationalMutationKind::Insert, Some("reinserted")),
        ))
        .unwrap();
    let reinserted = relational_revisions(&storage, "550e8400-e29b-41d4-a716-446655440023");
    assert_ne!(reinserted.mutations[0].id, row_id);

    storage.compact().unwrap();
    drop(storage);
    fs::remove_file(dir.path().join("projection.sqlite")).unwrap();
    let _reopened = open(dir.path());
    let projection = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let revision_count: u64 = projection
        .query_row(
            "SELECT COUNT(*) FROM hql2_record_revisions WHERE kind='row'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let identity_count: u64 = projection
        .query_row(
            "SELECT COUNT(*) FROM hql2_row_identity_registry WHERE namespace='durable_test' AND table_name='records'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let (registered_key, created_revision): (Vec<u8>, String) = projection
        .query_row(
            "SELECT key_bytes, created_revision_id FROM hql2_row_identity_registry
             WHERE namespace='durable_test' AND table_name='records' AND row_id=?1",
            [&row_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(revision_count, 4);
    assert_eq!(identity_count, 2);
    assert_eq!(
        registered_key,
        b"HQL2RK1\x01\x01\x00\x00\x00\x05row-1".to_vec()
    );
    assert_eq!(created_revision, first_revision);
}

#[test]
fn schema6_relational_rows_in_unified_transaction_share_the_revision_envelope() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .register_relational_schema(relational_schema())
        .unwrap();

    storage
        .commit_transaction(GenesisTransaction {
            transaction_id: "hql2-row-unified-transaction".into(),
            expected_frontier: None,
            relational: vec![genesis_block_native::RelationalMutationGroup {
                namespace: "durable_test".into(),
                mutations: vec![row_operation(
                    RelationalMutationKind::Insert,
                    Some("transactional"),
                )],
            }],
            graph: BatchInput {
                nodes: vec![],
                edges: vec![],
            },
            vectors: vec![],
        })
        .unwrap();

    let transaction = storage
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::Transaction(transaction)
                if transaction.transaction_id == "hql2-row-unified-transaction" =>
            {
                Some(transaction)
            }
            _ => None,
        })
        .unwrap();
    let revisions = transaction
        .record_revision_transaction
        .expect("row and transaction mutations must share one envelope");
    assert_eq!(revisions.mutations.len(), 1);
    assert_eq!(revisions.mutations[0].kind, RecordKindV2::Row);
}

#[test]
fn schema6_relational_revision_binding_rejects_unversioned_and_mismatched_events() {
    let source_dir = TempDir::new().unwrap();
    let source = open(source_dir.path());
    source
        .register_relational_schema(relational_schema())
        .unwrap();
    source
        .apply_relational_batch(relational_batch(
            "550e8400-e29b-41d4-a716-446655440024",
            row_operation(RelationalMutationKind::Insert, Some("bound")),
        ))
        .unwrap();
    let event = source
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            event @ Event::RelationalRows { .. } => Some(event),
            _ => None,
        })
        .unwrap();

    let unversioned_dir = TempDir::new().unwrap();
    let unversioned = open(unversioned_dir.path());
    unversioned
        .register_relational_schema(relational_schema())
        .unwrap();
    let mut unbound_event = event.clone();
    if let Event::RelationalRows {
        record_revision_transaction,
        ..
    } = &mut unbound_event
    {
        *record_revision_transaction = None;
    }
    let frontier = unversioned.stable_frontier();
    let error = unversioned.persist(&unbound_event).unwrap_err().to_string();
    assert!(error.contains("UPGRADE_REQUIRED"), "{error}");
    assert_eq!(unversioned.stable_frontier(), frontier);

    let mismatched_dir = TempDir::new().unwrap();
    let mismatched = open(mismatched_dir.path());
    mismatched
        .register_relational_schema(relational_schema())
        .unwrap();
    let mut mismatched_event = event;
    if let Event::RelationalRows {
        record_revision_transaction: Some(revisions),
        ..
    } = &mut mismatched_event
    {
        revisions.mutations[0].payload["after_image"]["value"] = json!("forged");
    }
    let frontier = mismatched.stable_frontier();
    let error = mismatched
        .persist(&mismatched_event)
        .unwrap_err()
        .to_string();
    assert!(error.contains("REVISION_EVENT_MISMATCH"), "{error}");
    assert_eq!(mismatched.stable_frontier(), frontier);
}

#[test]
fn schema6_nullable_primary_key_duplicates_keep_distinct_row_ids() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .register_relational_schema(relational_schema())
        .unwrap();
    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: "550e8400-e29b-41d4-a716-446655440025".into(),
            namespace: "durable_test".into(),
            schema_version: 1,
            operations: ["first", "second"]
                .into_iter()
                .map(|value| RelationalRowMutation {
                    table: "nullable_keys".into(),
                    kind: RelationalMutationKind::Insert,
                    values: json!({"id": null, "value": value}),
                    key: None,
                })
                .collect(),
        })
        .unwrap();

    let revisions = relational_revisions(&storage, "550e8400-e29b-41d4-a716-446655440025");
    assert_eq!(revisions.mutations.len(), 2);
    assert_ne!(revisions.mutations[0].id, revisions.mutations[1].id);
    assert_eq!(
        revisions.mutations[0].key_bytes,
        revisions.mutations[1].key_bytes
    );
    assert_eq!(
        revisions.mutations[0].key_bytes.as_deref(),
        Some(b"HQL2RK1\x01\x00\x00\x00\x00\x00".as_slice())
    );

    let projection = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let registered: u64 = projection
        .query_row(
            "SELECT COUNT(*) FROM hql2_row_identity_registry
             WHERE namespace='durable_test' AND table_name='nullable_keys'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(registered, 2);
}

#[test]
fn schema6_node_transaction_persists_its_revision_identity_in_wal_and_projection() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let result = storage
        .commit_transaction(GenesisTransaction {
            transaction_id: "hql2-node-revision-1".into(),
            expected_frontier: None,
            relational: vec![],
            graph: BatchInput {
                nodes: vec![NodeInput {
                    id: Some("revision-node".into()),
                    labels: vec!["Note".into()],
                    props: Some(json!({"title": "first"})),
                    embedding: None,
                    lang: Some("en".into()),
                    valid_from: Some("2026-09-28T00:00:00Z".into()),
                    caused_by: None,
                    ttl: None,
                    collection: None,
                }],
                edges: vec![],
            },
            vectors: vec![],
        })
        .unwrap();

    let event = storage
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::Transaction(event) if event.transaction_id == "hql2-node-revision-1" => {
                Some(event)
            }
            _ => None,
        })
        .expect("committed transaction must be present in the WAL");
    let revision = event
        .record_revision_transaction
        .expect("schema-v6 graph writes must carry a durable revision transaction");
    assert_eq!(revision.origin_database_id, database_id(&storage));
    assert_eq!(revision.mutations.len(), 1);
    let mutation = &revision.mutations[0];
    assert_eq!(mutation.kind, RecordKindV2::Node);
    assert_eq!(mutation.id, "revision-node");
    assert_eq!(
        mutation.operation,
        genesis_block_native::uee_v2::RevisionOperationV1::Upsert
    );
    assert_eq!(mutation.expected_revision_id, None);
    assert_eq!(mutation.predecessor_revision_id, None);
    assert_eq!(
        mutation.valid_from.to_rfc3339(),
        "2026-09-28T00:00:00+00:00"
    );

    let revision_id = mutation.revision_id.clone();
    let commit_sequence = result.commit_sequence;
    storage.compact().unwrap();
    drop(storage);
    fs::remove_file(dir.path().join("projection.sqlite")).unwrap();
    let reopened = open(dir.path());
    let replayed = reopened
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::Transaction(event) if event.transaction_id == "hql2-node-revision-1" => {
                event.record_revision_transaction
            }
            _ => None,
        })
        .expect("revision identity must survive reopen");
    assert_eq!(replayed.mutations[0].revision_id, revision_id);

    let projection = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let stored: (String, i64) = projection
        .query_row(
            "SELECT revision_id, tx_from FROM hql2_record_revisions WHERE record_id='revision-node'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(stored.0, revision_id);
    assert_eq!(stored.1 as u64, commit_sequence);
}

#[test]
fn schema6_direct_graph_writes_persist_revision_envelopes_in_the_wal() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let direct_node = storage
        .add_node(NodeInput {
            id: Some("direct-revision-node".into()),
            labels: vec!["Note".into()],
            props: Some(json!({"title": "direct"})),
            embedding: None,
            lang: Some("en".into()),
            valid_from: Some("2026-09-28T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    storage
        .add_edge(EdgeInput {
            id: Some("direct-revision-edge".into()),
            from: "direct-revision-node".into(),
            to: "direct-revision-node".into(),
            rel: "LINK".into(),
            props: Some(json!({})),
            valid_from: Some("2026-09-28T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    let mutations = storage
        .events_since_seq(0)
        .into_iter()
        .filter_map(|signed| match signed.event {
            Event::Transaction(transaction) => transaction.record_revision_transaction,
            _ => None,
        })
        .flat_map(|transaction| transaction.mutations)
        .collect::<Vec<_>>();

    assert!(mutations
        .iter()
        .any(|mutation| mutation.id == "direct-revision-node"));
    assert!(mutations
        .iter()
        .any(|mutation| mutation.id == "direct-revision-edge"));
    let original_node_revision = mutations
        .iter()
        .find(|mutation| mutation.id == "direct-revision-node")
        .unwrap()
        .revision_id
        .clone();
    let original_edge_revision = mutations
        .iter()
        .find(|mutation| mutation.id == "direct-revision-edge")
        .unwrap()
        .revision_id
        .clone();

    storage
        .retract_edge("direct-revision-edge".into(), None)
        .unwrap();
    let edge_retraction = storage
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::Transaction(transaction)
                if transaction.edges.iter().any(|edge| edge.id == "direct-revision-edge")
                    && transaction
                        .record_revision_transaction
                        .as_ref()
                        .is_some_and(|revisions| {
                            revisions.mutations.iter().any(|mutation| {
                                mutation.id == "direct-revision-edge"
                                    && mutation.operation
                                        == genesis_block_native::uee_v2::RevisionOperationV1::Retract
                            })
                        }) =>
            {
                transaction.record_revision_transaction
            }
            _ => None,
        })
        .expect("edge retraction must preserve its revision envelope");
    assert_eq!(
        edge_retraction.mutations[0].expected_revision_id.as_deref(),
        Some(original_edge_revision.as_str())
    );

    storage.retract_node("direct-revision-node").unwrap();
    let node_retraction = storage
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::Transaction(transaction) if !transaction.node_retractions.is_empty() => {
                transaction.record_revision_transaction
            }
            _ => None,
        })
        .expect("node retraction must preserve its revision envelope");
    let mutation = &node_retraction.mutations[0];
    assert_eq!(mutation.kind, RecordKindV2::Node);
    assert_eq!(mutation.id, "direct-revision-node");
    assert_eq!(
        mutation.operation,
        genesis_block_native::uee_v2::RevisionOperationV1::Retract
    );
    assert!(mutation.expected_revision_id.is_some());
    let retracted_node_revision = mutation.revision_id.clone();

    let frontier = storage.stable_frontier();
    let error = storage
        .persist(&Event::Node(direct_node.clone()))
        .unwrap_err()
        .to_string();
    assert!(error.contains("UPGRADE_REQUIRED"), "{error}");
    assert_eq!(storage.stable_frontier(), frontier);

    let mut legacy_node = direct_node.clone();
    legacy_node.id = "legacy-peer-node".into();
    let error = storage
        .reconcile_state(vec![SignedEvent {
            event: Event::Node(legacy_node),
            signature: vec![],
            signer_peer_id: storage.local_peer_id.clone(),
        }])
        .unwrap_err()
        .to_string();
    assert!(error.contains("UPGRADE_REQUIRED"), "{error}");
    assert!(storage.node_view("legacy-peer-node").is_none());
    assert_eq!(storage.stable_frontier(), frontier);

    let mut mismatched = storage
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::Transaction(transaction) if !transaction.nodes.is_empty() => Some(transaction),
            _ => None,
        })
        .unwrap();
    mismatched.transaction_id = "mismatched-revision-event".into();
    mismatched.nodes[0].props = json!({"title": "payload changed after signing"});
    mismatched.payload_hash = "different-payload".into();
    let error = storage
        .persist(&Event::Transaction(mismatched))
        .unwrap_err()
        .to_string();
    assert!(error.contains("REVISION_EVENT_MISMATCH"), "{error}");
    assert_eq!(storage.stable_frontier(), frontier);

    storage.compact().unwrap();
    drop(storage);
    fs::remove_file(dir.path().join("projection.sqlite")).unwrap();
    let reopened = open(dir.path());
    assert!(reopened.node_view("direct-revision-node").is_none());
    let projection = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let node_revisions: Vec<String> = projection
        .prepare(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE record_id='direct-revision-node' ORDER BY tx_from",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap();
    assert_eq!(
        node_revisions,
        vec![original_node_revision, retracted_node_revision]
    );
}

#[test]
fn schema6_graph_batch_uses_one_revision_bearing_wal_transaction() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .execute_batch(BatchInput {
            nodes: vec![NodeInput {
                id: Some("batch-revision-node".into()),
                labels: vec!["Note".into()],
                props: Some(json!({})),
                embedding: None,
                lang: Some("en".into()),
                valid_from: Some("2026-09-28T00:00:00Z".into()),
                caused_by: None,
                ttl: None,
                collection: None,
            }],
            edges: vec![EdgeInput {
                id: Some("batch-revision-edge".into()),
                from: "batch-revision-node".into(),
                to: "batch-revision-node".into(),
                rel: "LINK".into(),
                props: Some(json!({})),
                valid_from: Some("2026-09-28T00:00:00Z".into()),
                supersede: None,
                impact: None,
                caused_by: None,
            }],
        })
        .unwrap();

    let event = storage
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::Transaction(transaction)
                if transaction
                    .nodes
                    .iter()
                    .any(|node| node.id == "batch-revision-node") =>
            {
                Some(transaction)
            }
            _ => None,
        })
        .expect("graph batch must be one transaction frame");
    let revisions = event
        .record_revision_transaction
        .expect("schema-v6 graph batch requires durable revisions");
    assert_eq!(revisions.mutations.len(), 2);
    assert!(revisions
        .mutations
        .iter()
        .any(|mutation| mutation.id == "batch-revision-node"));
    assert!(revisions
        .mutations
        .iter()
        .any(|mutation| mutation.id == "batch-revision-edge"));
}

#[test]
fn schema6_node_revisions_use_compare_and_swap_and_survive_fold_rebuild() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .commit_transaction(graph_transaction("revision-tx-first", "first"))
        .unwrap();
    let first = storage
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::Transaction(event) if event.transaction_id == "revision-tx-first" => {
                event.record_revision_transaction
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(first.mutations.len(), 2);
    let first_revision = first.mutations[0].revision_id.clone();

    let second_input = graph_transaction("revision-tx-second", "second");
    let second_result = storage.commit_transaction(second_input.clone()).unwrap();
    let second = storage
        .events_since_seq(0)
        .into_iter()
        .find_map(|signed| match signed.event {
            Event::Transaction(event) if event.transaction_id == "revision-tx-second" => {
                event.record_revision_transaction
            }
            _ => None,
        })
        .unwrap();
    let second_mutation = &second.mutations[0];
    assert_ne!(second_mutation.revision_id, first_revision);
    assert_eq!(
        second_mutation.expected_revision_id.as_deref(),
        Some(first_revision.as_str())
    );
    assert_eq!(
        second_mutation.predecessor_revision_id.as_deref(),
        Some(first_revision.as_str())
    );
    assert_eq!(second.mutations[1].kind, RecordKindV2::Edge);
    assert_eq!(second.mutations[1].id, "revision-edge");
    assert_eq!(
        storage
            .commit_transaction(second_input)
            .unwrap()
            .commit_sequence,
        second_result.commit_sequence
    );

    let second_revision = second_mutation.revision_id.clone();
    let second_sequence = second_result.commit_sequence;
    storage.compact().unwrap();
    drop(storage);
    fs::remove_file(dir.path().join("projection.sqlite")).unwrap();
    let reopened = open(dir.path());
    let projection = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let rows: Vec<(String, Option<i64>)> = projection
        .prepare(
            "SELECT revision_id, tx_to FROM hql2_record_revisions
             WHERE record_id='revision-node' ORDER BY tx_from",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], (first_revision, Some(second_sequence as i64)));
    assert_eq!(rows[1], (second_revision, None));
    assert_eq!(
        reopened.node_view("revision-node").unwrap().props["title"],
        "second"
    );
}
