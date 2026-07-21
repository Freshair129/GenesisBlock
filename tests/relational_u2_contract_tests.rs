use genesis_block_native::{
    NamedQueryDefinition, NamedQueryRequest, OpenOptions, RelationalColumn, RelationalColumnType,
    RelationalFilter, RelationalForeignKey, RelationalJoin, RelationalJoinKind,
    RelationalMutationBatch, RelationalMutationKind, RelationalQuery, RelationalQueryParameter,
    RelationalRowMutation, RelationalSchemaPackage, RelationalTable, Storage,
};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Barrier};

const PACKAGE_ID: &str = "00000000-0000-4000-8000-000000000101";
const MUTATION_ID: &str = "00000000-0000-4000-8000-000000000201";

fn fresh(name: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&path).exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
    })
    .unwrap()
}

fn schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "appdata".to_string(),
        schema_version: 1,
        previous_version: None,
        package_id: PACKAGE_ID.to_string(),
        schema_hash: String::new(),
        tables: vec![
            RelationalTable {
                name: "projects".to_string(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::EntityId),
                    RelationalColumn::required("name", RelationalColumnType::Text),
                ],
                primary_key: vec!["id".to_string()],
                foreign_keys: vec![],
                indexes: vec![],
            },
            RelationalTable {
                name: "notes".to_string(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::EntityId),
                    RelationalColumn {
                        name: "project_id".to_string(),
                        column_type: RelationalColumnType::EntityId,
                        nullable: true,
                        default: None,
                    },
                    RelationalColumn::required("title", RelationalColumnType::Text),
                    RelationalColumn {
                        name: "active".to_string(),
                        column_type: RelationalColumnType::Boolean,
                        nullable: false,
                        default: Some(json!(true)),
                    },
                    RelationalColumn {
                        name: "metadata".to_string(),
                        column_type: RelationalColumnType::Json,
                        nullable: false,
                        default: Some(json!({})),
                    },
                ],
                primary_key: vec!["id".to_string()],
                foreign_keys: vec![RelationalForeignKey {
                    columns: vec!["project_id".to_string()],
                    referenced_table: "projects".to_string(),
                    referenced_columns: vec!["id".to_string()],
                }],
                indexes: vec![],
            },
        ],
        named_queries: vec![NamedQueryDefinition {
            name: "note_with_project".to_string(),
            parameters: vec![RelationalQueryParameter {
                name: "note_id".to_string(),
                column_type: RelationalColumnType::EntityId,
            }],
            query: RelationalQuery {
                namespace: "appdata".to_string(),
                table: "notes".to_string(),
                columns: vec![
                    "notes.title".to_string(),
                    "notes.active".to_string(),
                    "notes.metadata".to_string(),
                    "projects.name".to_string(),
                ],
                joins: vec![RelationalJoin {
                    table: "projects".to_string(),
                    left_column: "notes.project_id".to_string(),
                    right_column: "projects.id".to_string(),
                    kind: RelationalJoinKind::Left,
                }],
                filters: vec![RelationalFilter::equal(
                    "notes.id",
                    json!({"$param": "note_id"}),
                )],
                limit: None,
            },
            default_limit: 10,
            max_limit: 50,
        }],
    }
}

fn upsert_note(title: &str) -> RelationalMutationBatch {
    RelationalMutationBatch {
        mutation_id: MUTATION_ID.to_string(),
        namespace: "appdata".to_string(),
        schema_version: 1,
        operations: vec![RelationalRowMutation {
            table: "notes".to_string(),
            kind: RelationalMutationKind::Upsert,
            values: json!({"id": "note-1", "project_id": null, "title": title}),
            key: None,
        }],
    }
}

#[test]
fn schema_identity_hash_and_exact_version_chain_are_enforced() {
    let path = fresh("relational_u2_schema_contract");
    let storage = open(&path);
    storage.register_relational_schema(schema()).unwrap();

    let registered = storage
        .get_relational_schema("appdata")
        .unwrap()
        .expect("registered schema");
    assert_eq!(registered.package_id, PACKAGE_ID);
    assert_eq!(registered.schema_hash.len(), 64);
    assert_eq!(storage.register_relational_schema(registered).unwrap(), 1);

    let mut upgraded = schema();
    upgraded.schema_version = 2;
    upgraded.previous_version = Some(1);
    upgraded.package_id = "00000000-0000-4000-8000-000000000102".to_string();
    upgraded.tables[1].columns.push(RelationalColumn {
        name: "archived_at".to_string(),
        column_type: RelationalColumnType::Timestamp,
        nullable: true,
        default: None,
    });
    assert_eq!(storage.register_relational_schema(upgraded).unwrap(), 2);

    let mut skipped = schema();
    skipped.schema_version = 4;
    skipped.previous_version = Some(2);
    skipped.package_id = "00000000-0000-4000-8000-000000000103".to_string();
    assert!(storage.register_relational_schema(skipped).is_err());

    let mut tampered = schema();
    tampered.schema_hash = "not-the-package-hash".to_string();
    assert!(storage.register_relational_schema(tampered).is_err());

    let mut reserved = schema();
    reserved.namespace = "sqlite_app".to_string();
    assert!(storage.register_relational_schema(reserved).is_err());
}

