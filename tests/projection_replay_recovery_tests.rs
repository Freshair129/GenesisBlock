use ed25519_dalek::{Signer, SigningKey};
use genesis_block_native::{
    Event, LogicalClock, NodeOutput, OpenOptions, RelationalColumn, RelationalColumnType,
    RelationalMutationKind, RelationalRowMutation, RelationalSchemaPackage, RelationalTable,
    SignedEvent, Storage,
};
use rusqlite::{backup::StepResult, Connection};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions as FsOpenOptions};
use std::io::Write;
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

fn schema(version: u32) -> RelationalSchemaPackage {
    let mut columns = vec![
        RelationalColumn::required("id", RelationalColumnType::Text),
        RelationalColumn::required("value", RelationalColumnType::Text),
    ];
    if version >= 2 {
        columns.push(RelationalColumn {
            name: "extra".into(),
            column_type: RelationalColumnType::Text,
            nullable: true,
            default: None,
        });
    }
    RelationalSchemaPackage {
        namespace: "projection_recovery".into(),
        schema_version: version,
        previous_version: (version > 1).then_some(version - 1),
        package_id: format!("00000000-0000-4000-8000-0000000000{version:02}"),
        schema_hash: String::new(),
        tables: vec![RelationalTable {
            name: "records".into(),
            columns,
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        named_queries: vec![],
    }
}

fn insert_row(storage: &Storage, id: &str, value: &str, extra: Option<&str>) {
    let mut values = json!({"id": id, "value": value});
    if let Some(extra) = extra {
        values["extra"] = json!(extra);
    }
    storage
        .apply_relational_rows(
            "projection_recovery",
            vec![RelationalRowMutation {
                table: "records".into(),
                kind: RelationalMutationKind::Insert,
                values,
                key: None,
            }],
        )
        .unwrap();
}

fn projection_marker(path: &Path) -> String {
    Connection::open(path.join("projection.sqlite"))
        .unwrap()
        .query_row(
            "SELECT value FROM projection_state WHERE key='journal_projection_fingerprint_v1'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn upgraded_relational_schema_reopens_and_preserves_rows() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage.register_relational_schema(schema(1)).unwrap();
    insert_row(&storage, "before-upgrade", "kept", None);
    storage.register_relational_schema(schema(2)).unwrap();
    insert_row(&storage, "after-upgrade", "also-kept", Some("v2"));
    drop(storage);

    // Model the pre-fingerprint projection from the Windows repro: it already
    // contains the v2 registry/data, but has no sync marker. Reopen must replay
    // retained v1→v2 history against a fresh candidate.
    let legacy_projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    legacy_projection
        .execute(
            "DELETE FROM projection_state WHERE key='journal_projection_fingerprint_v1'",
            [],
        )
        .unwrap();
    drop(legacy_projection);

    let reopened = open(dir.path());
    assert_eq!(
        reopened
            .get_relational_schema("projection_recovery")
            .unwrap()
            .unwrap()
            .schema_version,
        2
    );
    drop(reopened);

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let rows: Vec<(String, String, Option<String>)> = projection
        .prepare("SELECT id, value, extra FROM app_projection_recovery__records ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        rows,
        vec![
            (
                "after-upgrade".into(),
                "also-kept".into(),
                Some("v2".into())
            ),
            ("before-upgrade".into(), "kept".into(), None),
        ]
    );
    let marker: String = projection
        .query_row(
            "SELECT value FROM projection_state WHERE key='journal_projection_fingerprint_v1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(marker.len(), 64);
    drop(projection);
    let first_projection_digest =
        Sha256::digest(fs::read(dir.path().join("projection.sqlite")).unwrap());

    let reopened_again = open(dir.path());
    assert_eq!(
        reopened_again
            .get_relational_schema("projection_recovery")
            .unwrap()
            .unwrap()
            .schema_version,
        2
    );
    drop(reopened_again);
    let second_projection_digest =
        Sha256::digest(fs::read(dir.path().join("projection.sqlite")).unwrap());
    assert_eq!(first_projection_digest, second_projection_digest);
    assert_eq!(marker, projection_marker(dir.path()));
}

#[test]
fn journal_append_invalidates_the_projection_sync_marker() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage.register_relational_schema(schema(1)).unwrap();
    insert_row(&storage, "before-append", "kept", None);
    drop(storage);

    let synced = open(dir.path());
    drop(synced);
    let marker_before_append = projection_marker(dir.path());

    let writer = open(dir.path());
    insert_row(&writer, "after-append", "replayed", None);
    drop(writer);
    assert_eq!(projection_marker(dir.path()), marker_before_append);

    let reopened = open(dir.path());
    drop(reopened);
    let marker_after_append = projection_marker(dir.path());
    assert_ne!(marker_before_append, marker_after_append);

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let value: String = projection
        .query_row(
            "SELECT value FROM app_projection_recovery__records WHERE id='after-append'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(value, "replayed");
}

fn append_signed_event_frame(path: &Path, seq: u64, event: Event) -> u64 {
    let secret: [u8; 32] = fs::read(path.join("identity.bin"))
        .unwrap()
        .try_into()
        .unwrap();
    let signing_key = SigningKey::from_bytes(&secret);
    let signature = signing_key.sign(&serde_json::to_vec(&event).unwrap());
    let fingerprint = hex::encode(Sha256::digest(signing_key.verifying_key().as_bytes()));
    let signed = SignedEvent {
        event,
        signature: signature.to_bytes().to_vec(),
        signer_peer_id: fingerprint[..16].into(),
    };
    let payload = serde_json::to_vec(&signed).unwrap();
    let crc = crc32c::crc32c_append(crc32c::crc32c(&seq.to_le_bytes()), &payload);
    let mut frame = Vec::with_capacity(16 + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&seq.to_le_bytes());
    frame.extend_from_slice(&crc.to_le_bytes());
    frame.extend_from_slice(&payload);

    let active_path = path.join("wal").join("active.gwal");
    let original_len = fs::metadata(&active_path).unwrap().len();
    FsOpenOptions::new()
        .append(true)
        .open(active_path)
        .unwrap()
        .write_all(&frame)
        .unwrap();
    original_len
}

fn append_invalid_schema_frame(path: &Path, seq: u64) -> u64 {
    append_signed_event_frame(
        path,
        seq,
        Event::RelationalSchema(RelationalSchemaPackage {
            namespace: "projection_recovery".into(),
            schema_version: 2,
            previous_version: Some(1),
            package_id: "00000000-0000-4000-8000-000000000099".into(),
            schema_hash: String::new(),
            tables: vec![],
            named_queries: vec![],
        }),
    )
}

#[test]
fn failed_rebuild_keeps_the_preexisting_projection_usable() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage.register_relational_schema(schema(1)).unwrap();
    insert_row(&storage, "preserved", "original", None);
    let next_seq = storage.stable_frontier() + 1;
    drop(storage);

    // Establish a marker for the current journal before appending the failing
    // frame. The new bytes must invalidate the marker and enter staged recovery.
    let synced = open(dir.path());
    drop(synced);
    let marker_before_append = projection_marker(dir.path());

    let active_path = dir.path().join("wal").join("active.gwal");
    let original_len = append_invalid_schema_frame(dir.path(), next_seq);
    let error = match Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    }) {
        Ok(storage) => {
            drop(storage);
            panic!("invalid relational schema event should fail projection recovery");
        }
        Err(error) => error,
    };
    assert!(error
        .to_string()
        .contains("relational schema requires a positive version"));

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let version: i64 = projection
        .query_row(
            "SELECT schema_version FROM relational_schema_registry WHERE namespace=?1",
            ["projection_recovery"],
            |row| row.get(0),
        )
        .unwrap();
    let value: String = projection
        .query_row(
            "SELECT value FROM app_projection_recovery__records WHERE id='preserved'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, 1);
    assert_eq!(value, "original");
    drop(projection);
    assert_eq!(projection_marker(dir.path()), marker_before_append);

    FsOpenOptions::new()
        .write(true)
        .open(&active_path)
        .unwrap()
        .set_len(original_len)
        .unwrap();
    let reopened = open(dir.path());
    assert_eq!(
        reopened
            .get_relational_schema("projection_recovery")
            .unwrap()
            .unwrap()
            .schema_version,
        1
    );
}

#[test]
fn failed_incremental_replay_does_not_mutate_the_live_projection() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage.register_relational_schema(schema(1)).unwrap();
    insert_row(&storage, "preserved", "original", None);
    drop(storage);

    let synced = open(dir.path());
    let first_seq = synced.stable_frontier() + 1;
    drop(synced);
    let marker_before_append = projection_marker(dir.path());
    let active_path = dir.path().join("wal").join("active.gwal");

    append_signed_event_frame(
        dir.path(),
        first_seq,
        Event::Node(NodeOutput {
            id: "partially-replayed".into(),
            labels: vec!["TEST".into()],
            props: json!({"value": "must-not-publish"}),
            impact: None,
            embedding: None,
            lang: None,
            valid_from: "2026-01-01T00:00:00Z".into(),
            valid_to: None,
            caused_by: None,
            expires_at: None,
            clock: LogicalClock {
                time: 1,
                peer_id: "projection-replay-test".into(),
            },
            collection: None,
        }),
    );
    let first_event_len = append_signed_event_frame(
        dir.path(),
        first_seq + 1,
        // Keep this old WAL fixture compatible with serde-defaulted fields
        // added to Event after a projection recovery branch was based.
        serde_json::from_value::<Event>(json!({
            "RelationalRows": {
                "namespace": "projection_recovery",
                "schema_version": 99,
                "mutation_id": "invalid-schema-version",
                "payload_hash": "invalid",
                "affected_rows": 0,
                "mutations": [],
            }
        }))
        .expect("deserialize legacy relational-rows event"),
    );

    let error = match Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    }) {
        Ok(storage) => {
            drop(storage);
            panic!("invalid row schema version should fail projection replay");
        }
        Err(error) => error,
    };
    assert!(error
        .to_string()
        .contains("REL_SCHEMA_VERSION_CONFLICT: mutation schema version is not current"));

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let schema_version: i64 = projection
        .query_row(
            "SELECT schema_version FROM relational_schema_registry WHERE namespace=?1",
            ["projection_recovery"],
            |row| row.get(0),
        )
        .unwrap();
    let preserved: String = projection
        .query_row(
            "SELECT value FROM app_projection_recovery__records WHERE id='preserved'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let partial_node_count: i64 = projection
        .query_row(
            "SELECT COUNT(*) FROM node_versions WHERE id='partially-replayed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(schema_version, 1);
    assert_eq!(preserved, "original");
    assert_eq!(partial_node_count, 0);
    drop(projection);
    assert_eq!(projection_marker(dir.path()), marker_before_append);

    FsOpenOptions::new()
        .write(true)
        .open(&active_path)
        .unwrap()
        .set_len(first_event_len)
        .unwrap();
    let reopened = open(dir.path());
    assert_eq!(
        reopened
            .get_relational_schema("projection_recovery")
            .unwrap()
            .unwrap()
            .schema_version,
        1
    );
    drop(reopened);
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let replayed_node_count: i64 = projection
        .query_row(
            "SELECT COUNT(*) FROM node_versions WHERE id='partially-replayed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(replayed_node_count, 1);
}

#[test]
fn journal_compaction_invalidates_the_projection_sync_marker() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage.register_relational_schema(schema(1)).unwrap();
    insert_row(&storage, "after-compaction", "preserved", None);
    drop(storage);

    let synced = open(dir.path());
    drop(synced);
    let marker_before_compaction = projection_marker(dir.path());

    let storage = open(dir.path());
    storage.compact().unwrap();
    drop(storage);

    let reopened = open(dir.path());
    assert_eq!(
        reopened
            .get_relational_schema("projection_recovery")
            .unwrap()
            .unwrap()
            .schema_version,
        1
    );
    drop(reopened);
    let marker_after_compaction = projection_marker(dir.path());
    assert_ne!(marker_before_compaction, marker_after_compaction);

    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    let value: String = projection
        .query_row(
            "SELECT value FROM app_projection_recovery__records WHERE id='after-compaction'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(value, "preserved");
}

#[test]
fn abandoned_sqlite_backup_rolls_back_after_copying_a_page() {
    let dir = TempDir::new().unwrap();
    let mut destination = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    destination
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE projection_state(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO projection_state VALUES('schema_version', '1');
             CREATE TABLE props(node_u32 INTEGER PRIMARY KEY, payload TEXT NOT NULL);
             INSERT INTO props VALUES(7, '{\"kept\":true}');",
        )
        .unwrap();

    let source = Connection::open_in_memory().unwrap();
    source
        .execute_batch(
            "PRAGMA page_size=4096;
             CREATE TABLE replacement(value TEXT NOT NULL);",
        )
        .unwrap();
    {
        let mut insert = source
            .prepare("INSERT INTO replacement(value) VALUES(?1)")
            .unwrap();
        for index in 0..2_000 {
            insert
                .execute([format!("replacement-{index}-{}", "x".repeat(120))])
                .unwrap();
        }
    }

    let backup = rusqlite::backup::Backup::new(&source, &mut destination).unwrap();
    assert_eq!(backup.step(1).unwrap(), StepResult::More);
    drop(backup);

    let version: String = destination
        .query_row(
            "SELECT value FROM projection_state WHERE key='schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let payload: String = destination
        .query_row("SELECT payload FROM props WHERE node_u32=7", [], |row| {
            row.get(0)
        })
        .unwrap();
    let replacement_exists: bool = destination
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='replacement')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, "1");
    assert_eq!(payload, "{\"kept\":true}");
    assert!(!replacement_exists);
}
