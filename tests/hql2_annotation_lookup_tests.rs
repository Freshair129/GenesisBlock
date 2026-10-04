use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::{AnnotationPutMutationV2, QueryRequestV2},
    AccessContext, NodeInput, OpenOptions, Storage,
};
use serde_json::json;
#[path = "support/hql2_pipeline_reference.rs"]
mod p7;
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

fn actor() -> AccessContext {
    AccessContext {
        principal: "annotation-lookup-reader".into(),
        namespace: "default".into(),
    }
}

fn add_node(storage: &Storage, id: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: vec!["Document".into()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn database_id(storage: &Storage) -> String {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-lookup-db-id",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{"id":"values","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}}],
            "root":"values",
            "parameter_types":{"xs":"List<I64>"}
        },
        "params":{"xs":{"type":"List<I64>","value":[]}}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("values query must execute")
    };
    result.snapshot.database_id
}

fn frozen_ref(path: &Path, database_id: &str, id: &str) -> serde_json::Value {
    let connection = rusqlite::Connection::open(path.join("projection.sqlite")).unwrap();
    let revision: String = connection
        .query_row(
            "SELECT revision_id FROM hql2_record_revisions
             WHERE database_id=?1 AND namespace='default' AND kind='node'
               AND record_id=?2 AND tx_to IS NULL",
            rusqlite::params![database_id, id],
            |row| row.get(0),
        )
        .unwrap();
    json!({
        "ref": {
            "database_id":database_id,
            "namespace":"default",
            "kind":"node",
            "id":id,
            "revision":revision
        },
        "binding":"frozen",
        "selector":{"type":"whole"}
    })
}

fn live_ref(database_id: &str, id: &str) -> serde_json::Value {
    json!({
        "ref": {
            "database_id":database_id,
            "namespace":"default",
            "kind":"node",
            "id":id
        },
        "binding":"live",
        "selector":{"type":"whole"}
    })
}

fn put_annotation(
    storage: &Storage,
    id: &str,
    targets: Vec<serde_json::Value>,
    evidence: Vec<serde_json::Value>,
) {
    storage
        .put_annotation(
            actor(),
            AnnotationPutMutationV2 {
                annotation: json!({
                    "id":id,
                    "namespace":"default",
                    "kind":"review",
                    "targets":targets,
                    "evidence":evidence,
                    "body":{"type":"text","text":"Reviewed."},
                    "author":"asserted-reviewer",
                    "created_at":"2026-09-22T00:00:00Z",
                    "valid_from":"2026-09-22T00:00:00Z",
                    "valid_to":null
                }),
                expected_revision: None,
                valid: None,
            },
        )
        .unwrap();
}

fn lookup(
    storage: &Storage,
    duplicate_input: bool,
) -> genesis_block_native::query::hql2::QueryResultV2 {
    let mut nodes = vec![json!({
        "id":"documents",
        "op":"NodeScan",
        "inputs":[],
        "config":{"as":"d","label":"Document"}
    })];
    let input = if duplicate_input {
        nodes.push(json!({
            "id":"documents_again",
            "op":"NodeScan",
            "inputs":[],
            "config":{"as":"d","label":"Document"}
        }));
        nodes.push(json!({
            "id":"duplicate_paths",
            "op":"UnionAll",
            "inputs":["documents","documents_again"],
            "config":{}
        }));
        "duplicate_paths"
    } else {
        "documents"
    };
    nodes.push(json!({
        "id":"lookup",
        "op":"AnnotationLookup",
        "inputs":[input],
        "config":{"target":"d","as":"a","optional":true}
    }));
    nodes.push(json!({
        "id":"project",
        "op":"Project",
        "inputs":["lookup"],
        "config":{"fields":[
            {"as":"document_id","expression":{"field":{"alias":"d","path":["id"]}}},
            {"as":"annotation","expression":{"field":{"alias":"a","path":[]}}},
            {"as":"annotation_id","expression":{"field":{"alias":"a","path":["id"]}}}
        ]}
    }));
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-lookup",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":nodes,
            "root":"project",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("annotation lookup must return rows")
    };
    result
}

fn hql_lookup(
    storage: &Storage,
    optional: bool,
) -> genesis_block_native::query::hql2::QueryResultV2 {
    let optional_stage = if optional { "OPTIONAL " } else { "" };
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-lookup-hql",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":format!(
            "USE default FROM NODES Document AS d |> {optional_stage}ANNOTATIONS OF d AS a |> RETURN d.id AS document_id, a.id AS annotation_id"
        ),
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("HQL annotation lookup must return rows")
    };
    result
}

