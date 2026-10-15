use super::*;

fn fixture() -> (tempfile::TempDir, Storage) {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: directory.path().display().to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some("full".into()),
    })
    .unwrap();
    storage
        .register_relational_schema(RelationalSchemaPackage {
            namespace: "preserve".into(),
            schema_version: 1,
            previous_version: None,
            package_id: "00000000-0000-4000-8000-000000000001".into(),
            schema_hash: String::new(),
            tables: vec![RelationalTable {
                name: "notes".into(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::Text),
                    RelationalColumn::required("text", RelationalColumnType::Text),
                ],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            }],
            named_queries: vec![],
        })
        .unwrap();
    storage
        .apply_relational_rows(
            "preserve",
            vec![RelationalRowMutation {
                table: "notes".into(),
                kind: RelationalMutationKind::Upsert,
                values: serde_json::json!({"id":"existing", "text":"ห้ามข้อมูลหาย"}),
                key: None,
            }],
        )
        .unwrap();
    storage.save_state().unwrap();
    (directory, storage)
}

fn rows(storage: &Storage) -> Vec<Value> {
    storage
        .query_relational(RelationalQuery {
            namespace: "preserve".into(),
            table: "notes".into(),
            columns: vec!["notes.id".into(), "notes.text".into()],
            joins: vec![],
            filters: vec![],
            limit: Some(10),
            offset: None,
        })
        .unwrap()
}

#[test]
fn failed_replay_keeps_original_projection_and_connection() {
    let (_directory, storage) = fixture();
    let baseline = rows(&storage);
    let registered = storage.get_relational_schema("preserve").unwrap();
    let mut invalid = registered.clone().unwrap();
    invalid.schema_version = 3;
    invalid.previous_version = Some(2);
    invalid.schema_hash.clear();
    // Deliberate invalid journal entry; public registration correctly rejects it.
    storage
        .append_wal_event(&Event::RelationalSchema(invalid))
        .unwrap();
    let error = storage.projection_rebuild_from_wal().unwrap_err();
    assert!(error.to_string().contains("REL_SCHEMA_VERSION_CONFLICT"));
    assert_eq!(
        storage.get_relational_schema("preserve").unwrap(),
        registered
    );
    assert_eq!(rows(&storage), baseline);
}

#[test]
fn busy_publication_keeps_original_projection_and_connection() {
    let (directory, storage) = fixture();
    let baseline = rows(&storage);
    let other = Connection::open(directory.path().join("projection.sqlite")).unwrap();
    other.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let result = storage.projection_rebuild_from_wal();
    other.execute_batch("ROLLBACK;").unwrap();
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("PROJECTION_REBUILD_BUSY"));
    assert_eq!(rows(&storage), baseline);
    storage.projection_rebuild_from_wal().unwrap();
    assert_eq!(rows(&storage), baseline);
    storage.save_state().unwrap();
    drop(storage);
    drop(other);
    let reopened = Storage::open(OpenOptions {
        path: directory.path().display().to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some("full".into()),
    })
    .unwrap();
    assert_eq!(rows(&reopened), baseline);
}
