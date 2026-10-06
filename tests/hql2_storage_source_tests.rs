use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::QueryRequestV2,
    AccessContext, EdgeInput, NodeInput, OpenOptions, RelationalColumn, RelationalColumnType,
    RelationalMutationBatch, RelationalMutationKind, RelationalRowMutation,
    RelationalSchemaPackage, RelationalTable, Storage,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use tempfile::TempDir;
use uuid::Uuid;

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
        principal: "source-reader".into(),
        namespace: "default".into(),
    }
}

fn node(storage: &Storage, id: &str, label: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: vec![label.into()],
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

fn hql(
    storage: &Storage,
    request_id: &str,
    query: &str,
) -> genesis_block_native::query::hql2::QueryResultV2 {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":query,
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("query must return rows")
    };
    result
}

fn ir(
    storage: &Storage,
    request_id: &str,
    nodes: Value,
    root: &str,
) -> genesis_block_native::query::hql2::QueryResultV2 {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":nodes,
            "root":root,
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("query must return rows")
    };
    result
}

fn entity_id(result: &genesis_block_native::query::hql2::QueryResultV2, alias: &str) -> String {
    match result.rows[0].get(alias).unwrap() {
        QueryValueV2::Entity(record) => record.id.clone(),
        other => panic!("expected entity, got {other:?}"),
    }
}