fn p7_entity(kind: p7::graph::Kind, id: &str, revision: &str) -> p7::graph::EntityRef {
    p7::graph::EntityRef {
        namespace: "default".into(),
        kind,
        id: id.into(),
        revision: revision.into(),
    }
}

fn p7_record(entity: p7::graph::EntityRef) -> p7::graph::Revision {
    p7::graph::Revision {
        entity,
        transaction: p7::graph::Interval {
            start: 1,
            end: None,
        },
        valid: p7::graph::Interval {
            start: 0,
            end: None,
        },
        retracted: false,
        fields: Default::default(),
        data: p7::graph::RecordData::Plain,
    }
}

fn p7_environment(target_revision: &str) -> p7::Environment {
    let target = p7_entity(p7::graph::Kind::Node, "doc:target", target_revision);
    let evidence = p7_entity(p7::graph::Kind::Node, "doc:evidence", "r1");
    let annotation = p7_entity(p7::graph::Kind::Annotation, "review:one", "r1");
    let mut annotation_record = p7_record(annotation.clone());
    annotation_record.data = p7::graph::RecordData::Annotation {
        targets: vec![p7::graph::Target {
            binding: p7::graph::TargetBinding::Frozen(target.clone()),
            selector: p7::graph::Selector::Whole,
        }],
    };
    let records = vec![p7_record(target), p7_record(evidence), annotation_record];
    let catalog = p7::graph::Catalog {
        frontier: 10,
        history: [p7::graph::Kind::Node, p7::graph::Kind::Annotation]
            .into_iter()
            .map(|kind| {
                (
                    kind,
                    p7::graph::HistoryCapability {
                        horizon: 0,
                        available: true,
                    },
                )
            })
            .collect(),
        revisions: records,
    };
    let readable = catalog
        .revisions
        .iter()
        .map(|record| record.entity.identity())
        .collect();
    p7::Environment {
        catalog,
        view: p7::graph::View {
            namespace: "default".into(),
            transaction: 10,
            valid_at: 5,
            permissions: p7::graph::Permissions {
                read: readable,
                annotation_body: [annotation.identity()].into(),
            },
        },
        ranking: p7::rank::Fixture {
            space: p7::rank::Space {
                fingerprint: "fixture".into(),
                dimension: 1,
                metric: p7::rank::Metric::L2Squared,
            },
            analyzer_fingerprint: String::new(),
            documents: vec![],
        },
        tokenizers: Default::default(),
        limits: p7::graph::Limits::default(),
    }
}

fn p7_lookup_plan(duplicate_input: bool) -> p7::Plan {
    let scan = || p7::Plan::NodeScan("d".into(), p7::graph::Predicate::default());
    let input = if duplicate_input {
        p7::Plan::UnionAll(Box::new(scan()), Box::new(scan()))
    } else {
        scan()
    };
    let lookup = p7::Plan::AnnotationLookup(
        Box::new(input),
        p7::graph::AnnotationLookup {
            target_alias: "d".into(),
            alias: "a".into(),
            predicate: p7::graph::Predicate::default(),
            optional: true,
        },
    );
    p7::Plan::Project(
        Box::new(lookup),
        vec![
            ("document".into(), p7::Expr::Field("d".into())),
            ("annotation".into(), p7::Expr::Field("a".into())),
        ],
    )
}

