use genesis_block_native::{
    OpenOptions, RelationalColumn, RelationalColumnType, RelationalMutationBatch,
    RelationalMutationKind, RelationalRowMutation, RelationalSchemaPackage, RelationalTable,
    Storage,
};
use rusqlite::Connection;
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;

fn open(path: &Path, retention: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some(retention.into()),
    })
    .unwrap()
}

fn schema(version: u32) -> RelationalSchemaPackage {
    let mut columns = vec![
        RelationalColumn::required("id", RelationalColumnType::Text),
        RelationalColumn::required("value", RelationalColumnType::Text),
    ];
    for (at, name) in [(2, "extra"), (3, "another")] {
        if version >= at {
            columns.push(RelationalColumn {
                name: name.into(),
                column_type: RelationalColumnType::Text,
                nullable: true,
                default: None,
            });
        }
    }
    RelationalSchemaPackage {
        namespace: "replay_upgrade".into(),
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

fn write_row(db: &Storage, version: u32) {
    db.apply_relational_batch(RelationalMutationBatch {
        mutation_id: "00000000-0000-4000-8000-000000000100".into(),
        namespace: "replay_upgrade".into(),
        schema_version: version,
        operations: vec![RelationalRowMutation {
            table: "records".into(),
            kind: RelationalMutationKind::Insert,
            values: json!({"id":"one", "value":"kept"}),
            key: None,
        }],
    })
    .unwrap();
}

fn check_rows(path: &Path, retention: &str) {
    for _ in 0..2 {
        let db = open(path, retention);
        assert_eq!(
            db.get_relational_schema("replay_upgrade")
                .unwrap()
                .unwrap()
                .schema_version,
            3
        );
        let conn = Connection::open(path.join("projection.sqlite")).unwrap();
        let row: (String, String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT id, value, extra, another FROM app_replay_upgrade__records",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(row, ("one".into(), "kept".into(), None, None));
        drop(conn);
        drop(db);
    }
}

fn scenario(retention: &str, later: bool, after: bool) {
    let dir = TempDir::new().unwrap();
    let mut db = open(dir.path(), retention);
    db.register_relational_schema(schema(1)).unwrap();
    if !after {
        write_row(&db, 1);
    }
    if later {
        drop(db);
        db = open(dir.path(), retention);
    }
    db.register_relational_schema(schema(2)).unwrap();
    db.register_relational_schema(schema(3)).unwrap();
    if after {
        write_row(&db, 3);
    }
    // Inspect the actual journal following a checkpoint, before a potentially failing reopen.
    db.save_state().unwrap();
    let events = db.events_since_seq(0);
    eprintln!(
        "journal {retention}: {}",
        serde_json::to_string_pretty(&events).unwrap()
    );
    drop(db);
    check_rows(dir.path(), retention);
}

#[test]
fn frontier_same_session_upgrade() {
    scenario("frontier_only", false, false);
}
#[test]
fn frontier_later_session_upgrade() {
    scenario("frontier_only", true, false);
}
#[test]
fn full_same_session_upgrade() {
    scenario("full", false, false);
}
#[test]
fn full_later_session_upgrade() {
    scenario("full", true, false);
}
#[test]
fn budget_same_session_upgrade() {
    scenario("budget:1", false, false);
}
#[test]
fn frontier_rows_after_upgrade_control() {
    scenario("frontier_only", false, true);
}

#[test]
fn live_batch_rejects_invalid_schema_version() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path(), "full");
    db.register_relational_schema(schema(1)).unwrap();
    let error = db
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: "00000000-0000-4000-8000-000000000099".into(),
            namespace: "replay_upgrade".into(),
            schema_version: 99,
            operations: vec![RelationalRowMutation {
                table: "records".into(),
                kind: RelationalMutationKind::Insert,
                values: json!({"id":"bad", "value":"bad"}),
                key: None,
            }],
        })
        .unwrap_err();
    assert!(error.to_string().contains("REL_SCHEMA_VERSION_CONFLICT"));
}

#[test]
fn replay_rejects_invalid_schema_version() {
    use ed25519_dalek::{Signer, SigningKey};
    use genesis_block_native::{Event, SignedEvent};
    use sha2::{Digest, Sha256};
    use std::io::Write;
    let dir = TempDir::new().unwrap();
    let db = open(dir.path(), "full");
    db.register_relational_schema(schema(1)).unwrap();
    let seq = db.stable_frontier() + 1;
    drop(db);
    let secret: [u8; 32] = std::fs::read(dir.path().join("identity.bin"))
        .unwrap()
        .try_into()
        .unwrap();
    let key = SigningKey::from_bytes(&secret);
    let event: Event = serde_json::from_value(json!({"RelationalRows": {"namespace": "replay_upgrade", "schema_version": 99, "mutation_id": "invalid-schema-version", "payload_hash": "invalid", "affected_rows": 0, "mutations": []}})).unwrap();
    let signature = key.sign(&serde_json::to_vec(&event).unwrap());
    let fingerprint = hex::encode(Sha256::digest(key.verifying_key().as_bytes()));
    let payload = serde_json::to_vec(&SignedEvent {
        event,
        signature: signature.to_bytes().to_vec(),
        signer_peer_id: fingerprint[..16].into(),
    })
    .unwrap();
    let crc = crc32c::crc32c_append(crc32c::crc32c(&seq.to_le_bytes()), &payload);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(dir.path().join("wal/active.gwal"))
        .unwrap();
    file.write_all(&(payload.len() as u32).to_le_bytes())
        .unwrap();
    file.write_all(&seq.to_le_bytes()).unwrap();
    file.write_all(&crc.to_le_bytes()).unwrap();
    file.write_all(&payload).unwrap();
    drop(file);
    let result = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    });
    match result {
        Ok(_) => panic!("version 99 must fail replay"),
        Err(error) => assert!(error.to_string().contains("REL_SCHEMA_VERSION_CONFLICT")),
    }
}
