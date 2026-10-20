use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::{AnnotationPutMutationV2, ExplainV2, QueryRequestV2},
    AccessAction, AccessContext, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource,
    EdgeInput, NodeInput, OpenOptions, PolicyAdminActor, RelationalColumn, RelationalColumnType,
    RelationalMutationBatch, RelationalMutationKind, RelationalRowMutation,
    RelationalSchemaPackage, RelationalTable, Storage,
};
use serde_json::{json, Value};
use tempfile::TempDir;
use uuid::Uuid;

#[allow(dead_code)]
#[path = "support/hql2_diagnostics.rs"]
mod diagnostics;

fn actor(principal: &str) -> AccessContext {
    AccessContext {
        principal: principal.into(),
        namespace: "default".into(),
    }
}

fn request(op: &str, config: Value) -> QueryRequestV2 {
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2", "request_id":"source-diagnostics",
        "namespace":"default", "params":{},
        "ir":{"contract_version":"query-ir.v2", "nodes":[
            {"id":"scan","op":op,"inputs":[],"config":config}
        ],"root":"scan","parameter_types":{}}
    }))
    .unwrap()
}

fn fixture(populated: bool) -> (TempDir, Storage) {
    let dir = TempDir::new().unwrap();
    let db = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap();
    db.register_relational_schema(RelationalSchemaPackage {
        namespace: "default".into(),
        schema_version: 1,
        previous_version: None,
        package_id: Uuid::new_v4().to_string(),
        schema_hash: String::new(),
        tables: vec![RelationalTable {
            name: "records".into(),
            columns: vec![RelationalColumn::required("id", RelationalColumnType::Text)],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        }],
        named_queries: vec![],
    })
    .unwrap();
    if populated {
        for id in ["one", "two"] {
            db.add_node(NodeInput {
                id: Some(id.into()),
                labels: vec!["Document".into()],
                props: Some(json!({"name":id})),
                embedding: None,
                lang: None,
                valid_from: Some("2026-09-22T00:00:00Z".into()),
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
        }
        db.add_edge(EdgeInput {
            id: Some("edge:one".into()),
            from: "one".into(),
            to: "two".into(),
            rel: "LINK".into(),
            props: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
        db.apply_relational_batch(RelationalMutationBatch {
            mutation_id: Uuid::new_v4().to_string(),
            namespace: "default".into(),
            schema_version: 1,
            operations: ["row:one", "row:two"]
                .into_iter()
                .map(|id| RelationalRowMutation {
                    table: "records".into(),
                    kind: RelationalMutationKind::Insert,
                    values: json!({"id":id}),
                    key: None,
                })
                .collect(),
        })
        .unwrap();
        let QueryOutcomeV2::Rows(nodes) = db
            .query_v2(
                actor("source-reader"),
                request("NodeScan", json!({"as":"x","label":"Document"})),
            )
            .unwrap()
        else {
            panic!("rows")
        };
        let QueryValueV2::Entity(target) = &nodes.rows[0]["x"] else {
            panic!("entity")
        };
        db.put_annotation(actor("source-reader"), AnnotationPutMutationV2 {
            annotation:json!({"id":"review:one","namespace":"default","kind":"review",
                "targets":[{"ref":target,"binding":"frozen","selector":{"type":"whole"}}],
                "evidence":[],"body":{"type":"text","text":"Reviewed."},"author":"fixture",
                "created_at":"2026-09-22T00:00:00Z","valid_from":"2026-09-22T00:00:00Z","valid_to":null}),
            expected_revision:None, valid:None,
        }).unwrap();
    }
    (dir, db)
}

#[test]
fn source_rows_are_exact_in_hql_ir_and_all_diagnostic_modes_including_empty() {
    for populated in [false, true] {
        let (_dir, db) = fixture(populated);
        for (op, config, source, count) in [
            (
                "NodeScan",
                json!({"as":"x","label":"Document"}),
                "NODES Document",
                2,
            ),
            (
                "EdgeScan",
                json!({"as":"x","relation":"LINK"}),
                "EDGES LINK",
                1,
            ),
            (
                "RowScan",
                json!({"as":"x","table":"records"}),
                "TABLE records",
                2,
            ),
            ("AnnotationScan", json!({"as":"x"}), "ANNOTATIONS", 1),
        ] {
            let expected = if populated { count } else { 0 };
            let ir = diagnostics::three_modes_with_access(
                &db,
                actor("source-reader"),
                request(op, config),
            );
            diagnostics::measured(&ir, "scan", "rows_in", "rows", 0);
            diagnostics::measured(&ir, "scan", "rows_out", "rows", expected);
            let hql: QueryRequestV2 = serde_json::from_value(json!({
                "contract_version":"genesis.api.v2","request_id":"source-hql-diagnostics",
                "namespace":"default","language_version":"hql.v2",
                "hql":format!("USE default FROM {source} AS x |> RETURN x"),"params":{}
            }))
            .unwrap();
            let h = diagnostics::three_modes_with_access(&db, actor("source-reader"), hql);
            diagnostics::measured(&h, "h0", "rows_in", "rows", 0);
            diagnostics::measured(&h, "h0", "rows_out", "rows", expected);
            assert_eq!(h.rows, ir.rows, "{op}");
            assert_eq!(h.columns, ir.columns);
            assert_eq!(h.semantics, ir.semantics);
            assert_eq!(h.completeness, ir.completeness);
            assert_eq!(ir.rows.len() as u64, expected);
            // Identity expectations come from fixture setup, not diagnostics.
            if populated && op != "RowScan" {
                let mut ids: Vec<_> = ir
                    .rows
                    .iter()
                    .map(|row| match &row["x"] {
                        QueryValueV2::Entity(entity) => entity.id.as_str(),
                        _ => panic!("entity"),
                    })
                    .collect();
                ids.sort();
                assert_eq!(
                    ids,
                    match op {
                        "NodeScan" => vec!["one", "two"],
                        "EdgeScan" => vec!["edge:one"],
                        "AnnotationScan" => vec!["review:one"],
                        _ => unreachable!(),
                    }
                );
            }
        }
    }
}

#[test]
fn source_budget_failures_return_errors_in_both_execution_modes() {
    let (_dir, db) = fixture(true);
    let original = request("NodeScan", json!({"as":"x","label":"Document"}));
    diagnostics::three_modes_with_access(&db, actor("source-reader"), original.clone());
    for limit in [
        json!({"max_expanded_nodes":1}),
        json!({"max_result_rows":1}),
        json!({"max_result_bytes":1}),
    ] {
        for mode in [None, Some(ExplainV2::Analyze)] {
            let mut limited = original.clone();
            limited.explain = mode;
            limited.budget = Some(serde_json::from_value(limit.clone()).unwrap());
            assert_eq!(
                db.query_v2(actor("source-reader"), limited)
                    .unwrap_err()
                    .code,
                "QUERY_BUDGET_EXCEEDED"
            );
        }
    }
}

#[test]
fn source_diagnostics_respect_explicit_actor_authority() {
    let (_dir, db) = fixture(true);
    db.replace_access_policy(
        PolicyAdminActor {
            access: actor("local-owner"),
        },
        0,
        AccessPolicy {
            revision: 1,
            mode: AccessPolicyMode::Enforced,
            grants: vec![
                AccessGrant {
                    principal: "source-reader".into(),
                    action: AccessAction::Read,
                    resource: AccessResource::Namespace("default".into()),
                },
                AccessGrant {
                    principal: "local-owner".into(),
                    action: AccessAction::ManagePolicy,
                    resource: AccessResource::Namespace("default".into()),
                },
            ],
        },
    )
    .unwrap();
    let original = request("AnnotationScan", json!({"as":"x"}));
    let allowed =
        diagnostics::three_modes_with_access(&db, actor("source-reader"), original.clone());
    diagnostics::measured(&allowed, "scan", "rows_out", "rows", 1);
    let lookup:QueryRequestV2=serde_json::from_value(json!({
        "contract_version":"genesis.api.v2","request_id":"actor-lookup",
        "namespace":"default","language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS d |> OPTIONAL ANNOTATIONS OF d AS a |> RETURN d.id AS document_id, a.id AS annotation_id","params":{}
    })).unwrap();
    let result = diagnostics::three_modes_with_access(&db, actor("source-reader"), lookup.clone());
    let lookup_node = result
        .explain
        .as_ref()
        .unwrap()
        .plan
        .iter()
        .find(|node| node.physical_op == "AuthorizedAnnotationLookup")
        .unwrap();
    diagnostics::measured(&result, &lookup_node.id, "rows_in", "rows", 2);
    diagnostics::measured(&result, &lookup_node.id, "rows_out", "rows", 2);
    for original in [original, lookup] {
        for mode in [None, Some(ExplainV2::Plan), Some(ExplainV2::Analyze)] {
            let mut denied = original.clone();
            denied.explain = mode;
            let error = db
                .query_v2(actor("unauthorized-reader"), denied)
                .unwrap_err();
            assert_eq!(error.code, "FORBIDDEN");
        }
    }
}
