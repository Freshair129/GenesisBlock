use genesis_block_native::{
    Event, NodeInput, OpenOptions, RelationalColumn, RelationalColumnType, RelationalMutationBatch,
    RelationalMutationKind, RelationalRowMutation, RelationalSchemaPackage, RelationalTable,
    Schema6MigrationManifestV1, SignedEvent, Storage,
};
use serde_json::json;
use std::{fs, path::Path};
use tempfile::TempDir;

fn open(path: &Path, read_only: bool) -> Result<Storage, String> {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(read_only),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .map_err(|error| error.to_string())
}

fn schema5_migration_fixture(
    migration_id: &str,
    node_id: &str,
) -> (TempDir, TempDir, Schema6MigrationManifestV1) {
    let source = TempDir::new().unwrap();
    fs::write(source.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    let storage = open(source.path(), false).unwrap();
    storage
        .add_node(NodeInput {
            id: Some(node_id.into()),
            labels: vec!["MigrationFixture".into()],
            props: Some(json!({"retained": true})),
            embedding: None,
            lang: Some("en".into()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    storage.save_state().unwrap();
    drop(storage);

    let backup = TempDir::new().unwrap();
    let manifest = Storage::schema6_migration_dry_run(
        OpenOptions {
            path: source.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(2),
            retention: Some("full".into()),
        },
        migration_id.into(),
        backup.path().join("migration.gdbak"),
    )
    .unwrap();
    (source, backup, manifest)
}

fn migration_options(path: &Path) -> OpenOptions {
    OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    }
}

fn schema5_relational_migration_fixture(
    migration_id: &str,
) -> (TempDir, TempDir, Schema6MigrationManifestV1) {
    let source = TempDir::new().unwrap();
    fs::write(source.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    let storage = open(source.path(), false).unwrap();
    storage
        .register_relational_schema(RelationalSchemaPackage {
            namespace: "default".into(),
            schema_version: 1,
            previous_version: None,
            package_id: "550e8400-e29b-41d4-a716-446655440080".into(),
            schema_hash: String::new(),
            tables: vec![RelationalTable {
                name: "migration_rows".into(),
                columns: vec![RelationalColumn::required("id", RelationalColumnType::Text)],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            }],
            named_queries: vec![],
        })
        .unwrap();
    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: "550e8400-e29b-41d4-a716-446655440081".into(),
            namespace: "default".into(),
            schema_version: 1,
            operations: vec![RelationalRowMutation {
                table: "migration_rows".into(),
                kind: RelationalMutationKind::Insert,
                values: json!({"id": "source-row-1"}),
                key: None,
            }],
        })
        .unwrap();
    storage.save_state().unwrap();
    drop(storage);

    let backup = TempDir::new().unwrap();
    let manifest = Storage::schema6_migration_dry_run(
        OpenOptions {
            path: source.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(2),
            retention: Some("full".into()),
        },
        migration_id.into(),
        backup.path().join("migration.gdbak"),
    )
    .unwrap();
    (source, backup, manifest)
}

#[test]
fn in_progress_schema6_migration_fails_closed_before_projection_open() {
    for read_only in [false, true] {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("state.json"),
            r#"{"schema_version":6,"upgrade_state":"in_progress","migration_id":"550e8400-e29b-41d4-a716-446655440000","source_schema":5,"frontier":"0"}"#,
        )
        .unwrap();

        let error = open(dir.path(), read_only)
            .err()
            .expect("in-progress upgrade must not open");
        assert!(error.contains("SCHEMA_UPGRADE_IN_PROGRESS"), "{error}");
        assert!(!dir.path().join("projection.sqlite").exists());
        assert!(!dir.path().join("wal").exists());
    }
}

#[test]
fn schema6_requires_a_recognized_upgrade_state_marker() {
    for (state, expected) in [
        (None, "SCHEMA_UPGRADE_STATE_MISSING"),
        (Some("unknown"), "SCHEMA_UPGRADE_STATE_INVALID"),
    ] {
        let dir = TempDir::new().unwrap();
        let marker = state
            .map(|value| format!(r#", "upgrade_state":"{value}""#))
            .unwrap_or_default();
        fs::write(
            dir.path().join("state.json"),
            format!(r#"{{"schema_version":6{marker}}}"#),
        )
        .unwrap();

        let error = open(dir.path(), false)
            .err()
            .expect("schema 6 without a valid marker must not open");
        assert!(error.contains(expected), "{error}");
        assert!(!dir.path().join("projection.sqlite").exists());
        assert!(!dir.path().join("wal").exists());
    }
}

#[test]
fn unknown_complete_schema_control_event_fails_closed_on_open() {
    let dir = TempDir::new().unwrap();
    drop(open(dir.path(), false).unwrap());
    let unknown = json!({
        "event": {"Schema6MigrationFutureV2": {"version": 1}},
        "signature": vec![0; 64],
        "signer_peer_id": "unknown-control-peer"
    });
    fs::write(
        dir.path().join("genesis-graph.wal"),
        format!("{}\n", serde_json::to_string(&unknown).unwrap()),
    )
    .unwrap();

    let error = open(dir.path(), false)
        .err()
        .expect("unknown control must fail closed");

    assert!(
        error.contains("JOURNAL_PREFLIGHT_FAILED")
            || error.contains("UNKNOWN_CONTROL")
            || error.contains("RECOVERY_REQUIRED"),
        "{error}"
    );
}

#[test]
fn schema5_legacy_writes_do_not_mark_the_database_as_schema6() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();

    let storage = open(dir.path(), false).unwrap();
    storage
        .add_node(NodeInput {
            id: Some("legacy-v5-node".into()),
            labels: vec!["Legacy".into()],
            props: Some(json!({"kept": true})),
            embedding: None,
            lang: Some("en".into()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    storage.save_state().unwrap();

    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("state.json")).unwrap()).unwrap();
    assert_eq!(state["schema_version"], 5);
    assert!(state.get("upgrade_state").is_none());
    drop(storage);

    let projection = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let has_v6_projection: bool = projection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='hql2_record_revisions')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!has_v6_projection);
}

#[test]
fn fresh_database_is_initialized_as_schema6_ready() {
    let dir = TempDir::new().unwrap();
    drop(open(dir.path(), false).unwrap());

    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("state.json")).unwrap()).unwrap();
    assert_eq!(state["schema_version"], 6);
    assert_eq!(state["upgrade_state"], "ready");

    let projection = rusqlite::Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let has_v6_projection: bool = projection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='hql2_record_revisions')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(has_v6_projection);
}

