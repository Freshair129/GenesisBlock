#[path = "support/hql2_pipeline_reference.rs"]
mod reference;

use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryErrorV2, QueryOutcomeV2},
    uee_v2::QueryRequestV2,
    AccessContext, EdgeInput, NodeInput, OpenOptions, Storage,
};
use reference::{graph, rank, Environment, Plan};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tempfile::TempDir;

fn entity(kind: graph::Kind, id: &str) -> graph::EntityRef {
    graph::EntityRef {
        namespace: "default".into(),
        kind,
        id: id.into(),
        revision: "r1".into(),
    }
}

fn label_fields(labels: &[&str]) -> BTreeMap<String, graph::Scalar> {
    labels
        .iter()
        .map(|label| (format!("label:{label}"), graph::Scalar::Boolean(true)))
        .collect()
}

fn node_record(id: &str, labels: &[&str]) -> graph::Revision {
    graph::Revision {
        entity: entity(graph::Kind::Node, id),
        transaction: graph::Interval {
            start: 1,
            end: None,
        },
        valid: graph::Interval {
            start: 0,
            end: None,
        },
        retracted: false,
        fields: label_fields(labels),
        data: graph::RecordData::Plain,
    }
}

fn oracle_environment() -> Environment {
    let nodes = [
        ("a", ["Person", "Employee"].as_slice()),
        ("b", ["Company"].as_slice()),
        ("c", ["Person"].as_slice()),
        ("d", ["Company", "Employee"].as_slice()),
    ];
    let mut revisions = nodes
        .iter()
        .map(|(id, labels)| node_record(id, labels))
        .collect::<Vec<_>>();
    for (id, from, to) in [
        ("ab-1", "a", "b"),
        ("ab-2", "a", "b"),
        ("bc", "b", "c"),
        ("ad", "a", "d"),
        ("dc", "d", "c"),
    ] {
        let mut edge = node_record(id, &[]);
        edge.entity.kind = graph::Kind::Edge;
        edge.data = graph::RecordData::Edge {
            source: entity(graph::Kind::Node, from).identity(),
            target: entity(graph::Kind::Node, to).identity(),
            relation: "LINK".into(),
        };
        revisions.push(edge);
    }
    let catalog = graph::Catalog {
        frontier: 10,
        history: [graph::Kind::Node, graph::Kind::Edge]
            .into_iter()
            .map(|kind| {
                (
                    kind,
                    graph::HistoryCapability {
                        horizon: 0,
                        available: true,
                    },
                )
            })
            .collect(),
        revisions,
    };
    let view = graph::View {
        namespace: "default".into(),
        transaction: 10,
        valid_at: 5,
        permissions: graph::Permissions {
            read: catalog
                .revisions
                .iter()
                .map(|revision| revision.entity.identity())
                .collect(),
            annotation_body: BTreeSet::new(),
        },
    };
    Environment {
        catalog,
        view,
        ranking: rank::Fixture {
            space: rank::Space {
                fingerprint: "unused".into(),
                dimension: 1,
                metric: rank::Metric::L2Squared,
            },
            analyzer_fingerprint: "unused".into(),
            documents: vec![],
        },
        tokenizers: rank::TokenizerRegistry::default(),
        limits: graph::Limits::default(),
    }
}

fn oracle_environment_with_properties() -> Environment {
    let mut environment = oracle_environment();
    for revision in &mut environment.catalog.revisions {
        let property = match (revision.entity.kind, revision.entity.id.as_str()) {
            (graph::Kind::Node, "a") => Some(("name", graph::Scalar::Text("A".into()))),
            (graph::Kind::Node, "b") => Some(("name", graph::Scalar::Text("B".into()))),
            (graph::Kind::Node, "d") => Some(("name", graph::Scalar::Text("D".into()))),
            (graph::Kind::Edge, "ab-1") => Some(("weight", graph::Scalar::Integer(1))),
            (graph::Kind::Edge, "ab-2") => Some(("weight", graph::Scalar::Integer(2))),
            (graph::Kind::Edge, _) => Some(("weight", graph::Scalar::Integer(1))),
            _ => None,
        };
        if let Some((name, value)) = property {
            revision.fields.insert(name.into(), value);
        }
    }
    environment
}

fn nested_profile() -> Value {
    json!({
        "role":"developer",
        "flags":["core","stable"],
        "meta":{"level":2,"active":true,"retired":null}
    })
}

fn nested_config() -> Value {
    json!({"limits":{"daily":5},"flags":[true,false]})
}

fn nested_config_miss() -> Value {
    json!({"limits":{"daily":5},"flags":[true]})
}

fn nested_payload() -> Value {
    json!({"route":["a","b"],"weights":[1,2],"meta":{"primary":true}})
}

