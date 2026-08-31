use genesis_block_native::{
    OpenOptions, RelationalColumn, RelationalColumnType, RelationalFilter, RelationalForeignKey,
    RelationalJoin, RelationalJoinKind, RelationalMutationKind, RelationalQuery,
    RelationalRowMutation, RelationalSchemaPackage, RelationalTable, Storage,
};
use serde_json::json;
use std::fs;
use std::path::Path;

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
        retention: None,
    })
    .unwrap()
}

fn fung_schema() -> RelationalSchemaPackage {
    RelationalSchemaPackage {
        namespace: "fung".to_string(),
        schema_version: 1,
        previous_version: None,
        package_id: "00000000-0000-4000-8000-000000000001".to_string(),
        schema_hash: String::new(),
        tables: vec![
            RelationalTable {
                name: "projects".to_string(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::Text),
                    RelationalColumn::required("name", RelationalColumnType::Text),
                ],
                primary_key: vec!["id".to_string()],
                foreign_keys: vec![],
                indexes: vec![],
            },
            RelationalTable {
                name: "notes".to_string(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::Text),
                    RelationalColumn::required("project_id", RelationalColumnType::Text),
                    RelationalColumn::required("title", RelationalColumnType::Text),
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
        named_queries: vec![],
    }
}

#[test]
fn relational_schema_rows_and_join_use_one_genesis_handle() {
    let path = fresh("relational_u2_join");
    let storage = open(&path);

    assert_eq!(
        storage.register_relational_schema(fung_schema()).unwrap(),
        1
    );
    storage
        .apply_relational_rows(
            "fung",
            vec![
                RelationalRowMutation {
                    table: "projects".to_string(),
                    kind: RelationalMutationKind::Upsert,
                    values: json!({"id": "project-1", "name": "FUNG Mobile"}),
                    key: None,
                },
                RelationalRowMutation {
                    table: "notes".to_string(),
                    kind: RelationalMutationKind::Upsert,
                    values: json!({"id": "note-1", "project_id": "project-1", "title": "Genesis boundary"}),
                    key: None,
                },
            ],
        )
        .unwrap();

    let rows = storage
        .query_relational(RelationalQuery {
            namespace: "fung".to_string(),
            table: "notes".to_string(),
            columns: vec!["notes.title".to_string(), "projects.name".to_string()],
            joins: vec![RelationalJoin {
                table: "projects".to_string(),
                left_column: "notes.project_id".to_string(),
                right_column: "projects.id".to_string(),
                kind: RelationalJoinKind::Inner,
            }],
            filters: vec![RelationalFilter::equal("notes.id", json!("note-1"))],
            limit: Some(10),
            offset: None,
        })
        .unwrap();

    assert_eq!(
        rows,
        vec![json!({"notes.title": "Genesis boundary", "projects.name": "FUNG Mobile"})]
    );
}

#[test]
fn relational_projection_rebuilds_from_genesis_wal() {
    let path = fresh("relational_u2_replay");
    {
        let storage = open(&path);
        storage.register_relational_schema(fung_schema()).unwrap();
        storage
            .apply_relational_rows(
                "fung",
                vec![RelationalRowMutation {
                    table: "projects".to_string(),
                    kind: RelationalMutationKind::Upsert,
                    values: json!({"id": "project-1", "name": "Recovered"}),
                    key: None,
                }],
            )
            .unwrap();
    }
    fs::remove_file(Path::new(&path).join("projection.sqlite")).unwrap();

    let storage = open(&path);
    let rows = storage
        .query_relational(RelationalQuery {
            namespace: "fung".to_string(),
            table: "projects".to_string(),
            columns: vec!["projects.name".to_string()],
            joins: vec![],
            filters: vec![RelationalFilter::equal("projects.id", json!("project-1"))],
            limit: Some(1),
            offset: None,
        })
        .unwrap();
    assert_eq!(rows, vec![json!({"projects.name": "Recovered"})]);
}

#[test]
fn relational_schema_rejects_downgrade_and_namespace_escape() {
    let path = fresh("relational_u2_guards");
    let storage = open(&path);
    storage.register_relational_schema(fung_schema()).unwrap();

    let mut downgrade = fung_schema();
    downgrade.schema_version = 0;
    assert!(storage.register_relational_schema(downgrade).is_err());

    let mut escape = fung_schema();
    escape.namespace = "fung; DROP TABLE props".to_string();
    assert!(storage.register_relational_schema(escape).is_err());
}

#[test]
fn relational_batch_validation_prevents_partial_write() {
    let path = fresh("relational_u2_atomic_validation");
    let storage = open(&path);
    storage.register_relational_schema(fung_schema()).unwrap();

    let result = storage.apply_relational_rows(
        "fung",
        vec![
            RelationalRowMutation {
                table: "projects".to_string(),
                kind: RelationalMutationKind::Upsert,
                values: json!({"id": "project-1", "name": "Must roll back"}),
                key: None,
            },
            RelationalRowMutation {
                table: "missing".to_string(),
                kind: RelationalMutationKind::Upsert,
                values: json!({"id": "bad"}),
                key: None,
            },
        ],
    );
    assert!(result.is_err());

    let rows = storage
        .query_relational(RelationalQuery {
            namespace: "fung".to_string(),
            table: "projects".to_string(),
            columns: vec!["projects.id".to_string()],
            joins: vec![],
            filters: vec![],
            limit: Some(10),
            offset: None,
        })
        .unwrap();
    assert!(rows.is_empty());
}

#[test]
fn relational_offset_pages_partition_the_result_set() {
    let path = fresh("relational_u2_offset");
    let storage = open(&path);
    storage.register_relational_schema(fung_schema()).unwrap();

    let mut mutations = vec![RelationalRowMutation {
        table: "projects".to_string(),
        kind: RelationalMutationKind::Upsert,
        values: json!({"id": "project-1", "name": "FUNG Mobile"}),
        key: None,
    }];
    for index in 0..25 {
        mutations.push(RelationalRowMutation {
            table: "notes".to_string(),
            kind: RelationalMutationKind::Upsert,
            values: json!({
                "id": format!("note-{index:03}"),
                "project_id": "project-1",
                "title": format!("segment {index}"),
            }),
            key: None,
        });
    }
    storage.apply_relational_rows("fung", mutations).unwrap();

    let page = |offset: u32| {
        storage
            .query_relational(RelationalQuery {
                namespace: "fung".to_string(),
                table: "notes".to_string(),
                columns: vec!["notes.id".to_string()],
                joins: vec![],
                filters: vec![RelationalFilter::equal(
                    "notes.project_id",
                    json!("project-1"),
                )],
                limit: Some(10),
                offset: Some(offset),
            })
            .unwrap()
            .into_iter()
            .map(|row| row["notes.id"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };

    // Three consecutive pages partition all 25 rows in primary-key order,
    // with no duplicates and no gaps.
    let mut collected = page(0);
    assert_eq!(collected.len(), 10);
    collected.extend(page(10));
    collected.extend(page(20));
    assert_eq!(
        collected,
        (0..25)
            .map(|index| format!("note-{index:03}"))
            .collect::<Vec<_>>()
    );

    // An offset past the result set is an empty page, not an error.
    assert!(page(25).is_empty());

    // The pre-offset shape still works untouched.
    let unpaged = storage
        .query_relational(RelationalQuery {
            namespace: "fung".to_string(),
            table: "notes".to_string(),
            columns: vec!["notes.id".to_string()],
            joins: vec![],
            filters: vec![],
            limit: Some(1000),
            offset: None,
        })
        .unwrap();
    assert_eq!(unpaged.len(), 25);
}
