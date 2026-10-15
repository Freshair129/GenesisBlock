use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2, QueryResultV2},
    uee_v2::QueryRequestV2,
    AccessContext, OpenOptions, RelationalColumn, RelationalColumnType, RelationalMutationBatch,
    RelationalMutationKind, RelationalRowMutation, RelationalSchemaPackage, RelationalTable,
    Storage,
};
#[allow(dead_code)]
#[path = "support/hql2_graph_reference.rs"]
mod p7_graph;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
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

fn row_revision_mutations(storage: &Storage) -> Vec<Value> {
    storage
        .events_since_seq(0)
        .into_iter()
        .filter_map(|signed| serde_json::to_value(signed.event).ok())
        .filter_map(|event| {
            event
                .get("Transaction")
                .or_else(|| event.get("RelationalRows"))
                .cloned()
        })
        .filter_map(|transaction| transaction.get("record_revision_transaction").cloned())
        .filter_map(|revisions| revisions.get("mutations").cloned())
        .filter_map(|mutations| mutations.as_array().cloned())
        .flatten()
        .filter(|mutation| mutation.get("kind").and_then(Value::as_str) == Some("row"))
        .collect()
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
    let first_tx = storage.stable_frontier();
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
    let second_tx = storage.stable_frontier();

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

    let mutations = row_revision_mutations(&storage);
    assert_eq!(mutations.len(), 2);
    let p7_revisions = mutations
        .iter()
        .enumerate()
        .map(|(index, mutation)| {
            let valid_from =
                chrono::DateTime::parse_from_rfc3339(mutation["valid_from"].as_str().unwrap())
                    .unwrap()
                    .timestamp_micros();
            let valid_to = mutation["valid_to"].as_str().map(|value| {
                chrono::DateTime::parse_from_rfc3339(value)
                    .unwrap()
                    .timestamp_micros()
            });
            p7_graph::Revision {
                entity: p7_graph::EntityRef {
                    namespace: mutation["namespace"].as_str().unwrap().into(),
                    kind: p7_graph::Kind::Row,
                    id: mutation["id"].as_str().unwrap().into(),
                    revision: mutation["revision_id"].as_str().unwrap().into(),
                },
                transaction: p7_graph::Interval {
                    start: [first_tx, second_tx][index],
                    end: [Some(second_tx), None][index],
                },
                valid: p7_graph::Interval {
                    start: valid_from,
                    end: valid_to,
                },
                retracted: false,
                fields: p7_graph::Fields::new(),
                data: p7_graph::RecordData::Plain,
            }
        })
        .collect();
    let row = p7_graph::Identity {
        namespace: "default".into(),
        kind: p7_graph::Kind::Row,
        id: row_id.clone(),
    };
    let p7_catalog = p7_graph::Catalog {
        frontier: second_tx,
        history: BTreeMap::from([(
            p7_graph::Kind::Row,
            p7_graph::HistoryCapability {
                horizon: 0,
                available: true,
            },
        )]),
        revisions: p7_revisions,
    };
    let p7_plan = p7_graph::Plan {
        source: p7_graph::Source::HistoryScan {
            kind: p7_graph::Kind::Row,
            alias: "h".into(),
            predicate: p7_graph::Predicate {
                id: Some(row_id.clone()),
                ..p7_graph::Predicate::default()
            },
            transactions: p7_graph::Interval {
                start: 0,
                end: second_tx.checked_add(1),
            },
            valid: p7_graph::Interval {
                start: i64::MIN,
                end: None,
            },
        },
        stages: vec![],
    };
    let p7_view = p7_graph::View {
        namespace: "default".into(),
        transaction: second_tx,
        valid_at: 0,
        permissions: p7_graph::Permissions {
            read: BTreeSet::from([row]),
            annotation_body: BTreeSet::new(),
        },
    };
    let p7_result =
        p7_graph::execute(&p7_catalog, &p7_view, &p7_plan, p7_graph::Limits::default()).unwrap();
    assert_eq!(history_hql.rows.len(), p7_result.rows.len());
    for (actual, expected) in history_hql.rows.iter().zip(&p7_result.rows) {
        let p7_graph::Binding::Entity(entity) = &expected["h"] else {
            panic!("P7 HistoryScan binds row revision identities")
        };
        let QueryValueV2::Utf8(revision) = &actual["revision"] else {
            panic!("HQL2 row history exposes the exact revision ID")
        };
        assert_eq!(entity.namespace, "default");
        assert_eq!(entity.kind, p7_graph::Kind::Row);
        assert_eq!(entity.id, row_id);
        assert_eq!(entity.revision, *revision);
    }

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