fn nested_payload_miss() -> Value {
    json!({"route":["a","b"],"weights":[1,3],"meta":{"primary":true}})
}

fn oracle_environment_with_nested_json() -> Environment {
    let mut environment = oracle_environment();
    for revision in &mut environment.catalog.revisions {
        let property = match (revision.entity.kind, revision.entity.id.as_str()) {
            (graph::Kind::Node, "a") => Some(("profile", nested_profile())),
            (graph::Kind::Node, "b") => Some(("config", nested_config())),
            (graph::Kind::Node, "d") => Some(("config", nested_config_miss())),
            (graph::Kind::Edge, "ab-1" | "ad") => Some(("payload", nested_payload())),
            (graph::Kind::Edge, "ab-2") => Some(("payload", nested_payload_miss())),
            _ => None,
        };
        if let Some((name, value)) = property {
            revision
                .fields
                .insert(name.into(), graph::Scalar::Json(value));
        }
    }
    environment
}

fn predicate(id: Option<&str>, labels: &[&str]) -> graph::Predicate {
    graph::Predicate {
        id: id.map(str::to_owned),
        equals: label_fields(labels),
    }
}

fn oracle_rows(
    start_id: Option<&str>,
    start_labels: &[&str],
    steps: &[(&str, &[&str])],
    shortest: bool,
) -> Vec<(String, String, String)> {
    let expansion = graph::Expand {
        start_alias: "a".into(),
        segments: steps
            .iter()
            .enumerate()
            .map(|(index, (end_alias, labels))| graph::Segment {
                end_alias: (*end_alias).into(),
                edge_alias: Some(format!("e{}", index + 1)),
                direction: graph::Direction::Out,
                relations: BTreeSet::from(["LINK".into()]),
                min_hops: 1,
                max_hops: 1,
                node_predicate: predicate(None, labels),
                edge_predicate: graph::Predicate::default(),
            })
            .collect(),
        path_alias: None,
        mode: graph::PathMode::Walk,
        optional: false,
        shortest,
    };
    reference::execute(
        &oracle_environment(),
        &Plan::Match {
            start: "a".into(),
            predicate: predicate(start_id, start_labels),
            expansion,
        },
    )
    .unwrap()
    .rows
    .into_iter()
    .map(|row| {
        let id = |alias: &str| match &row[alias] {
            reference::Value::Graph(graph::Binding::Entity(record)) => record.id.clone(),
            _ => panic!("{alias} must bind an entity"),
        };
        (id("a"), id(steps.last().unwrap().0), id("e1"))
    })
    .collect()
}

fn predicate_with_property(labels: &[&str], name: &str, value: graph::Scalar) -> graph::Predicate {
    let mut equals = label_fields(labels);
    equals.insert(name.into(), value);
    graph::Predicate { id: None, equals }
}

fn oracle_property_rows() -> Vec<(String, String, String)> {
    let expansion = graph::Expand {
        start_alias: "a".into(),
        segments: vec![graph::Segment {
            end_alias: "b".into(),
            edge_alias: Some("e1".into()),
            direction: graph::Direction::Out,
            relations: BTreeSet::from(["LINK".into()]),
            min_hops: 1,
            max_hops: 1,
            node_predicate: predicate_with_property(
                &["Company"],
                "name",
                graph::Scalar::Text("B".into()),
            ),
            edge_predicate: predicate_with_property(&[], "weight", graph::Scalar::Integer(1)),
        }],
        path_alias: None,
        mode: graph::PathMode::Walk,
        optional: false,
        shortest: false,
    };
    reference::execute(
        &oracle_environment_with_properties(),
        &Plan::Match {
            start: "a".into(),
            predicate: predicate_with_property(
                &["Person"],
                "name",
                graph::Scalar::Text("A".into()),
            ),
            expansion,
        },
    )
    .unwrap()
    .rows
    .into_iter()
    .map(|row| {
        let id = |alias: &str| match &row[alias] {
            reference::Value::Graph(graph::Binding::Entity(record)) => record.id.clone(),
            _ => panic!("{alias} must bind an entity"),
        };
        (id("a"), id("b"), id("e1"))
    })
    .collect()
}

fn bag(rows: Vec<(String, String, String)>) -> BTreeMap<(String, String, String), usize> {
    let mut result = BTreeMap::new();
    for row in rows {
        *result.entry(row).or_insert(0) += 1;
    }
    result
}

fn optional_bag(
    rows: Vec<(String, Option<String>, Option<String>)>,
) -> BTreeMap<(String, Option<String>, Option<String>), usize> {
    let mut result = BTreeMap::new();
    for row in rows {
        *result.entry(row).or_insert(0) += 1;
    }
    result
}