fn p7_ids(result: &p7::ResultSet) -> Vec<(String, Option<String>)> {
    let mut rows = result
        .rows
        .iter()
        .map(|row| {
            let document_id = match &row["document"] {
                p7::Value::Graph(p7::graph::Binding::Entity(entity)) => entity.id.clone(),
                other => panic!("expected P7 document entity, got {other:?}"),
            };
            let annotation_id = match &row["annotation"] {
                p7::Value::Graph(p7::graph::Binding::Entity(entity)) => Some(entity.id.clone()),
                p7::Value::Graph(p7::graph::Binding::Null) | p7::Value::Null => None,
                other => panic!("expected P7 annotation or null, got {other:?}"),
            };
            (document_id, annotation_id)
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

fn storage_ids(
    result: &genesis_block_native::query::hql2::QueryResultV2,
    document_column: &str,
    annotation_column: &str,
) -> Vec<(String, Option<String>)> {
    let mut rows = result
        .rows
        .iter()
        .map(|row| {
            let document_id = match row.get(document_column).unwrap() {
                QueryValueV2::Utf8(id) => id.clone(),
                other => panic!("expected document ID, got {other:?}"),
            };
            let annotation_id = match row.get(annotation_column).unwrap() {
                QueryValueV2::Utf8(id) => Some(id.clone()),
                QueryValueV2::Null => None,
                other => panic!("expected annotation ID or null, got {other:?}"),
            };
            (document_id, annotation_id)
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn hql_and_ir_annotation_lookup_match_independent_p7_bags() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:target");
    add_node(&storage, "doc:evidence");
    let database_id = database_id(&storage);
    let target_ref = frozen_ref(dir.path(), &database_id, "doc:target");
    let target_revision = target_ref["ref"]["revision"].as_str().unwrap().to_owned();
    put_annotation(
        &storage,
        "review:one",
        vec![target_ref],
        vec![frozen_ref(dir.path(), &database_id, "doc:evidence")],
    );

    let environment = p7_environment(&target_revision);
    let expected = p7_ids(&p7::execute(&environment, &p7_lookup_plan(false)).unwrap());
    assert_eq!(
        expected,
        vec![
            ("doc:evidence".into(), None),
            ("doc:target".into(), Some("review:one".into())),
        ]
    );
    assert_eq!(
        storage_ids(&hql_lookup(&storage, true), "document_id", "annotation_id"),
        expected
    );

    let duplicate_expected = p7_ids(&p7::execute(&environment, &p7_lookup_plan(true)).unwrap());
    let mut doubled_expected = [expected.clone(), expected.clone()].concat();
    doubled_expected.sort();
    assert_eq!(duplicate_expected, doubled_expected);
    assert_eq!(
        storage_ids(&lookup(&storage, true), "document_id", "annotation_id"),
        duplicate_expected
    );
}

#[test]
fn annotation_lookup_uses_the_selected_transaction_frontier() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:target");
    let database_id = database_id(&storage);
    let selected_tx = storage.stable_frontier();
    put_annotation(
        &storage,
        "review:late",
        vec![live_ref(&database_id, "doc:target")],
        vec![],
    );

    let hql: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-at-tx-hql",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":format!("USE default AT TX {selected_tx} FROM NODES Document AS d |> OPTIONAL ANNOTATIONS OF d AS a |> RETURN d.id AS document_id, a.id AS annotation_id"),
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(actor(), hql).unwrap() else {
        panic!("annotation lookup must return rows")
    };

    let ir: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"annotation-at-tx-ir",
        "namespace":"default",
        "temporal":{"tx_as_of":selected_tx.to_string()},
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"d","label":"Document"}},
                {"id":"lookup","op":"AnnotationLookup","inputs":["scan"],"config":{"target":"d","as":"a","optional":true}},
                {"id":"project","op":"Project","inputs":["lookup"],"config":{"fields":[
                    {"as":"document_id","expression":{"field":{"alias":"d","path":["id"]}}},
                    {"as":"annotation_id","expression":{"field":{"alias":"a","path":["id"]}}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(actor(), ir).unwrap() else {
        panic!("annotation lookup must return rows")
    };
    assert_eq!(hql_result.snapshot.tx, selected_tx.to_string());
    assert_eq!(ir_result.snapshot.tx, selected_tx.to_string());
    assert_eq!(hql_result.columns, ir_result.columns);
    assert_eq!(hql_result.rows, ir_result.rows);
    assert_eq!(hql_result.rows.len(), 1);
    assert_eq!(hql_result.rows[0]["annotation_id"], QueryValueV2::Null);

    let current = hql_lookup(&storage, true);
    assert_eq!(current.rows.len(), 1);
    assert_eq!(
        current.rows[0]["annotation_id"],
        QueryValueV2::Utf8("review:late".into())
    );
    assert!(current.snapshot.tx.parse::<u64>().unwrap() > selected_tx);
}

#[test]
fn lookup_matches_targets_but_not_evidence_and_keeps_optional_rows() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:target");
    add_node(&storage, "doc:evidence");
    let database_id = database_id(&storage);
    put_annotation(
        &storage,
        "review:one",
        vec![frozen_ref(dir.path(), &database_id, "doc:target")],
        vec![frozen_ref(dir.path(), &database_id, "doc:evidence")],
    );

    let result = lookup(&storage, false);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.columns[1].data_type, "Nullable<Entity>");
    assert_eq!(result.columns[2].data_type, "Nullable<Utf8>");
    assert!(result
        .rows
        .iter()
        .any(|row| row["annotation"] == QueryValueV2::Null));
    assert!(result
        .rows
        .iter()
        .any(|row| matches!(row.get("annotation"), Some(QueryValueV2::Entity(_)))));
    let rows: std::collections::BTreeMap<_, _> = result
        .rows
        .into_iter()
        .map(|row| {
            let QueryValueV2::Utf8(document_id) = row["document_id"].clone() else {
                panic!("document id must be Utf8")
            };
            (document_id, row["annotation_id"].clone())
        })
        .collect();
    assert_eq!(
        rows,
        std::collections::BTreeMap::from([
            ("doc:evidence".to_owned(), QueryValueV2::Null,),
            (
                "doc:target".to_owned(),
                QueryValueV2::Utf8("review:one".into()),
            ),
        ])
    );
}

#[test]
fn lookup_preserves_duplicate_input_paths() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:target");
    let database_id = database_id(&storage);
    put_annotation(
        &storage,
        "review:one",
        vec![frozen_ref(dir.path(), &database_id, "doc:target")],
        vec![],
    );

    let result = lookup(&storage, true);
    assert_eq!(result.rows.len(), 2);
    assert!(result.rows.iter().all(|row| {
        row["document_id"] == QueryValueV2::Utf8("doc:target".into())
            && row["annotation_id"] == QueryValueV2::Utf8("review:one".into())
    }));
}

#[test]
fn hql_annotation_lookup_distinguishes_optional_and_required_stages() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:target");
    add_node(&storage, "doc:evidence");
    let database_id = database_id(&storage);
    put_annotation(
        &storage,
        "review:one",
        vec![frozen_ref(dir.path(), &database_id, "doc:target")],
        vec![frozen_ref(dir.path(), &database_id, "doc:evidence")],
    );

    let optional = hql_lookup(&storage, true);
    assert_eq!(optional.rows.len(), 2);
    assert_eq!(optional.columns[1].data_type, "Nullable<Utf8>");
    assert!(optional
        .rows
        .iter()
        .any(|row| row["annotation_id"] == QueryValueV2::Null));
    assert!(optional
        .rows
        .iter()
        .any(|row| { row["annotation_id"] == QueryValueV2::Utf8("review:one".into()) }));
    let ir = lookup(&storage, false);
    let projected = |rows: &[std::collections::BTreeMap<String, QueryValueV2>]| {
        rows.iter()
            .map(|row| {
                let QueryValueV2::Utf8(document_id) = &row["document_id"] else {
                    panic!("document id must be Utf8")
                };
                (document_id.clone(), row["annotation_id"].clone())
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    assert_eq!(projected(&optional.rows), projected(&ir.rows));

    let required = hql_lookup(&storage, false);
    assert_eq!(required.rows.len(), 1);
    assert_eq!(required.columns[1].data_type, "Utf8");
    assert_eq!(
        required.rows[0]["annotation_id"],
        QueryValueV2::Utf8("review:one".into())
    );
}

#[test]
fn live_annotation_target_resolves_at_the_query_snapshot() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:frozen");
    add_node(&storage, "doc:live");
    let database_id = database_id(&storage);
    put_annotation(
        &storage,
        "review:frozen",
        vec![frozen_ref(dir.path(), &database_id, "doc:frozen")],
        vec![],
    );
    put_annotation(
        &storage,
        "review:live",
        vec![live_ref(&database_id, "doc:live")],
        vec![],
    );
    storage
        .supersede_node("doc:frozen".into(), Some(json!({"version":2})), None)
        .unwrap();
    storage
        .supersede_node("doc:live".into(), Some(json!({"version":2})), None)
        .unwrap();

    let result = hql_lookup(&storage, true);
    assert_eq!(result.rows.len(), 2);
    let rows: std::collections::BTreeMap<_, _> = result
        .rows
        .into_iter()
        .map(|row| {
            let QueryValueV2::Utf8(document_id) = row["document_id"].clone() else {
                panic!("document id must be Utf8")
            };
            (document_id, row["annotation_id"].clone())
        })
        .collect();
    assert_eq!(
        rows,
        std::collections::BTreeMap::from([
            ("doc:frozen".to_owned(), QueryValueV2::Null),
            (
                "doc:live".to_owned(),
                QueryValueV2::Utf8("review:live".into())
            )
        ])
    );
}