fn p7_scan_entities(
    path: &Path,
    storage: &Storage,
    kind: p7_graph::Kind,
    predicate: p7_graph::Predicate,
) -> Vec<p7_graph::EntityRef> {
    let kind_name = match kind {
        p7_graph::Kind::Node => "node",
        p7_graph::Kind::Edge => "edge",
        p7_graph::Kind::Row => "row",
        _ => panic!("unsupported source-scan fixture kind: {kind:?}"),
    };
    let connection = Connection::open(path.join("projection.sqlite")).unwrap();
    let mut statement = connection
        .prepare(
            "SELECT kind, namespace, record_id, revision_id, operation, tx_from, tx_to,
                    valid_from, valid_to, payload_json
             FROM hql2_record_revisions
             WHERE tx_to IS NULL AND (kind=?1 OR (?1='edge' AND kind='node'))
             ORDER BY namespace, kind, record_id, revision_id",
        )
        .unwrap();
    let rows = statement
        .query_map([kind_name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let revisions: Vec<_> = rows
        .into_iter()
        .map(
            |(
                record_kind,
                namespace,
                id,
                revision,
                operation,
                tx_from,
                tx_to,
                valid_from,
                valid_to,
                json,
            )| {
                let revision_kind = match record_kind.as_str() {
                    "node" => p7_graph::Kind::Node,
                    "edge" => p7_graph::Kind::Edge,
                    "row" => p7_graph::Kind::Row,
                    _ => unreachable!(),
                };
                let payload: Value = serde_json::from_str(&json).unwrap();
                let fields = match revision_kind {
                    p7_graph::Kind::Node => payload["labels"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|label| {
                            (
                                "label".into(),
                                p7_graph::Scalar::Text(label.as_str().unwrap().into()),
                            )
                        })
                        .collect(),
                    p7_graph::Kind::Edge => BTreeMap::from([(
                        "relation".into(),
                        p7_graph::Scalar::Text(payload["rel"].as_str().unwrap().into()),
                    )]),
                    p7_graph::Kind::Row => BTreeMap::new(),
                    _ => unreachable!(),
                };
                let data = if revision_kind == p7_graph::Kind::Edge {
                    p7_graph::RecordData::Edge {
                        source: p7_graph::Identity {
                            namespace: namespace.clone(),
                            kind: p7_graph::Kind::Node,
                            id: payload["from"].as_str().unwrap().into(),
                        },
                        target: p7_graph::Identity {
                            namespace: namespace.clone(),
                            kind: p7_graph::Kind::Node,
                            id: payload["to"].as_str().unwrap().into(),
                        },
                        relation: payload["rel"].as_str().unwrap().into(),
                    }
                } else {
                    p7_graph::RecordData::Plain
                };
                p7_graph::Revision {
                    entity: p7_graph::EntityRef {
                        namespace,
                        kind: revision_kind,
                        id,
                        revision,
                    },
                    transaction: p7_graph::Interval {
                        start: u64::try_from(tx_from).unwrap(),
                        end: tx_to.map(|value| u64::try_from(value).unwrap()),
                    },
                    valid: p7_graph::Interval {
                        start: chrono::DateTime::parse_from_rfc3339(&valid_from)
                            .unwrap()
                            .timestamp_micros(),
                        end: valid_to.map(|value| {
                            chrono::DateTime::parse_from_rfc3339(&value)
                                .unwrap()
                                .timestamp_micros()
                        }),
                    },
                    retracted: operation == "retract",
                    fields,
                    data,
                }
            },
        )
        .collect();
    let capability = p7_graph::HistoryCapability {
        horizon: 0,
        available: true,
    };
    let mut history = BTreeMap::from([(kind, capability.clone())]);
    if kind == p7_graph::Kind::Edge {
        history.insert(p7_graph::Kind::Node, capability);
    }
    let catalog = p7_graph::Catalog {
        frontier: storage.stable_frontier(),
        history,
        revisions,
    };
    let relation = p7_graph::execute(
        &catalog,
        &p7_graph::View {
            namespace: "default".into(),
            transaction: storage.stable_frontier(),
            valid_at: chrono::Utc::now().timestamp_micros(),
            permissions: p7_graph::Permissions {
                read: catalog
                    .revisions
                    .iter()
                    .map(|revision| revision.entity.identity())
                    .collect(),
                annotation_body: BTreeSet::new(),
            },
        },
        &p7_graph::Plan {
            source: p7_graph::Source::Scan {
                kind,
                alias: "record".into(),
                predicate,
            },
            stages: vec![],
        },
        p7_graph::Limits::default(),
    )
    .unwrap();
    relation
        .rows
        .into_iter()
        .map(|row| match row.get("record").unwrap() {
            p7_graph::Binding::Entity(entity) => entity.clone(),
            other => panic!("expected P7 entity binding, got {other:?}"),
        })
        .collect()
}

fn query_entity_refs(
    result: &genesis_block_native::query::hql2::QueryResultV2,
    field: &str,
) -> Vec<p7_graph::EntityRef> {
    let mut entities: Vec<_> = result
        .rows
        .iter()
        .map(|row| match row.get(field).unwrap() {
            QueryValueV2::Entity(record) => p7_graph::EntityRef {
                namespace: record.namespace.clone(),
                kind: match &record.kind {
                    genesis_block_native::uee_v2::RecordKindV2::Node => p7_graph::Kind::Node,
                    genesis_block_native::uee_v2::RecordKindV2::Edge => p7_graph::Kind::Edge,
                    genesis_block_native::uee_v2::RecordKindV2::Row => p7_graph::Kind::Row,
                    genesis_block_native::uee_v2::RecordKindV2::Vector => p7_graph::Kind::Vector,
                    genesis_block_native::uee_v2::RecordKindV2::Annotation => {
                        p7_graph::Kind::Annotation
                    }
                    genesis_block_native::uee_v2::RecordKindV2::Artifact => {
                        p7_graph::Kind::Artifact
                    }
                },
                id: record.id.clone(),
                revision: record.revision.clone(),
            },
            other => panic!("expected entity result for {field}, got {other:?}"),
        })
        .collect();
    entities.sort();
    entities
}

#[test]
fn node_scan_filters_labels_and_returns_revision_bound_entities() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    node(&storage, "doc:one", "Document");
    node(&storage, "note:one", "Note");

    let result = hql(
        &storage,
        "node-source",
        "USE default FROM NODES Document AS n |> RETURN n",
    );
    assert_eq!(result.rows.len(), 1);
    assert_eq!(entity_id(&result, "n"), "doc:one");
    assert_eq!(result.columns[0].data_type, "Entity");
}

#[test]
fn entity_identity_fields_and_properties_use_typed_projection() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    node(&storage, "doc:one", "Document");
    node(&storage, "doc:two", "Document");

    let result = hql(
        &storage,
        "entity-property-source",
        "USE default FROM NODES Document AS n |> FILTER n.id = \"doc:one\" |> RETURN n.id AS id, prop(n, \"name\") AS name, has_prop(n, \"name\") AS has_name",
    );
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0]["id"], QueryValueV2::Utf8("doc:one".into()));
    assert_eq!(result.rows[0]["name"], QueryValueV2::Json(json!("doc:one")));
    assert_eq!(result.rows[0]["has_name"], QueryValueV2::Bool(true));
}