fn optional_text_rows(
    result: genesis_block_native::query::hql2::QueryResultV2,
) -> Vec<(String, Option<String>, Option<String>)> {
    result
        .rows
        .into_iter()
        .map(|row| {
            let field = |name: &str| match &row[name] {
                QueryValueV2::Utf8(text) => Some(text.clone()),
                QueryValueV2::Null => None,
                other => panic!("{name} must be UTF-8 or NULL, got {other:?}"),
            };
            (
                field("from").expect("input entity must be preserved"),
                field("to"),
                field("edge"),
            )
        })
        .collect()
}

fn nested_json_oracle_rows() -> Vec<(String, String, String)> {
    let expansion = graph::Expand {
        start_alias: "a".into(),
        segments: vec![graph::Segment {
            end_alias: "b".into(),
            edge_alias: Some("e1".into()),
            direction: graph::Direction::Out,
            relations: BTreeSet::from(["LINK".into()]),
            min_hops: 1,
            max_hops: 1,
            node_predicate: predicate_with_property(
                &["Company"],
                "config",
                graph::Scalar::Json(nested_config()),
            ),
            edge_predicate: predicate_with_property(
                &[],
                "payload",
                graph::Scalar::Json(nested_payload()),
            ),
        }],
        path_alias: None,
        mode: graph::PathMode::Walk,
        optional: false,
        shortest: false,
    };
    reference::execute(
        &oracle_environment_with_nested_json(),
        &Plan::Match {
            start: "a".into(),
            predicate: predicate_with_property(
                &["Person"],
                "profile",
                graph::Scalar::Json(nested_profile()),
            ),
            expansion,
        },
    )
    .unwrap()
    .rows
    .into_iter()
    .map(|row| {
        let id = |alias: &str| match &row[alias] {
            reference::Value::Graph(graph::Binding::Entity(record)) => record.id.clone(),
            _ => panic!("{alias} must bind an entity"),
        };
        (id("a"), id("b"), id("e1"))
    })
    .collect()
}

fn optional_expand_oracle_rows(
    environment: &Environment,
    node_predicate: graph::Predicate,
    edge_predicate: graph::Predicate,
) -> Vec<(String, Option<String>, Option<String>)> {
    let expansion = graph::Expand {
        start_alias: "a".into(),
        segments: vec![graph::Segment {
            end_alias: "b".into(),
            edge_alias: Some("e1".into()),
            direction: graph::Direction::Out,
            relations: BTreeSet::from(["LINK".into()]),
            min_hops: 1,
            max_hops: 1,
            node_predicate,
            edge_predicate,
        }],
        path_alias: None,
        mode: graph::PathMode::Walk,
        optional: true,
        shortest: false,
    };
    reference::execute(
        environment,
        &Plan::Expand(
            Box::new(Plan::NodeScan("a".into(), graph::Predicate::default())),
            expansion,
        ),
    )
    .unwrap()
    .rows
    .into_iter()
    .map(|row| {
        let id = |alias: &str| match &row[alias] {
            reference::Value::Graph(graph::Binding::Entity(record)) => Some(record.id.clone()),
            reference::Value::Null => None,
            other => panic!("{alias} must be an entity or NULL, got {other:?}"),
        };
        (
            id("a").expect("input entity must be preserved"),
            id("b"),
            id("e1"),
        )
    })
    .collect()
}

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

fn access() -> AccessContext {
    AccessContext {
        principal: "graph-reader".into(),
        namespace: "default".into(),
    }
}

fn add_node(storage: &Storage, id: &str, labels: &[&str], props: Option<Value>) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: labels.iter().map(|label| (*label).into()).collect(),
            props,
            embedding: None,
            lang: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
}