#[test]
fn old_transaction_projection_adds_frontier_kind_with_legacy_true_default() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    fs::write(dir.path().join("identity.bin"), [7u8; 32]).unwrap();
    let projection_path = dir.path().join("projection.sqlite");
    let old_projection = rusqlite::Connection::open(&projection_path).unwrap();
    old_projection
        .execute_batch(
            "CREATE TABLE applied_transactions (
                transaction_id TEXT PRIMARY KEY,
                commit_sequence INTEGER NOT NULL,
                payload_hash TEXT NOT NULL,
                frame_seq INTEGER
            );
            INSERT INTO applied_transactions
                (transaction_id, commit_sequence, payload_hash, frame_seq)
                VALUES ('legacy-tx', 7, 'legacy-hash', 7);",
        )
        .unwrap();
    drop(old_projection);

    let storage = open(dir.path(), false).unwrap();
    drop(storage);

    let upgraded = rusqlite::Connection::open(projection_path).unwrap();
    let legacy_value: i64 = upgraded
        .query_row(
            "SELECT advances_txn_frontier FROM applied_transactions WHERE transaction_id='legacy-tx'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        legacy_value, 1,
        "preexisting transaction receipts retain semantics"
    );
}

#[test]
fn schema6_migration_chunk_is_a_typed_journal_event() {
    let event = json!({
        "Schema6MigrationChunkV1": {
            "migration_id": "550e8400-e29b-41d4-a716-446655440000",
            "manifest_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "source_database_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "source_schema": 5,
            "source_frontier": 17,
            "source_history_floors": {"graph": 1, "row": 17, "vector": 17, "annotation": 17},
            "chunk_index": 0,
            "total_chunks": 1,
            "chunk_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "baseline_records": []
        }
    });

    let decoded = serde_json::from_value::<Event>(event.clone())
        .expect("schema-v6 migration chunks must have a typed signed-WAL representation");
    assert_eq!(serde_json::to_value(decoded).unwrap(), event);
}