#[test]
fn dynamic_property_names_use_typed_hydration_and_preserve_presence() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(NodeInput {
            id: Some("doc:dynamic".into()),
            labels: vec!["Document".into()],
            props: Some(json!({"name":"doc:dynamic","optional":null})),
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"dynamic-property-hydration",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"fields","op":"Values","inputs":[],"config":{"param":"fields","as":"field"}},
                {"id":"nodes","op":"NodeScan","inputs":[],"config":{"as":"n","label":"Document"}},
                {"id":"pairs","op":"Join","inputs":["fields","nodes"],"config":{"kind":"inner","condition":{"type":"Bool","literal":true}}},
                {"id":"project","op":"Project","inputs":["pairs"],"config":{"fields":[
                    {"as":"field","expression":{"field":{"alias":"field","path":[]}}},
                    {"as":"value","expression":{"call":"prop","args":[{"field":{"alias":"n","path":[]}},{"field":{"alias":"field","path":[]}}]}},
                    {"as":"exists","expression":{"call":"has_prop","args":[{"field":{"alias":"n","path":[]}},{"field":{"alias":"field","path":[]}}]}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{"fields":"List<Utf8>"}
        },
        "params":{"fields":{"type":"List<Utf8>","value":["name","optional","missing"]}}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("dynamic property query must return rows")
    };
    let observed: BTreeMap<_, _> = result
        .rows
        .iter()
        .map(|row| {
            let QueryValueV2::Utf8(field) = &row["field"] else {
                panic!("field name must remain Utf8")
            };
            (field.clone(), (row["value"].clone(), row["exists"].clone()))
        })
        .collect();
    assert_eq!(
        observed,
        BTreeMap::from([
            (
                "missing".into(),
                (QueryValueV2::Null, QueryValueV2::Bool(false)),
            ),
            (
                "name".into(),
                (
                    QueryValueV2::Json(json!("doc:dynamic")),
                    QueryValueV2::Bool(true),
                ),
            ),
            (
                "optional".into(),
                (
                    QueryValueV2::Json(serde_json::Value::Null),
                    QueryValueV2::Bool(true),
                ),
            ),
        ])
    );
}

#[test]
fn unmatched_left_join_entity_fields_and_properties_propagate_null() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    node(&storage, "doc:left", "Document");
    node(&storage, "doc:right", "Document");
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"nullable-entity-source",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"left","op":"NodeScan","inputs":[],"config":{"as":"n","label":"Document"}},
                {"id":"right","op":"NodeScan","inputs":[],"config":{"as":"m","label":"Document"}},
                {"id":"join","op":"Join","inputs":["left","right"],"config":{"kind":"left","condition":{"binary":"eq","left":{"field":{"alias":"m","path":["id"]}},"right":{"type":"Utf8","literal":"missing"}}}},
                {"id":"project","op":"Project","inputs":["join"],"config":{"fields":[
                    {"as":"id","expression":{"field":{"alias":"m","path":["id"]}}},
                    {"as":"name","expression":{"call":"prop","args":[{"field":{"alias":"m","path":[]}}, {"type":"Utf8","literal":"name"}]}},
                    {"as":"has_name","expression":{"call":"has_prop","args":[{"field":{"alias":"m","path":[]}}, {"type":"Utf8","literal":"name"}]}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("left join must return rows")
    };
    assert_eq!(result.rows.len(), 2);
    for row in &result.rows {
        assert_eq!(row["id"], QueryValueV2::Null);
        assert_eq!(row["name"], QueryValueV2::Null);
        assert_eq!(row["has_name"], QueryValueV2::Bool(false));
    }
    assert_eq!(result.columns[0].data_type, "Nullable<Utf8>");
    assert_eq!(result.columns[1].data_type, "Nullable<Json>");
}

#[test]
fn edge_scan_filters_relations_and_returns_revision_bound_entities() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    node(&storage, "node:one", "Node");
    node(&storage, "node:two", "Node");
    for (id, rel) in [("edge:depends", "DEPENDS"), ("edge:mentions", "MENTIONS")] {
        storage
            .add_edge(EdgeInput {
                id: Some(id.into()),
                from: "node:one".into(),
                to: "node:two".into(),
                rel: rel.into(),
                props: None,
                valid_from: Some("2026-09-22T00:00:00Z".into()),
                supersede: None,
                impact: None,
                caused_by: None,
            })
            .unwrap();
    }

    let result = hql(
        &storage,
        "edge-source",
        "USE default FROM EDGES DEPENDS AS e |> RETURN e",
    );
    assert_eq!(result.rows.len(), 1);
    assert_eq!(entity_id(&result, "e"), "edge:depends");
}

#[test]
fn row_scan_returns_registry_identity_from_the_same_revision_projection() {
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
                columns: vec![RelationalColumn::required("id", RelationalColumnType::Text)],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            }],
            named_queries: vec![],
        })
        .unwrap();
    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: Uuid::new_v4().to_string(),
            namespace: "default".into(),
            schema_version: 1,
            operations: vec![RelationalRowMutation {
                table: "records".into(),
                kind: RelationalMutationKind::Insert,
                values: json!({"id":"row:one"}),
                key: None,
            }],
        })
        .unwrap();
    let projected_row_id: String = rusqlite::Connection::open(dir.path().join("projection.sqlite"))
        .unwrap()
        .query_row(
            "SELECT record_id FROM hql2_record_revisions
             WHERE namespace='default' AND kind='row' AND schema_ref='records' AND tx_to IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"row-source",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{"id":"rows","op":"RowScan","inputs":[],"config":{"table":"records","as":"r"}}],
            "root":"rows",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(actor(), request).unwrap() else {
        panic!("row scan must return rows")
    };
    assert_eq!(result.rows.len(), 1);
    let row_id = entity_id(&result, "r");
    assert_eq!(row_id, projected_row_id);
    assert_eq!(Uuid::parse_str(&row_id).unwrap().get_version_num(), 4);
    match result.rows[0].get("r").unwrap() {
        QueryValueV2::Entity(record) => {
            assert_eq!(record.kind, genesis_block_native::uee_v2::RecordKindV2::Row);
            assert_eq!(record.namespace, "default");
        }
        other => panic!("expected entity, got {other:?}"),
    }
}