fn add_edge(storage: &Storage, id: &str, from: &str, to: &str, props: Option<Value>) {
    storage
        .add_edge(EdgeInput {
            id: Some(id.into()),
            from: from.into(),
            to: to.into(),
            rel: "LINK".into(),
            props,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
}

fn storage_fixture(storage: &Storage) {
    for (id, labels) in [
        ("a", vec!["Person", "Employee"]),
        ("b", vec!["Company"]),
        ("c", vec!["Person"]),
        ("d", vec!["Company", "Employee"]),
    ] {
        add_node(storage, id, &labels, None);
    }
    for (id, from, to) in [
        ("ab-1", "a", "b"),
        ("ab-2", "a", "b"),
        ("bc", "b", "c"),
        ("ad", "a", "d"),
        ("dc", "d", "c"),
    ] {
        add_edge(storage, id, from, to, None);
    }
}

fn storage_property_fixture(storage: &Storage) {
    for (id, labels, props) in [
        ("a", vec!["Person", "Employee"], json!({"name":"A"})),
        ("b", vec!["Company"], json!({"name":"B"})),
        ("c", vec!["Person"], json!({})),
        ("d", vec!["Company", "Employee"], json!({"name":"D"})),
    ] {
        add_node(storage, id, &labels, Some(props));
    }
    for (id, from, to, weight) in [
        ("ab-1", "a", "b", 1),
        ("ab-2", "a", "b", 2),
        ("bc", "b", "c", 1),
        ("ad", "a", "d", 1),
        ("dc", "d", "c", 1),
    ] {
        add_edge(storage, id, from, to, Some(json!({"weight":weight})));
    }
}

fn storage_nested_json_fixture(storage: &Storage) {
    for (id, labels, props) in [
        (
            "a",
            vec!["Person", "Employee"],
            json!({"profile":nested_profile()}),
        ),
        ("b", vec!["Company"], json!({"config":nested_config()})),
        ("c", vec!["Person"], json!({})),
        (
            "d",
            vec!["Company", "Employee"],
            json!({"config":nested_config_miss()}),
        ),
    ] {
        add_node(storage, id, &labels, Some(props));
    }
    for (id, from, to, props) in [
        ("ab-1", "a", "b", Some(json!({"payload":nested_payload()}))),
        (
            "ab-2",
            "a",
            "b",
            Some(json!({"payload":nested_payload_miss()})),
        ),
        ("bc", "b", "c", None),
        ("ad", "a", "d", Some(json!({"payload":nested_payload()}))),
        ("dc", "d", "c", None),
    ] {
        add_edge(storage, id, from, to, props);
    }
}

fn run(
    storage: &Storage,
    request_id: &str,
    hql: Option<&str>,
    ir: Value,
    budget: Value,
) -> Result<genesis_block_native::query::hql2::QueryResultV2, QueryErrorV2> {
    run_with_params(storage, request_id, hql, ir, budget, json!({}))
}

fn run_with_params(
    storage: &Storage,
    request_id: &str,
    hql: Option<&str>,
    ir: Value,
    budget: Value,
    params: Value,
) -> Result<genesis_block_native::query::hql2::QueryResultV2, QueryErrorV2> {
    let mut body = json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "budget":budget,
        "params":params
    });
    if let Some(hql) = hql {
        body["language_version"] = json!("hql.v2");
        body["hql"] = json!(hql);
    } else {
        body["ir"] = ir;
    }
    let request: QueryRequestV2 = serde_json::from_value(body).unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request)? else {
        panic!("query must return rows")
    };
    Ok(result)
}

fn text_rows(
    result: genesis_block_native::query::hql2::QueryResultV2,
) -> Vec<(String, String, String)> {
    result
        .rows
        .into_iter()
        .map(|row| {
            let field = |name: &str| match &row[name] {
                QueryValueV2::Utf8(text) => text.clone(),
                _ => panic!("{name} must be UTF-8"),
            };
            (field("from"), field("to"), field("edge"))
        })
        .collect()
}

fn pattern_node(alias: &str, id: Option<Value>, labels: &[&str]) -> Value {
    let mut node = json!({
        "alias":alias,
        "labels":labels,
        "properties":{}
    });
    if let Some(id) = id {
        node["id"] = id;
    }
    node
}

fn utf8_id(id: &str) -> Value {
    json!({"literal":id,"type":"Utf8"})
}

fn pattern_step(alias: &str, labels: &[&str], id: Option<Value>) -> Value {
    json!({
        "edge":{"alias":"e1","relations":["LINK"],"direction":"out","min_hops":1,"max_hops":1,"properties":{}},
        "node":pattern_node(alias,id,labels)
    })
}

fn root_ir(start: Value, steps: Vec<Value>, shortest: bool, end_alias: &str) -> Value {
    json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"match","op":"Match","inputs":[],"config":{
                "pattern":{"form":"sequence","start":start,"steps":steps,"mode":"walk"},
                "anchors":{},"shortest":shortest
            }},
            {"id":"project","op":"Project","inputs":["match"],"config":{"fields":[
                {"as":"from","expression":{"field":{"alias":"a","path":["id"]}}},
                {"as":"to","expression":{"field":{"alias":end_alias,"path":["id"]}}},
                {"as":"edge","expression":{"field":{"alias":"e1","path":["id"]}}}
            ]}}
        ],
        "root":"project",
        "parameter_types":{}
    })
}

fn root_hql() -> &'static str {
    "USE default MATCH (a:Person {id:\"a\"})-[e:LINK]->(b:Company) AS p WALK |> RETURN a.id AS from, b.id AS to, e.id AS edge"
}

