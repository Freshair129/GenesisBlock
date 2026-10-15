use genesis_block_native::{
    OpenOptions, RelationalColumn, RelationalColumnType, RelationalMutationKind, RelationalQuery,
    RelationalRowMutation, RelationalSchemaPackage, RelationalTable, Storage,
};
use serde_json::{json, Value};

fn open(path: &std::path::Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.display().to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some("full".into()),
    })
    .expect("reopen retained schema history")
}

fn schema(version: u32) -> RelationalSchemaPackage {
    let mut columns = vec![
        RelationalColumn::required("id", RelationalColumnType::Text),
        RelationalColumn::required("text", RelationalColumnType::Text),
    ];
    if version == 2 {
        columns.push(RelationalColumn {
            name: "annotation".into(),
            column_type: RelationalColumnType::Text,
            nullable: true,
            default: None,
        });
    }
    RelationalSchemaPackage {
        namespace: "reopen".into(),
        schema_version: version,
        previous_version: (version > 1).then_some(version - 1),
        package_id: format!("00000000-0000-4000-8000-{version:012}"),
        schema_hash: String::new(),
        tables: vec![RelationalTable {
            name: "notes".into(),
            columns,
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        named_queries: vec![],
    }
}

fn write(storage: &Storage, values: Value) {
    storage
        .apply_relational_rows(
            "reopen",
            vec![RelationalRowMutation {
                table: "notes".into(),
                kind: RelationalMutationKind::Upsert,
                values,
                key: None,
            }],
        )
        .unwrap();
}

fn rows(storage: &Storage) -> Vec<Value> {
    storage
        .query_relational(RelationalQuery {
            namespace: "reopen".into(),
            table: "notes".into(),
            columns: vec![
                "notes.id".into(),
                "notes.text".into(),
                "notes.annotation".into(),
            ],
            joins: vec![],
            filters: vec![],
            limit: Some(100),
            offset: Some(0),
        })
        .unwrap()
}

#[test]
fn upgraded_projection_reopens_without_losing_old_rows_or_new_edits() {
    let directory = tempfile::tempdir().unwrap();
    let storage = open(directory.path());
    storage.register_relational_schema(schema(1)).unwrap();
    write(
        &storage,
        json!({"id":"before", "text":"ข้อความเดิมก่อนอัปเกรด"}),
    );
    storage.register_relational_schema(schema(2)).unwrap();
    write(
        &storage,
        json!({"id":"after", "text":"after upgrade", "annotation":"keep me"}),
    );
    let baseline = rows(&storage);
    storage.save_state().unwrap();
    drop(storage);

    let storage = open(directory.path());
    assert_eq!(rows(&storage), baseline);
    assert_eq!(storage.register_relational_schema(schema(2)).unwrap(), 2);
    write(
        &storage,
        json!({"id":"after", "text":"edited after reopen", "annotation":"keep me"}),
    );
    let edited = rows(&storage);
    storage.save_state().unwrap();
    drop(storage);

    for _ in 0..2 {
        let storage = open(directory.path());
        assert_eq!(rows(&storage), edited);
        assert_eq!(
            storage
                .get_relational_schema("reopen")
                .unwrap()
                .unwrap()
                .schema_version,
            2
        );
        storage.save_state().unwrap();
    }
}