#[test]
fn hql_and_ir_source_scans_have_identical_node_edge_and_row_results() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    node(&storage, "doc:one", "Document");
    node(&storage, "note:one", "Note");
    storage
        .add_edge(EdgeInput {
            id: Some("edge:depends".into()),
            from: "doc:one".into(),
            to: "note:one".into(),
            rel: "DEPENDS".into(),
            props: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    storage
        .register_relational_schema(RelationalSchemaPackage {
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
    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: Uuid::new_v4().to_string(),
            namespace: "default".into(),
            schema_version: 1,
            operations: vec![RelationalRowMutation {
                table: "records".into(),
                kind: RelationalMutationKind::Insert,
                values: json!({"id":"row:one"}),
                key: None,
            }],
        })
        .unwrap();

    let node_hql = hql(
        &storage,
        "source-node-hql",
        "USE default FROM NODES Document AS n |> RETURN n AS entity, n.id AS id",
    );
    let node_ir = ir(
        &storage,
        "source-node-ir",
        json!([
            {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"n","label":"Document"}},
            {"id":"project","op":"Project","inputs":["scan"],"config":{"fields":[
                {"as":"entity","expression":{"field":{"alias":"n","path":[]}}},
                {"as":"id","expression":{"field":{"alias":"n","path":["id"]}}}
            ]}}
        ]),
        "project",
    );
    assert_eq!(node_hql.columns, node_ir.columns);
    assert_eq!(node_hql.rows, node_ir.rows);
    assert_eq!(node_hql.semantics, node_ir.semantics);
    let expected_nodes = p7_scan_entities(
        dir.path(),
        &storage,
        p7_graph::Kind::Node,
        p7_graph::Predicate {
            equals: BTreeMap::from([("label".into(), p7_graph::Scalar::Text("Document".into()))]),
            ..p7_graph::Predicate::default()
        },
    );
    assert_eq!(query_entity_refs(&node_hql, "entity"), expected_nodes);
    assert_eq!(query_entity_refs(&node_ir, "entity"), expected_nodes);
    assert_eq!(node_hql.rows.len(), 1);
    assert_eq!(node_hql.rows[0]["id"], QueryValueV2::Utf8("doc:one".into()));

    let edge_hql = hql(
        &storage,
        "source-edge-hql",
        "USE default FROM EDGES DEPENDS AS e |> RETURN e AS entity, e.id AS id",
    );
    let edge_ir = ir(
        &storage,
        "source-edge-ir",
        json!([
            {"id":"scan","op":"EdgeScan","inputs":[],"config":{"as":"e","relation":"DEPENDS"}},
            {"id":"project","op":"Project","inputs":["scan"],"config":{"fields":[
                {"as":"entity","expression":{"field":{"alias":"e","path":[]}}},
                {"as":"id","expression":{"field":{"alias":"e","path":["id"]}}}
            ]}}
        ]),
        "project",
    );
    assert_eq!(edge_hql.columns, edge_ir.columns);
    assert_eq!(edge_hql.rows, edge_ir.rows);
    assert_eq!(edge_hql.semantics, edge_ir.semantics);
    let expected_edges = p7_scan_entities(
        dir.path(),
        &storage,
        p7_graph::Kind::Edge,
        p7_graph::Predicate {
            equals: BTreeMap::from([("relation".into(), p7_graph::Scalar::Text("DEPENDS".into()))]),
            ..p7_graph::Predicate::default()
        },
    );
    assert_eq!(query_entity_refs(&edge_hql, "entity"), expected_edges);
    assert_eq!(query_entity_refs(&edge_ir, "entity"), expected_edges);
    assert_eq!(edge_hql.rows.len(), 1);
    assert_eq!(
        edge_hql.rows[0]["id"],
        QueryValueV2::Utf8("edge:depends".into())
    );

    let row_hql = hql(
        &storage,
        "source-row-hql",
        "USE default FROM TABLE records AS r |> RETURN r AS entity, r.id AS id, prop(r, \"id\") AS key",
    );
    let row_ir = ir(
        &storage,
        "source-row-ir",
        json!([
            {"id":"scan","op":"RowScan","inputs":[],"config":{"table":"records","as":"r"}},
                {"id":"project","op":"Project","inputs":["scan"],"config":{"fields":[
                    {"as":"entity","expression":{"field":{"alias":"r","path":[]}}},
                    {"as":"id","expression":{"field":{"alias":"r","path":["id"]}}},
                {"as":"key","expression":{"call":"prop","args":[
                    {"field":{"alias":"r","path":[]}},
                    {"type":"Utf8","literal":"id"}
                ]}}
            ]}}
        ]),
        "project",
    );
    assert_eq!(row_hql.columns, row_ir.columns);
    assert_eq!(row_hql.rows, row_ir.rows);
    assert_eq!(row_hql.semantics, row_ir.semantics);
    assert_eq!(row_hql.rows.len(), 1);
    let expected_rows = p7_scan_entities(
        dir.path(),
        &storage,
        p7_graph::Kind::Row,
        p7_graph::Predicate::default(),
    );
    assert_eq!(query_entity_refs(&row_hql, "entity"), expected_rows);
    assert_eq!(query_entity_refs(&row_ir, "entity"), expected_rows);
    let QueryValueV2::Utf8(record_id) = &row_hql.rows[0]["id"] else {
        panic!("row entity id must be its durable revision UUID")
    };
    assert_eq!(record_id, &expected_rows[0].id);
    assert_eq!(Uuid::parse_str(record_id).unwrap().get_version_num(), 4);
    assert_eq!(row_hql.rows[0]["key"], QueryValueV2::Json(json!("row:one")));
}