#[test]
fn hql_and_typed_ir_sequence_id_and_labels_match_independent_p7_bag() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_fixture(&storage);

    let hql = run(
        &storage,
        "pattern-hql-root-match",
        Some(root_hql()),
        Value::Null,
        json!({}),
    )
    .unwrap();
    let ir = run(
        &storage,
        "pattern-ir-root-match",
        None,
        root_ir(
            pattern_node("a", Some(utf8_id("a")), &["Person"]),
            vec![pattern_step("b", &["Company"], None)],
            false,
            "b",
        ),
        json!({}),
    )
    .unwrap();
    let expected = bag(oracle_rows(
        Some("a"),
        &["Person"],
        &[("b", &["Company"])],
        false,
    ));
    assert_eq!(bag(text_rows(hql.clone())), expected);
    assert_eq!(
        hql.columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        ["from", "to", "edge"]
    );
    assert_eq!(hql.rows.len(), ir.rows.len());
    assert_eq!(
        bag(text_rows(ir)),
        bag(oracle_rows(
            Some("a"),
            &["Person"],
            &[("b", &["Company"])],
            false
        ))
    );

    let conjunctive = run(
        &storage,
        "pattern-ir-conjunctive-labels",
        None,
        root_ir(
            pattern_node("a", Some(utf8_id("a")), &["Person", "Employee", "Person"]),
            vec![pattern_step("b", &["Company", "Employee"], None)],
            false,
            "b",
        ),
        json!({}),
    )
    .unwrap();
    assert_eq!(
        bag(text_rows(conjunctive)),
        bag(oracle_rows(
            Some("a"),
            &["Person", "Employee"],
            &[("b", &["Company", "Employee"])],
            false
        ))
    );

    let case_sensitive_hql = run(
        &storage,
        "pattern-label-case-hql",
        Some("USE default MATCH (a:person {id:\"a\"})-[e:LINK]->(b:Company) |> RETURN a.id AS from, b.id AS to, e.id AS edge"),
        Value::Null,
        json!({}),
    )
    .unwrap();
    let case_sensitive_ir = run(
        &storage,
        "pattern-label-case-ir",
        None,
        root_ir(
            pattern_node("a", Some(utf8_id("a")), &["person"]),
            vec![pattern_step("b", &["Company"], None)],
            false,
            "b",
        ),
        json!({}),
    )
    .unwrap();
    assert!(case_sensitive_hql.rows.is_empty());
    assert!(case_sensitive_ir.rows.is_empty());
    assert!(oracle_rows(Some("a"), &["person"], &[("b", &["Company"])], false).is_empty());
}

#[test]
fn sequence_node_and_edge_properties_match_independent_p7_bag() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_property_fixture(&storage);

    let hql = run(
        &storage,
        "pattern-property-p7-hql",
        Some("USE default MATCH (a:Person {name: \"A\"})-[e:LINK {weight: 1}]->(b:Company {name: \"B\"}) AS p WALK |> RETURN a.id AS from, b.id AS to, e.id AS edge"),
        Value::Null,
        json!({}),
    )
    .unwrap();

    let mut start = pattern_node("a", None, &["Person"]);
    start["properties"] = json!({"name":{"literal":"A","type":"Json"}});
    let mut step = pattern_step("b", &["Company"], None);
    step["edge"]["properties"] = json!({"weight":{"literal":1,"type":"Json"}});
    step["node"]["properties"] = json!({"name":{"literal":"B","type":"Json"}});
    let ir = run(
        &storage,
        "pattern-property-p7-ir",
        None,
        root_ir(start, vec![step], false, "b"),
        json!({}),
    )
    .unwrap();

    let expected = bag(oracle_property_rows());
    assert_eq!(
        expected,
        BTreeMap::from([(("a".into(), "b".into(), "ab-1".into()), 1)])
    );
    assert_eq!(bag(text_rows(hql)), expected);
    assert_eq!(bag(text_rows(ir)), expected);
}

#[test]
fn sequence_nested_json_properties_match_hql_typed_ir_and_p7_bag() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_nested_json_fixture(&storage);

    let hql = run(
        &storage,
        "pattern-nested-json-p7-hql",
        Some(
            r#"USE default MATCH (a:Person {profile: {role: "developer", flags: ["core", "stable"], meta: {level: 2, active: true, retired: NULL}}})-[e:LINK {payload: {route: ["a", "b"], weights: [1, 2], meta: {primary: true}}}]->(b:Company {config: {limits: {daily: 5}, flags: [true, false]}}) AS p WALK |> RETURN a.id AS from, b.id AS to, e.id AS edge"#,
        ),
        Value::Null,
        json!({}),
    )
    .unwrap();

    let mut start = pattern_node("a", None, &["Person"]);
    start["properties"] = json!({"profile":{"literal":nested_profile(),"type":"Json"}});
    let mut step = pattern_step("b", &["Company"], None);
    step["edge"]["properties"] = json!({"payload":{"literal":nested_payload(),"type":"Json"}});
    step["node"]["properties"] = json!({"config":{"literal":nested_config(),"type":"Json"}});
    let ir = run(
        &storage,
        "pattern-nested-json-p7-ir",
        None,
        root_ir(start, vec![step], false, "b"),
        json!({}),
    )
    .unwrap();

    let expected = bag(nested_json_oracle_rows());
    assert_eq!(
        expected,
        BTreeMap::from([(("a".into(), "b".into(), "ab-1".into()), 1)])
    );
    assert_eq!(bag(text_rows(hql)), expected);
    assert_eq!(bag(text_rows(ir)), expected);
}