#[test]
fn schema6_migration_events_are_rejected_recursively_at_peer_and_public_write_ingress() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path(), false).unwrap();
    let chunk: Event = serde_json::from_value(json!({
        "Schema6MigrationChunkV1": {
            "migration_id": "550e8400-e29b-41d4-a716-446655440000",
            "manifest_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "source_database_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "source_schema": 5,
            "source_frontier": 17,
            "source_history_floors": {"graph": 1, "row": 17, "vector": 17, "annotation": 17},
            "chunk_index": 0,
            "total_chunks": 1,
            "chunk_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "baseline_records": []
        }
    }))
    .unwrap();
    let nested = Event::Batch(vec![Event::Batch(vec![chunk])]);

    let error = storage
        .reconcile_state(vec![SignedEvent {
            event: nested.clone(),
            signature: Vec::new(),
            signer_peer_id: "untrusted-peer".into(),
        }])
        .unwrap_err()
        .to_string();
    assert!(error.contains("P6_LOCAL_ONLY"), "{error}");

    let error = storage.persist(&nested).unwrap_err().to_string();
    assert!(error.contains("P6_LOCAL_ONLY"), "{error}");
}

#[test]
fn schema6_migration_dry_run_binds_the_v5_frontier_and_engine_backup() {
    let source = TempDir::new().unwrap();
    fs::write(source.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    let storage = open(source.path(), false).unwrap();
    storage
        .add_node(NodeInput {
            id: Some("migration-source-node".into()),
            labels: vec!["MigrationFixture".into()],
            props: Some(json!({"retained": true})),
            embedding: None,
            lang: Some("en".into()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    storage.save_state().unwrap();
    drop(storage);

    let backup = TempDir::new().unwrap();
    let backup_path = backup.path().join("schema5-backup.gdbak");
    let migration_id = "550e8400-e29b-41d4-a716-446655440000".to_string();
    let manifest = Storage::schema6_migration_dry_run(
        OpenOptions {
            path: source.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(2),
            retention: Some("full".into()),
        },
        migration_id.clone(),
        backup_path.clone(),
    )
    .unwrap();

    assert_eq!(manifest.migration_id, migration_id);
    assert_eq!(manifest.source_schema, 5);
    assert_eq!(manifest.backup.schema_version, 5);
    assert_eq!(manifest.backup.stable_frontier, manifest.source_frontier);
    assert_eq!(
        manifest.backup.bundle_path,
        fs::canonicalize(&backup_path).unwrap()
    );
    assert_eq!(manifest.source_record_counts["nodes"], 1);
    assert_eq!(manifest.database_id.len(), 64);
    assert_eq!(manifest.p6_manifest_sha256.len(), 64);
    assert_eq!(manifest.manifest_sha256.len(), 64);
    assert!(manifest.source_frontier > 0);
}

#[test]
fn schema6_migration_commits_graph_baselines_and_reopens_only_with_proof() {
    let source = TempDir::new().unwrap();
    fs::write(source.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    let storage = open(source.path(), false).unwrap();
    storage
        .add_node(NodeInput {
            id: Some("migration-committed-node".into()),
            labels: vec!["MigrationFixture".into()],
            props: Some(json!({"retained": true})),
            embedding: None,
            lang: Some("en".into()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    storage.save_state().unwrap();
    drop(storage);

    let backup = TempDir::new().unwrap();
    let manifest = Storage::schema6_migration_dry_run(
        OpenOptions {
            path: source.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(2),
            retention: Some("full".into()),
        },
        "550e8400-e29b-41d4-a716-446655440001".into(),
        backup.path().join("migration.gdbak"),
    )
    .unwrap();
    let report = Storage::migrate_schema5_to6(
        OpenOptions {
            path: source.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(2),
            retention: Some("full".into()),
        },
        manifest.clone(),
    )
    .unwrap();

    assert_eq!(report.migration_id, manifest.migration_id);
    assert_eq!(report.source_frontier, manifest.source_frontier);
    assert_eq!(report.target_schema, 6);
    assert_eq!(report.manifest_sha256, manifest.manifest_sha256);
    assert_eq!(report.baseline_counts["node"], 1);
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(source.path().join("state.json")).unwrap()).unwrap();
    assert_eq!(state["schema_version"], 6);
    assert_eq!(state["upgrade_state"], "ready");
    assert_eq!(state["migration_id"], manifest.migration_id);
    assert_eq!(state["migration_manifest_sha256"], manifest.manifest_sha256);
    let peer_events = open(source.path(), true).unwrap().events_since_seq(0);
    assert!(peer_events.iter().all(|signed| !matches!(
        &signed.event,
        Event::Schema6MigrationChunkV1(_) | Event::Schema6MigrationCommitV1(_)
    )));

    let projection = rusqlite::Connection::open(source.path().join("projection.sqlite")).unwrap();
    let migrated: (String, i64, Option<i64>, String) = projection
        .query_row(
            "SELECT revision_id, tx_from, tx_to, record_id FROM hql2_record_revisions WHERE database_id=?1 AND kind='node' AND record_id='migration-committed-node'",
            [manifest.database_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(migrated.1 as u64, manifest.source_frontier);
    assert!(migrated.2.is_none());
    assert_eq!(migrated.3, "migration-committed-node");
    drop(
        Storage::open(OpenOptions {
            path: source.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(true),
            vector_dim: Some(2),
            retention: Some("full".into()),
        })
        .unwrap(),
    );
}

#[test]
fn schema6_migration_resumes_when_crash_left_only_the_exact_in_progress_marker() {
    let (source, _backup, manifest) =
        schema5_migration_fixture("550e8400-e29b-41d4-a716-446655440002", "resume-marker-node");
    let marker = json!({
        "version": 1,
        "upgrade_state": "in_progress",
        "migration_id": manifest.migration_id.clone(),
        "manifest_sha256": manifest.manifest_sha256.clone(),
        "database_id": manifest.database_id.clone(),
        "source_schema": manifest.source_schema,
        "source_frontier": manifest.source_frontier,
        "backup_sha256": manifest.backup.sha256.clone(),
        "source_p6_manifest_sha256": manifest.p6_manifest_sha256.clone(),
        "source_history_floors": manifest.source_history_floors.clone(),
    });
    fs::write(
        source.path().join("schema6_migration.json"),
        serde_json::to_vec(&marker).unwrap(),
    )
    .unwrap();

    let report =
        Storage::migrate_schema5_to6(migration_options(source.path()), manifest.clone()).unwrap();

    assert_eq!(report.migration_id, manifest.migration_id);
    assert_eq!(report.baseline_counts["node"], 1);
    assert!(!source.path().join("schema6_migration.json").exists());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(source.path().join("state.json")).unwrap()
        )
        .unwrap()["upgrade_state"],
        "ready"
    );
}

#[test]
fn schema6_migration_finishes_cleanup_after_ready_state_before_sidecar_removal() {
    let (source, _backup, manifest) =
        schema5_migration_fixture("550e8400-e29b-41d4-a716-446655440007", "ready-sidecar-node");
    Storage::migrate_schema5_to6(migration_options(source.path()), manifest.clone()).unwrap();
    let marker = json!({
        "version": 1,
        "upgrade_state": "in_progress",
        "migration_id": manifest.migration_id.clone(),
        "manifest_sha256": manifest.manifest_sha256.clone(),
        "database_id": manifest.database_id.clone(),
        "source_schema": manifest.source_schema,
        "source_frontier": manifest.source_frontier,
        "backup_sha256": manifest.backup.sha256.clone(),
        "source_p6_manifest_sha256": manifest.p6_manifest_sha256.clone(),
        "source_history_floors": manifest.source_history_floors.clone(),
    });
    fs::write(
        source.path().join("schema6_migration.json"),
        serde_json::to_vec(&marker).unwrap(),
    )
    .unwrap();

    let report =
        Storage::migrate_schema5_to6(migration_options(source.path()), manifest.clone()).unwrap();

    assert_eq!(report.migration_id, manifest.migration_id);
    assert!(!source.path().join("schema6_migration.json").exists());
}

#[test]
fn schema6_migration_resume_rejects_a_different_manifest_id() {
    let (source, _backup, first) = schema5_migration_fixture(
        "550e8400-e29b-41d4-a716-446655440010",
        "resume-mismatch-node",
    );
    let second_backup = TempDir::new().unwrap();
    let second = Storage::schema6_migration_dry_run(
        migration_options(source.path()),
        "550e8400-e29b-41d4-a716-446655440011".into(),
        second_backup.path().join("migration.gdbak"),
    )
    .unwrap();
    let marker = json!({
        "version": 1,
        "upgrade_state": "in_progress",
        "migration_id": first.migration_id.clone(),
        "manifest_sha256": first.manifest_sha256.clone(),
        "database_id": first.database_id.clone(),
        "source_schema": first.source_schema,
        "source_frontier": first.source_frontier,
        "backup_sha256": first.backup.sha256.clone(),
        "source_p6_manifest_sha256": first.p6_manifest_sha256.clone(),
        "source_history_floors": first.source_history_floors.clone(),
    });
    fs::write(
        source.path().join("schema6_migration.json"),
        serde_json::to_vec(&marker).unwrap(),
    )
    .unwrap();

    let error = Storage::migrate_schema5_to6(migration_options(source.path()), second)
        .err()
        .expect("a different manifest must not resume this migration")
        .to_string();

    assert!(
        error.contains("SCHEMA6_MIGRATION_RESUME_MISMATCH"),
        "{error}"
    );
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(source.path().join("state.json")).unwrap()).unwrap();
    assert_eq!(state["schema_version"], 5);
}

#[test]
fn schema6_migration_preserves_every_graph_revision_source_interval() {
    let source = TempDir::new().unwrap();
    fs::write(source.path().join("state.json"), r#"{"schema_version":5}"#).unwrap();
    let storage = open(source.path(), false).unwrap();
    storage
        .add_node(NodeInput {
            id: Some("history-node".into()),
            labels: vec!["MigrationFixture".into()],
            props: Some(json!({"version": 1})),
            embedding: None,
            lang: Some("en".into()),
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    storage
        .supersede_node("history-node".into(), Some(json!({"version": 2})), None)
        .unwrap();
    storage
        .supersede_node("history-node".into(), Some(json!({"version": 3})), None)
        .unwrap();
    storage.save_state().unwrap();
    drop(storage);

    let source_projection =
        rusqlite::Connection::open(source.path().join("projection.sqlite")).unwrap();
    let mut source_statement = source_projection
        .prepare("SELECT frame_seq FROM node_versions WHERE id='history-node' ORDER BY frame_seq")
        .unwrap();
    let source_sequences = source_statement
        .query_map([], |row| row.get::<_, i64>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    drop(source_statement);
    drop(source_projection);
    assert_eq!(source_sequences.len(), 5);
    let expected_intervals = source_sequences
        .iter()
        .enumerate()
        .map(|(index, from)| (*from, source_sequences.get(index + 1).copied()))
        .collect::<Vec<_>>();

    let backup = TempDir::new().unwrap();
    let manifest = Storage::schema6_migration_dry_run(
        OpenOptions {
            path: source.path().to_string_lossy().into_owned(),
            page_cache_mb: Some(16),
            read_only: Some(false),
            vector_dim: Some(2),
            retention: Some("full".into()),
        },
        "550e8400-e29b-41d4-a716-446655440008".into(),
        backup.path().join("migration.gdbak"),
    )
    .unwrap();
    let report = Storage::migrate_schema5_to6(migration_options(source.path()), manifest).unwrap();

    let projection = rusqlite::Connection::open(source.path().join("projection.sqlite")).unwrap();
    let mut statement = projection
        .prepare("SELECT tx_from, tx_to FROM hql2_record_revisions WHERE kind='node' AND record_id='history-node' ORDER BY tx_from")
        .unwrap();
    let migrated = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(migrated, expected_intervals);
    assert_eq!(report.baseline_counts["node"], 5);
}

#[test]
fn schema6_migration_creates_one_row_baseline_at_the_source_frontier() {
    let (source, _backup, manifest) =
        schema5_relational_migration_fixture("550e8400-e29b-41d4-a716-446655440009");
    let report =
        Storage::migrate_schema5_to6(migration_options(source.path()), manifest.clone()).unwrap();

    let projection = rusqlite::Connection::open(source.path().join("projection.sqlite")).unwrap();
    let row: (i64, Option<i64>, String, Vec<u8>) = projection
        .query_row(
            "SELECT revision.tx_from, revision.tx_to, revision.record_id, identity.key_bytes FROM hql2_record_revisions AS revision JOIN hql2_row_identity_registry AS identity ON identity.database_id=revision.database_id AND identity.namespace=revision.namespace AND identity.row_id=revision.record_id WHERE revision.database_id=?1 AND revision.kind='row' AND revision.schema_ref='migration_rows'",
            [&manifest.database_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(row.0 as u64, manifest.source_frontier);
    assert!(row.1.is_none());
    assert_eq!(row.2.len(), 36);
    assert!(row.3.starts_with(b"HQL2RK1"));
    assert_eq!(report.baseline_counts["row"], 1);
    assert_eq!(
        report.source_history_floors["row"],
        manifest.source_frontier
    );
}

#[test]
fn schema6_migration_rejects_a_tampered_backup_before_cutover() {
    let (source, _backup, manifest) =
        schema5_migration_fixture("550e8400-e29b-41d4-a716-446655440003", "backup-tamper-node");
    use std::io::Write;
    let mut bundle = fs::OpenOptions::new()
        .append(true)
        .open(&manifest.backup.bundle_path)
        .unwrap();
    bundle.write_all(b"tamper").unwrap();
    drop(bundle);

    let error = Storage::migrate_schema5_to6(migration_options(source.path()), manifest)
        .unwrap_err()
        .to_string();

    assert!(
        error.contains("SCHEMA6_MIGRATION_BACKUP_MISMATCH"),
        "{error}"
    );
    assert!(!source.path().join("schema6_migration.json").exists());
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(source.path().join("state.json")).unwrap()).unwrap();
    assert_eq!(state["schema_version"], 5);
}

#[test]
fn schema6_ordinary_open_rejects_ready_state_without_matching_signed_manifest() {
    let (source, _backup, manifest) =
        schema5_migration_fixture("550e8400-e29b-41d4-a716-446655440004", "proof-tamper-node");
    Storage::migrate_schema5_to6(migration_options(source.path()), manifest).unwrap();
    let state_path = source.path().join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["migration_manifest_sha256"] =
        json!("0000000000000000000000000000000000000000000000000000000000000000");
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

    let error = open(source.path(), true)
        .err()
        .expect("tampered proof must fail open");

    assert!(error.contains("RECOVERY_REQUIRED"), "{error}");
}

#[test]
fn schema6_migration_authority_survives_fold_and_wal_only_projection_rebuild() {
    let (source, _backup, manifest) =
        schema5_migration_fixture("550e8400-e29b-41d4-a716-446655440005", "fold-recovery-node");
    Storage::migrate_schema5_to6(migration_options(source.path()), manifest.clone()).unwrap();
    let storage = open(source.path(), false).unwrap();
    storage.compact().unwrap();
    drop(storage);

    fs::remove_file(source.path().join("state.json")).unwrap();
    fs::remove_file(source.path().join("projection.sqlite")).unwrap();
    let recovered = open(source.path(), false).unwrap();
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(source.path().join("state.json")).unwrap()).unwrap();
    assert_eq!(state["schema_version"], 6);
    assert_eq!(state["upgrade_state"], "ready");
    assert_eq!(state["migration_id"], manifest.migration_id);
    let projection = rusqlite::Connection::open(source.path().join("projection.sqlite")).unwrap();
    let migrated: i64 = projection
        .query_row(
            "SELECT COUNT(*) FROM hql2_record_revisions WHERE database_id=?1 AND kind='node' AND record_id='fold-recovery-node'",
            [&manifest.database_id],
            |row| row.get(0),
        )
        .unwrap();
    let committed: i64 = projection
        .query_row(
            "SELECT COUNT(*) FROM hql2_schema6_migrations WHERE migration_id=?1 AND status='committed'",
            [&manifest.migration_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(migrated, 1);
    assert_eq!(committed, 1);
    drop(projection);
    drop(recovered);
}

#[test]
fn schema6_migration_rerun_with_same_manifest_is_idempotent() {
    let (source, _backup, manifest) =
        schema5_migration_fixture("550e8400-e29b-41d4-a716-446655440006", "rerun-node");
    let first =
        Storage::migrate_schema5_to6(migration_options(source.path()), manifest.clone()).unwrap();
    let second = Storage::migrate_schema5_to6(migration_options(source.path()), manifest).unwrap();

    assert_eq!(first.target_frontier, second.target_frontier);
    assert_eq!(first.generation, second.generation);
    assert_eq!(first.baseline_counts, second.baseline_counts);
}