#[test]
fn mutation_identity_is_idempotent_and_conflicting_reuse_fails() {
    let path = fresh("relational_u2_mutation_identity");
    let storage = open(&path);
    storage.register_relational_schema(schema()).unwrap();

    let first = storage
        .apply_relational_batch(upsert_note("Original"))
        .unwrap();
    let retry = storage
        .apply_relational_batch(upsert_note("Original"))
        .unwrap();
    assert_eq!(first, retry);
    assert_eq!(first.affected_rows, 1);

    let error = storage
        .apply_relational_batch(upsert_note("Conflicting payload"))
        .unwrap_err();
    assert!(error.to_string().contains("REL_MUTATION_CONFLICT"));
}

#[test]
fn typed_insert_update_and_named_left_join_are_bounded() {
    let path = fresh("relational_u2_typed_named_query");
    let storage = open(&path);
    storage.register_relational_schema(schema()).unwrap();

    let insert = RelationalRowMutation {
        table: "notes".to_string(),
        kind: RelationalMutationKind::Insert,
        values: json!({"id": "note-1", "project_id": null, "title": "Draft"}),
        key: None,
    };
    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: "00000000-0000-4000-8000-000000000202".to_string(),
            namespace: "appdata".to_string(),
            schema_version: 1,
            operations: vec![insert.clone()],
        })
        .unwrap();
    assert!(storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: "00000000-0000-4000-8000-000000000203".to_string(),
            namespace: "appdata".to_string(),
            schema_version: 1,
            operations: vec![insert],
        })
        .is_err());
    let wal = fs::read_to_string(Path::new(&path).join("genesis-graph.wal")).unwrap();
    assert!(!wal.contains("00000000-0000-4000-8000-000000000203"));

    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: "00000000-0000-4000-8000-000000000204".to_string(),
            namespace: "appdata".to_string(),
            schema_version: 1,
            operations: vec![RelationalRowMutation {
                table: "notes".to_string(),
                kind: RelationalMutationKind::Update,
                values: json!({"title": "Published"}),
                key: Some(json!({"id": "note-1"})),
            }],
        })
        .unwrap();

    let request = NamedQueryRequest {
        namespace: "appdata".to_string(),
        schema_version: 1,
        query_name: "note_with_project".to_string(),
        parameters: json!({"note_id": "note-1"}),
        limit: Some(1),
    };
    assert_eq!(
        storage.execute_named_query(request.clone()).unwrap(),
        vec![json!({
            "notes.title": "Published",
            "notes.active": true,
            "notes.metadata": {},
            "projects.name": null
        })]
    );

    let mut wrong_type = request.clone();
    wrong_type.parameters = json!({"note_id": 7});
    assert!(storage.execute_named_query(wrong_type).is_err());
    let mut excessive = request;
    excessive.limit = Some(51);
    assert!(storage.execute_named_query(excessive).is_err());
}

#[test]
fn concurrent_mutation_identity_commits_only_one_payload() {
    let path = fresh("relational_u2_concurrent_identity");
    let storage = Arc::new(open(&path));
    storage.register_relational_schema(schema()).unwrap();
    let barrier = Arc::new(Barrier::new(3));

    let handles = ["First", "Second"].map(|title| {
        let storage = Arc::clone(&storage);
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            storage.apply_relational_batch(upsert_note(title))
        })
    });
    barrier.wait();
    let results = handles.map(|handle| handle.join().unwrap());
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);

    let wal = fs::read_to_string(Path::new(&path).join("genesis-graph.wal")).unwrap();
    assert_eq!(wal.matches(MUTATION_ID).count(), 1);
}

#[test]
fn compacted_wal_rebuilds_rows_schema_and_mutation_identity() {
    let path = fresh("relational_u2_compacted_replay");
    let batch = upsert_note("Recovered");
    let original_result;
    {
        let storage = open(&path);
        storage.register_relational_schema(schema()).unwrap();
        original_result = storage.apply_relational_batch(batch.clone()).unwrap();
        storage.compact().unwrap();
    }

    fs::remove_file(Path::new(&path).join("projection.sqlite")).unwrap();
    let storage = open(&path);
    assert_eq!(
        storage.apply_relational_batch(batch).unwrap(),
        original_result
    );
    assert!(storage.get_relational_schema("appdata").unwrap().is_some());
    assert_eq!(
        storage
            .execute_named_query(NamedQueryRequest {
                namespace: "appdata".to_string(),
                schema_version: 1,
                query_name: "note_with_project".to_string(),
                parameters: json!({"note_id": "note-1"}),
                limit: None,
            })
            .unwrap(),
        vec![json!({
            "notes.title": "Recovered",
            "notes.active": true,
            "notes.metadata": {},
            "projects.name": null
        })]
    );
}