#[test]
fn constrained_single_step_expand_matches_hql_and_typed_ir() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_fixture(&storage);
    let hql = run(
        &storage,
        "pattern-hql-expand",
        Some("USE default FROM NODES AS a |> FILTER a.id = \"a\" |> EXPAND (a)-[e:LINK]->(b:Company {id:\"b\"}) |> RETURN a.id AS from, b.id AS to, e.id AS edge"),
        Value::Null,
        json!({}),
    )
    .unwrap();
    let typed = run(
        &storage,
        "pattern-ir-expand",
        None,
        json!({
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"a"}},
                {"id":"expand","op":"Expand","inputs":["scan"],"config":{
                    "optional":false,
                    "pattern":{
                        "form":"sequence",
                        "start":pattern_node("a",Some(json!({"field":{"alias":"a","path":["id"]}})),&[]),
                        "steps":[pattern_step("b",&["Company","Company"],Some(utf8_id("b")))],
                        "mode":"walk"
                    }
                }},
                {"id":"project","op":"Project","inputs":["expand"],"config":{"fields":[
                    {"as":"from","expression":{"field":{"alias":"a","path":["id"]}}},
                    {"as":"to","expression":{"field":{"alias":"b","path":["id"]}}},
                    {"as":"edge","expression":{"field":{"alias":"e1","path":["id"]}}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{}
        }),
        json!({}),
    )
    .unwrap();
    let expected = bag(vec![
        ("a".into(), "b".into(), "ab-1".into()),
        ("a".into(), "b".into(), "ab-2".into()),
    ]);
    assert_eq!(bag(text_rows(hql.clone())), expected);
    assert_eq!(bag(text_rows(typed.clone())), expected);
    assert_eq!(hql.rows, typed.rows);
    assert_eq!(hql.columns, typed.columns);
}

#[test]
fn hql_sequence_id_parameter_is_bound_as_utf8() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_fixture(&storage);
    let result = run_with_params(
        &storage,
        "pattern-hql-id-param",
        Some("USE default MATCH (a:Person {id:$id})-[e:LINK]->(b:Company) |> RETURN a.id AS from, b.id AS to, e.id AS edge"),
        Value::Null,
        json!({}),
        json!({"id":{"type":"Utf8","value":"a"}}),
    )
    .unwrap();
    assert_eq!(
        bag(text_rows(result)),
        bag(oracle_rows(
            Some("a"),
            &["Person"],
            &[("b", &["Company"])],
            false
        ))
    );
}

#[test]
fn sequence_node_labels_filter_before_shortest_path_deduplication() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_fixture(&storage);
    let hql = "USE default MATCH SHORTEST (a:Person {id:\"a\"})-[e1:LINK]->(m:Employee)-[e2:LINK]->(c:Person) AS p WALK |> RETURN a.id AS from, c.id AS to, e1.id AS edge";
    let result = run(
        &storage,
        "pattern-shortest-filter",
        Some(hql),
        Value::Null,
        json!({}),
    )
    .unwrap();
    assert_eq!(result.rows[0]["edge"], QueryValueV2::Utf8("ad".into()));
    assert_eq!(result.rows.len(), 1);
    let expected = oracle_rows(
        Some("a"),
        &["Person"],
        &[("m", &["Employee"]), ("c", &["Person"])],
        true,
    );
    assert_eq!(bag(text_rows(result)), bag(expected));
}

