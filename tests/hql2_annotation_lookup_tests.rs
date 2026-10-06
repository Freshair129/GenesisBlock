use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::{AnnotationPutMutationV2, QueryRequestV2},
    AccessContext, NodeInput, OpenOptions, Storage,
};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;
#[allow(dead_code)]
#[path = "support/hql2_graph_reference.rs"]
mod p7_graph;

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

fn database_id(path: &Path) -> String {
    rusqlite::Connection::open(path.join("projection.sqlite"))
        .unwrap()
        .query_row(
            "SELECT database_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='node' ORDER BY tx_from LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

fn durable_ref(
    path: &Path,
    database_id: &str,
    kind: p7_graph::Kind,
    id: &str,
) -> p7_graph::EntityRef {
    let kind_name = match kind {
        p7_graph::Kind::Node => "node",
        p7_graph::Kind::Annotation => "annotation",
        other => panic!("unexpected fixture kind: {other:?}"),
    };
    let (namespace, stored_kind, stored_id, revision): (String, String, String, String) =
        rusqlite::Connection::open(path.join("projection.sqlite"))
            .unwrap()
            .query_row(
                "SELECT namespace, kind, record_id, revision_id FROM hql2_record_revisions
                 WHERE database_id=?1 AND kind=?2 AND record_id=?3 AND tx_to IS NULL
                 ORDER BY tx_from DESC LIMIT 1",
                rusqlite::params![database_id, kind_name, id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
    assert_eq!(stored_kind, kind_name);
    assert_eq!(stored_id, id);
    p7_graph::EntityRef {
        namespace,
        kind,
        id: stored_id,
        revision,
    }
}

fn frozen_ref(path: &Path, database_id: &str, id: &str) -> serde_json::Value {
    let reference = durable_ref(path, database_id, p7_graph::Kind::Node, id);
    json!({
        "ref": {
            "database_id":database_id,
            "namespace":reference.namespace,
            "kind":"node",
            "id":reference.id,
            "revision":reference.revision
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
            {"as":"annotation_id","expression":{"field":{"alias":"a","path":["id"]}}},
            {"as":"document_ref","expression":{"field":{"alias":"d","path":[]}}},
            {"as":"annotation_ref","expression":{"field":{"alias":"a","path":[]}}}
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
            "USE default FROM NODES Document AS d |> {optional_stage}ANNOTATIONS OF d AS a |> RETURN d.id AS document_id, a.id AS annotation_id, d AS document_ref, a AS annotation_ref"
        ),
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("HQL annotation lookup must return rows")
    };
    result
}

fn p7_revision(entity: p7_graph::EntityRef, data: p7_graph::RecordData) -> p7_graph::Revision {
    p7_graph::Revision {
        entity,
        transaction: p7_graph::Interval {
            start: 0,
            end: None,
        },
        valid: p7_graph::Interval {
            start: i64::MIN,
            end: None,
        },
        retracted: false,
        fields: p7_graph::Fields::new(),
        data,
    }
}

fn p7_lookup(
    documents: &[p7_graph::EntityRef],
    annotations: &[(p7_graph::EntityRef, p7_graph::EntityRef)],
    optional: bool,
) -> Vec<(p7_graph::EntityRef, Option<p7_graph::EntityRef>)> {
    use std::collections::BTreeSet;

    let revisions: Vec<_> = documents
        .iter()
        .cloned()
        .map(|entity| p7_revision(entity, p7_graph::RecordData::Plain))
        .chain(annotations.iter().map(|(annotation, target)| {
            p7_revision(
                annotation.clone(),
                p7_graph::RecordData::Annotation {
                    targets: vec![p7_graph::Target {
                        binding: p7_graph::TargetBinding::Frozen(target.clone()),
                        selector: p7_graph::Selector::Whole,
                    }],
                },
            )
        }))
        .collect();
    let identities: BTreeSet<_> = revisions
        .iter()
        .map(|revision| revision.entity.identity())
        .collect();
    let catalog = p7_graph::Catalog {
        frontier: 1,
        history: [p7_graph::Kind::Node, p7_graph::Kind::Annotation]
            .into_iter()
            .map(|kind| {
                (
                    kind,
                    p7_graph::HistoryCapability {
                        horizon: 0,
                        available: true,
                    },
                )
            })
            .collect(),
        revisions,
    };
    let plan = p7_graph::Plan {
        source: p7_graph::Source::Scan {
            kind: p7_graph::Kind::Node,
            alias: "d".into(),
            predicate: p7_graph::Predicate::default(),
        },
        stages: vec![p7_graph::Stage::Annotations(p7_graph::AnnotationLookup {
            target_alias: "d".into(),
            alias: "a".into(),
            predicate: p7_graph::Predicate::default(),
            optional,
        })],
    };
    let expected = p7_graph::execute(
        &catalog,
        &p7_graph::View {
            namespace: "default".into(),
            transaction: 1,
            valid_at: 0,
            permissions: p7_graph::Permissions {
                read: identities.clone(),
                annotation_body: identities
                    .into_iter()
                    .filter(|identity| identity.kind == p7_graph::Kind::Annotation)
                    .collect(),
            },
        },
        &plan,
        p7_graph::Limits::default(),
    )
    .unwrap();
    expected
        .rows
        .into_iter()
        .map(|row| {
            let p7_graph::Binding::Entity(document) = &row["d"] else {
                panic!("P7 scan binds document identities")
            };
            let annotation = match &row["a"] {
                p7_graph::Binding::Entity(annotation) => Some(annotation.clone()),
                p7_graph::Binding::Null => None,
                other => panic!("unexpected P7 annotation binding: {other:?}"),
            };
            (document.clone(), annotation)
        })
        .collect()
}

fn projected_lookup(
    rows: &[std::collections::BTreeMap<String, QueryValueV2>],
) -> Vec<(p7_graph::EntityRef, Option<p7_graph::EntityRef>)> {
    rows.iter()
        .map(|row| {
            let QueryValueV2::Entity(document) = &row["document_ref"] else {
                panic!("document ref must be an entity")
            };
            let annotation = match &row["annotation_ref"] {
                QueryValueV2::Entity(annotation) => Some(p7_ref(annotation)),
                QueryValueV2::Null => None,
                other => panic!("unexpected annotation ref: {other:?}"),
            };
            (p7_ref(document), annotation)
        })
        .collect()
}

fn p7_ref(record: &genesis_block_native::uee_v2::RecordRefV2) -> p7_graph::EntityRef {
    p7_graph::EntityRef {
        namespace: record.namespace.clone(),
        kind: match &record.kind {
            genesis_block_native::uee_v2::RecordKindV2::Node => p7_graph::Kind::Node,
            genesis_block_native::uee_v2::RecordKindV2::Annotation => p7_graph::Kind::Annotation,
            other => panic!("unexpected lookup entity kind: {other:?}"),
        },
        id: record.id.clone(),
        revision: record.revision.clone(),
    }
}

#[test]
fn annotation_lookup_uses_the_selected_transaction_frontier() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:target");
    let database_id = database_id(dir.path());
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
    let database_id = database_id(dir.path());
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
}

#[test]
fn lookup_preserves_duplicate_input_paths() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "doc:target");
    let database_id = database_id(dir.path());
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
    let database_id = database_id(dir.path());
    put_annotation(
        &storage,
        "review:one",
        vec![frozen_ref(dir.path(), &database_id, "doc:target")],
        vec![frozen_ref(dir.path(), &database_id, "doc:evidence")],
    );

    let optional = hql_lookup(&storage, true);
    let target = durable_ref(dir.path(), &database_id, p7_graph::Kind::Node, "doc:target");
    let evidence = durable_ref(
        dir.path(),
        &database_id,
        p7_graph::Kind::Node,
        "doc:evidence",
    );
    let annotation = durable_ref(
        dir.path(),
        &database_id,
        p7_graph::Kind::Annotation,
        "review:one",
    );
    let documents = [target.clone(), evidence];
    let annotations = [(annotation, target)];
    let expected_optional = p7_lookup(&documents, &annotations, true);
    assert_eq!(
        projected_lookup(&optional.rows),
        expected_optional,
        "HQL differs from P7"
    );
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
    assert_eq!(
        projected_lookup(&ir.rows),
        expected_optional,
        "typed IR differs from P7"
    );
    assert_eq!(projected_lookup(&optional.rows), projected_lookup(&ir.rows));

    let required = hql_lookup(&storage, false);
    assert_eq!(required.rows.len(), 1);
    assert_eq!(required.columns[1].data_type, "Utf8");
    assert_eq!(
        projected_lookup(&required.rows),
        p7_lookup(&documents, &annotations, false),
        "required HQL differs from P7"
    );
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
    let database_id = database_id(dir.path());
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