#[test]
fn row_scan_rejects_a_selected_frontier_below_its_history_floor() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let selected_tx = storage.stable_frontier();
    storage
        .register_relational_schema(RelationalSchemaPackage {
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
    storage
        .apply_relational_batch(RelationalMutationBatch {
            mutation_id: Uuid::new_v4().to_string(),
            namespace: "default".into(),
            schema_version: 1,
            operations: vec![RelationalRowMutation {
                table: "records".into(),
                kind: RelationalMutationKind::Insert,
                values: json!({"id":"row:one"}),
                key: None,
            }],
        })
        .unwrap();

    let floor = selected_tx + 1;
    assert!(floor <= storage.stable_frontier());
    let projection = Connection::open(dir.path().join("projection.sqlite")).unwrap();
    projection
        .execute(
            "UPDATE hql2_source_history_floors SET history_floor=?1
             WHERE namespace='default' AND source='row'",
            [i64::try_from(floor).unwrap()],
        )
        .unwrap();

    let hql_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"row-at-tx-floor-hql",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":format!("USE default AT TX {selected_tx} FROM TABLE records AS r |> RETURN r"),
        "params":{}
    }))
    .unwrap();
    let hql_error = storage.query_v2(actor(), hql_request).unwrap_err();
    assert_eq!(hql_error.code, "HISTORY_UNAVAILABLE");

    let ir_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"row-at-tx-floor-ir",
        "namespace":"default",
        "temporal":{"tx_as_of":selected_tx.to_string()},
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{"id":"rows","op":"RowScan","inputs":[],"config":{"table":"records","as":"r"}}],
            "root":"rows",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let ir_error = storage.query_v2(actor(), ir_request).unwrap_err();
    assert_eq!(ir_error.code, "HISTORY_UNAVAILABLE");
}

#[test]
fn row_scan_rejects_tables_outside_the_authorized_catalog() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    let before = storage.stable_frontier();
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"unknown-row-source",
        "namespace":"default",
        "ir":{
            "contract_version":"query-ir.v2",
            "nodes":[{"id":"rows","op":"RowScan","inputs":[],"config":{"table":"missing","as":"r"}}],
            "root":"rows",
            "parameter_types":{}
        },
        "params":{}
    }))
    .unwrap();
    let error = storage.query_v2(actor(), request).unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(storage.stable_frontier(), before);
}

#[test]
fn label_filtered_scan_charges_payload_before_loading_it() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage
        .add_node(NodeInput {
            id: Some("doc:large".into()),
            labels: vec!["Document".into()],
            props: Some(json!({"body":"x".repeat(1_000_000)})),
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"bounded-source-payload",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":"USE default FROM NODES Document AS n |> RETURN n",
        "params":{},
        "budget":{"max_memory_bytes":8388608}
    }))
    .unwrap();
    let error = storage.query_v2(actor(), request).unwrap_err();
    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
}