#[test]
fn optional_sequence_constraint_miss_preserves_input_and_null_extends_new_aliases() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_fixture(&storage);
    let ir = json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"a"}},
            {"id":"expand","op":"Expand","inputs":["scan"],"config":{
                "optional":true,
                "pattern":{
                    "form":"sequence",
                    "start":pattern_node("a",Some(utf8_id("absent")),&[]),
                    "steps":[pattern_step("b",&["Company"],None)],
                    "mode":"walk"
                }
            }},
            {"id":"project","op":"Project","inputs":["expand"],"config":{"fields":[
                {"as":"from","expression":{"field":{"alias":"a","path":["id"]}}},
                {"as":"to","expression":{"field":{"alias":"b","path":["id"]}}},
                {"as":"edge","expression":{"field":{"alias":"e1","path":["id"]}}}
            ]}}
        ],
        "root":"project",
        "parameter_types":{}
    });
    let result = run(&storage, "pattern-optional-miss", None, ir, json!({})).unwrap();
    assert_eq!(result.rows.len(), 4);
    assert!(result
        .rows
        .iter()
        .all(|row| row["to"] == QueryValueV2::Null));
    assert!(result
        .rows
        .iter()
        .all(|row| row["edge"] == QueryValueV2::Null));

    let step_miss = json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"a"}},
            {"id":"expand","op":"Expand","inputs":["scan"],"config":{
                "optional":true,
                "pattern":{
                    "form":"sequence",
                    "start":pattern_node("a",None,&[]),
                    "steps":[pattern_step("b",&["Company"],Some(utf8_id("absent")))],
                    "mode":"walk"
                }
            }},
            {"id":"project","op":"Project","inputs":["expand"],"config":{"fields":[
                {"as":"from","expression":{"field":{"alias":"a","path":["id"]}}},
                {"as":"to","expression":{"field":{"alias":"b","path":["id"]}}},
                {"as":"edge","expression":{"field":{"alias":"e1","path":["id"]}}}
            ]}}
        ],
        "root":"project",
        "parameter_types":{}
    });
    let result = run(
        &storage,
        "pattern-optional-step-miss",
        None,
        step_miss,
        json!({}),
    )
    .unwrap();
    assert_eq!(result.rows.len(), 4);
    assert!(result
        .rows
        .iter()
        .all(|row| row["to"] == QueryValueV2::Null));
    assert!(result
        .rows
        .iter()
        .all(|row| row["edge"] == QueryValueV2::Null));
    assert_eq!(
        result
            .rows
            .iter()
            .map(|row| match &row["from"] {
                QueryValueV2::Utf8(id) => id.as_str(),
                _ => panic!("preserved input entity must expose its id"),
            })
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["a", "b", "c", "d"])
    );

    let hql = run(
        &storage,
        "pattern-optional-step-miss-hql",
        Some("USE default FROM NODES AS a |> OPTIONAL EXPAND (a)-[e:LINK]->(b:Company {id:\"absent\"}) |> RETURN a.id AS from, b.id AS to, e.id AS edge"),
        Value::Null,
        json!({}),
    )
    .unwrap();
    let expected = optional_bag(optional_expand_oracle_rows(
        &oracle_environment(),
        predicate(Some("absent"), &["Company"]),
        graph::Predicate::default(),
    ));
    assert_eq!(optional_bag(optional_text_rows(result)), expected);
    assert_eq!(optional_bag(optional_text_rows(hql)), expected);
}

#[test]
fn optional_sequence_property_miss_matches_hql_typed_ir_and_p7() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_property_fixture(&storage);

    let hql = run(
        &storage,
        "pattern-optional-property-miss-hql",
        Some("USE default FROM NODES AS a |> OPTIONAL EXPAND (a)-[e:LINK]->(b:Company {missing:\"x\"}) |> RETURN a.id AS from, b.id AS to, e.id AS edge"),
        Value::Null,
        json!({}),
    )
    .unwrap();

    let mut step = pattern_step("b", &["Company"], None);
    step["node"]["properties"] = json!({"missing":{"literal":"x","type":"Json"}});
    let ir = run(
        &storage,
        "pattern-optional-property-miss-ir",
        None,
        json!({
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"a"}},
                {"id":"expand","op":"Expand","inputs":["scan"],"config":{
                    "optional":true,
                    "pattern":{"form":"sequence","start":pattern_node("a",None,&[]),"steps":[step],"mode":"walk"}
                }},
                {"id":"project","op":"Project","inputs":["expand"],"config":{"fields":[
                    {"as":"from","expression":{"field":{"alias":"a","path":["id"]}}},
                    {"as":"to","expression":{"field":{"alias":"b","path":["id"]}}},
                    {"as":"edge","expression":{"field":{"alias":"e1","path":["id"]}}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{}
        }),
        json!({}),
    )
    .unwrap();

    let expected = optional_bag(optional_expand_oracle_rows(
        &oracle_environment_with_properties(),
        predicate_with_property(&["Company"], "missing", graph::Scalar::Text("x".into())),
        graph::Predicate::default(),
    ));
    assert_eq!(optional_bag(optional_text_rows(hql)), expected);
    assert_eq!(optional_bag(optional_text_rows(ir)), expected);
}

#[test]
fn optional_sequence_edge_property_miss_matches_hql_typed_ir_and_p7() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_property_fixture(&storage);

    let hql = run(
        &storage,
        "pattern-optional-edge-property-miss-hql",
        Some("USE default FROM NODES AS a |> OPTIONAL EXPAND (a)-[e:LINK {missing:\"x\"}]->(b:Company) |> RETURN a.id AS from, b.id AS to, e.id AS edge"),
        Value::Null,
        json!({}),
    )
    .unwrap();

    let mut step = pattern_step("b", &["Company"], None);
    step["edge"]["properties"] = json!({"missing":{"literal":"x","type":"Json"}});
    let ir = run(
        &storage,
        "pattern-optional-edge-property-miss-ir",
        None,
        json!({
            "contract_version":"query-ir.v2",
            "nodes":[
                {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"a"}},
                {"id":"expand","op":"Expand","inputs":["scan"],"config":{
                    "optional":true,
                    "pattern":{"form":"sequence","start":pattern_node("a",None,&[]),"steps":[step],"mode":"walk"}
                }},
                {"id":"project","op":"Project","inputs":["expand"],"config":{"fields":[
                    {"as":"from","expression":{"field":{"alias":"a","path":["id"]}}},
                    {"as":"to","expression":{"field":{"alias":"b","path":["id"]}}},
                    {"as":"edge","expression":{"field":{"alias":"e1","path":["id"]}}}
                ]}}
            ],
            "root":"project",
            "parameter_types":{}
        }),
        json!({}),
    )
    .unwrap();

    let expected = optional_bag(optional_expand_oracle_rows(
        &oracle_environment_with_properties(),
        graph::Predicate::default(),
        predicate_with_property(&[], "missing", graph::Scalar::Text("x".into())),
    ));
    assert_eq!(
        expected,
        BTreeMap::from([
            (("a".into(), None, None), 1),
            (("b".into(), None, None), 1),
            (("c".into(), None, None), 1),
            (("d".into(), None, None), 1),
        ])
    );
    assert_eq!(optional_bag(optional_text_rows(hql)), expected);
    assert_eq!(optional_bag(optional_text_rows(ir)), expected);
}

