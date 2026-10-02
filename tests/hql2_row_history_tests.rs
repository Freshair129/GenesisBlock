use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2, QueryResultV2},
    uee_v2::QueryRequestV2,
    AccessContext, OpenOptions, RelationalColumn, RelationalColumnType, RelationalMutationBatch,
    RelationalMutationKind, RelationalRowMutation, RelationalSchemaPackage, RelationalTable,
    Storage,
};
use serde_json::{json, Value};
use std::path::Path;
use tempfile::TempDir;
use uuid::Uuid;

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

fn actor() -> AccessContext {
    AccessContext {
        principal: "row-history-reader".into(),
        namespace: "default".into(),
    }
}

fn execute(
    storage: &Storage,
    request: Value,
) -> Result<QueryResultV2, genesis_block_native::query::hql2::QueryErrorV2> {
    let request: QueryRequestV2 = serde_json::from_value(request).unwrap();
    match storage.query_v2(actor(), request)? {
        QueryOutcomeV2::Rows(result) => Ok(result),
        QueryOutcomeV2::Plan(_) => panic!("query must return rows"),
    }
}

fn field(alias: &str, path: &[&str]) -> Value {
    json!({"field":{"alias":alias,"path":path}})
}

fn project_field(alias: &str, expression: Value) -> Value {
    json!({"as":alias,"expression":expression})
}

#[test]
fn hql_and_ir_history_and_change_scans_read_exact_row_revisions() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .register_relational_schema(RelationalSchemaPackage {
            namespace: "default".into(),
            schema_version: 1,
            previous_version: None,
            package_id: Uuid::new_v4().to_string(),
            schema_hash: String::new(),
            tables: vec![RelationalTable {
                name: "records".into(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::Text),
                    RelationalColumn::required("title", RelationalColumnType::Text),
                ],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            }],
            named_queries: vec![],
        })
        .unwrap();
    let after = storage.stable_frontier();
    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: Uuid::new_v4().to_string(),
            namespace: "default".into(),
            schema_version: 1,
            operations: vec![RelationalRowMutation {
                table: "records".into(),
                kind: RelationalMutationKind::Insert,
                values: json!({"id":"row:one","title":"original"}),
                key: None,
            }],
        })
        .unwrap();
    let row_id: String = rusqlite::Connection::open(dir.path().join("projection.sqlite"))
        .unwrap()
        .query_row(
            "SELECT record_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='row' AND schema_ref='records'
             ORDER BY tx_from, revision_id LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: Uuid::new_v4().to_string(),
            namespace: "default".into(),
            schema_version: 1,
            operations: vec![RelationalRowMutation {
                table: "records".into(),
                kind: RelationalMutationKind::Update,
                values: json!({"title":"replacement"}),
                key: Some(json!({"id":"row:one"})),
            }],
        })
        .unwrap();

    let history_hql = execute(
        &storage,
        json!({
            "contract_version":"genesis.api.v2",
            "request_id":"row-history-hql",
            "namespace":"default",
            "language_version":"hql.v2",
            "hql":format!(
                "USE default HISTORY ROW \"{row_id}\" AS h |> RETURN h.operation AS operation, h.revision_id AS revision, prop(h.subject, \"title\") AS title"
            ),
            "params":{}
        }),
    )
    .unwrap();
    let history_ir = execute(
        &storage,
        json!({
            "contract_version":"genesis.api.v2",
            "request_id":"row-history-ir",
            "namespace":"default",
            "ir":{
                "contract_version":"query-ir.v2",
                "nodes":[
                    {"id":"history","op":"HistoryScan","inputs":[],"config":{"kind":"row","id":{"literal":row_id,"type":"Utf8"},"as":"h"}},
                    {"id":"project","op":"Project","inputs":["history"],"config":{"fields":[
                        project_field("operation", field("h", &["operation"])),
                        project_field("revision", field("h", &["revision_id"])),
                        project_field("title", json!({"call":"prop","args":[field("h", &["subject"]),{"literal":"title","type":"Utf8"}]}))
                    ]}}
                ],
                "root":"project",
                "parameter_types":{}
            },
            "params":{}
        }),
    )
    .unwrap();

    assert_eq!(history_hql.columns, history_ir.columns);
    assert_eq!(history_hql.rows, history_ir.rows);
    assert_eq!(history_hql.rows.len(), 2);
    assert_eq!(
        history_hql.rows[0]["title"],
        QueryValueV2::Json(json!("original"))
    );
    assert_eq!(
        history_hql.rows[1]["title"],
        QueryValueV2::Json(json!("replacement"))
    );
    assert_ne!(
        history_hql.rows[0]["revision"],
        history_hql.rows[1]["revision"]
    );

    let changes_hql = execute(
        &storage,
        json!({
            "contract_version":"genesis.api.v2",
            "request_id":"row-changes-hql",
            "namespace":"default",
            "language_version":"hql.v2",
            "hql":format!(
                "USE default CHANGES SINCE {after} AS c |> ORDER BY c.sequence ASC NULLS LAST |> RETURN c.operation AS operation, c.subject AS subject"
            ),
            "params":{}
        }),
    )
    .unwrap();
    let changes_ir = execute(
        &storage,
        json!({
            "contract_version":"genesis.api.v2",
            "request_id":"row-changes-ir",
            "namespace":"default",
            "ir":{
                "contract_version":"query-ir.v2",
                "nodes":[
                    {"id":"changes","op":"ChangeScan","inputs":[],"config":{"after_seq":after.to_string(),"as":"c"}},
                    {"id":"project","op":"Project","inputs":["changes"],"config":{"fields":[
                        project_field("operation", field("c", &["operation"])),
                        project_field("subject", field("c", &["subject"]))
                    ]}}
                ],
                "root":"project",
                "parameter_types":{}
            },
            "params":{}
        }),
    )
    .unwrap();

    assert_eq!(changes_hql.columns, changes_ir.columns);
    assert_eq!(changes_hql.rows, changes_ir.rows);
    assert_eq!(changes_hql.rows.len(), 2);
    assert_eq!(
        changes_hql.rows[0]["operation"],
        QueryValueV2::Utf8("upsert".into())
    );
    assert_eq!(
        changes_hql.rows[1]["operation"],
        QueryValueV2::Utf8("correct".into())
    );
    for row in &changes_hql.rows {
        let QueryValueV2::Entity(subject) = &row["subject"] else {
            panic!("row revision event must carry its exact entity subject")
        };
        assert_eq!(subject.id, row_id);
        assert_eq!(
            subject.kind,
            genesis_block_native::uee_v2::RecordKindV2::Row
        );
    }
}