#[test]
fn missing_ids_and_unmatched_properties_return_no_rows() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_fixture(&storage);
    let missing = run(
        &storage,
        "pattern-missing-id",
        None,
        root_ir(
            pattern_node("a", Some(utf8_id("not-visible")), &[]),
            vec![pattern_step("b", &[], None)],
            false,
            "b",
        ),
        json!({}),
    )
    .unwrap();
    assert!(missing.rows.is_empty());

    let mut constrained = root_ir(
        pattern_node("a", None, &[]),
        vec![pattern_step("b", &[], None)],
        false,
        "b",
    );
    constrained["nodes"][0]["config"]["pattern"]["steps"][0]["node"]["properties"] =
        json!({"name":{"literal":"B","type":"Json"}});
    let node_property_miss = run(
        &storage,
        "pattern-node-property-miss",
        None,
        constrained,
        json!({}),
    )
    .unwrap();
    assert!(node_property_miss.rows.is_empty());

    let mut edge_constrained = root_ir(
        pattern_node("a", None, &[]),
        vec![pattern_step("b", &[], None)],
        false,
        "b",
    );
    edge_constrained["nodes"][0]["config"]["pattern"]["steps"][0]["edge"]["properties"] =
        json!({"weight":{"literal":1,"type":"Json"}});
    let edge_property_miss = run(
        &storage,
        "pattern-edge-property-miss",
        None,
        edge_constrained,
        json!({}),
    )
    .unwrap();
    assert!(edge_property_miss.rows.is_empty());

    let empty_input = run(
        &storage,
        "pattern-hql-property-empty-input",
        Some("USE default FROM NODES AS a |> FILTER a.id = \"absent\" |> EXPAND (a)-[e:LINK]->(b:Company {name:\"B\"}) |> RETURN b.id AS to"),
        Value::Null,
        json!({}),
    )
    .unwrap();
    assert!(empty_input.rows.is_empty());

    let mut invalid_type = root_ir(
        pattern_node(
            "a",
            Some(json!({"literal":"a","type":"Nullable<Utf8>"})),
            &[],
        ),
        vec![pattern_step("b", &[], None)],
        false,
        "b",
    );
    let error = run(
        &storage,
        "pattern-nullable-id-type",
        None,
        invalid_type.take(),
        json!({}),
    )
    .unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "pattern_id_type");

    let introduced_alias = json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"a"}},
            {"id":"expand","op":"Expand","inputs":["scan"],"config":{
                "optional":false,
                "pattern":{
                    "form":"sequence",
                    "start":pattern_node("a",None,&[]),
                    "steps":[pattern_step("b",&[],Some(json!({"field":{"alias":"b","path":["id"]}})))],
                    "mode":"walk"
                }
            }}
        ],
        "root":"expand",
        "parameter_types":{}
    });
    let error = run(
        &storage,
        "pattern-id-cannot-reference-introduced-alias",
        None,
        introduced_alias,
        json!({}),
    )
    .unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
}

#[test]
fn sequence_label_snapshot_exhaustion_returns_no_partial_result() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    storage_fixture(&storage);
    let error = run(
        &storage,
        "pattern-label-budget",
        Some(root_hql()),
        Value::Null,
        json!({"max_expanded_nodes":1}),
    )
    .unwrap_err();
    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
}
