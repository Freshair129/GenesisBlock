use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryOutcomeV2},
    uee_v2::QueryRequestV2,
    AccessContext, EdgeInput, NodeInput, OpenOptions, Storage,
};
use serde_json::{json, Value};
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

fn access() -> AccessContext {
    AccessContext {
        principal: "graph-reader".into(),
        namespace: "default".into(),
    }
}

fn add_node(storage: &Storage, id: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.into()),
            labels: vec!["Node".into()],
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

fn add_edge(storage: &Storage, id: &str, from: &str, to: &str, rel: &str) {
    storage
        .add_edge(EdgeInput {
            id: Some(id.into()),
            from: from.into(),
            to: to.into(),
            rel: rel.into(),
            props: None,
            valid_from: Some("2026-09-22T00:00:00Z".into()),
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
}

fn path_fixture(storage: &Storage) {
    for id in ["a", "b", "c", "d"] {
        add_node(storage, id);
    }
    add_edge(storage, "ab-1", "a", "b", "R");
    add_edge(storage, "ab-2", "a", "b", "R");
    add_edge(storage, "bc", "b", "c", "S");
}

fn sequence(mode: &str) -> Value {
    json!({
        "form":"sequence",
        "start":{"alias":"a","labels":[],"properties":{}},
        "steps":[
            {
                "edge":{"alias":"e1","relations":["R"],"direction":"out","min_hops":1,"max_hops":1,"properties":{}},
                "node":{"alias":"b","labels":[],"properties":{}}
            },
            {
                "edge":{"alias":"e2","relations":["S"],"direction":"out","min_hops":1,"max_hops":1,"properties":{}},
                "node":{"alias":"c","labels":[],"properties":{}}
            }
        ],
        "mode":mode,
        "path_alias":"p"
    })
}

fn ir(pattern: Value, optional: bool) -> Value {
    json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"a"}},
            {"id":"anchor","op":"Filter","inputs":["scan"],"config":{"predicate":{"binary":"eq","left":{"field":{"alias":"a","path":["id"]}},"right":{"type":"Utf8","literal":"a"}}}},
            {"id":"expand","op":"Expand","inputs":["anchor"],"config":{"optional":optional,"pattern":pattern}},
            {"id":"project","op":"Project","inputs":["expand"],"config":{"fields":[
                {"as":"middle","expression":{"field":{"alias":"b","path":["id"]}}},
                {"as":"end","expression":{"field":{"alias":"c","path":["id"]}}},
                {"as":"first_edge","expression":{"field":{"alias":"e1","path":[]}}},
                {"as":"second_edge","expression":{"field":{"alias":"e2","path":[]}}},
                {"as":"path","expression":{"field":{"alias":"p","path":[]}}}
            ]}}
        ],
        "root":"project",
        "parameter_types":{}
    })
}

fn root_match(shortest: bool) -> Value {
    json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"match","op":"Match","inputs":[],"config":{
                "pattern":sequence("trail"),"anchors":{},"shortest":shortest
            }},
            {"id":"project","op":"Project","inputs":["match"],"config":{"fields":[
                {"as":"start","expression":{"field":{"alias":"a","path":["id"]}}},
                {"as":"end","expression":{"field":{"alias":"c","path":["id"]}}},
                {"as":"first_edge","expression":{"field":{"alias":"e1","path":["id"]}}},
                {"as":"second_edge","expression":{"field":{"alias":"e2","path":["id"]}}},
                {"as":"path","expression":{"field":{"alias":"p","path":[]}}}
            ]}}
        ],
        "root":"project",
        "parameter_types":{}
    })
}

fn run(
    storage: &Storage,
    request_id: &str,
    hql: Option<&str>,
    ir: Value,
) -> genesis_block_native::query::hql2::QueryResultV2 {
    let mut body = json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "params":{}
    });
    if let Some(hql) = hql {
        body["language_version"] = json!("hql.v2");
        body["hql"] = json!(hql);
    } else {
        body["ir"] = ir;
    }
    let request: QueryRequestV2 = serde_json::from_value(body).unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("query must return rows")
    };
    result
}

#[test]
fn ir_sequence_returns_each_named_segment_and_one_typed_path() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    path_fixture(&storage);

    let result = run(&storage, "ir-sequence", None, ir(sequence("trail"), false));
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0]["middle"], QueryValueV2::Utf8("b".into()));
    assert_eq!(result.rows[0]["end"], QueryValueV2::Utf8("c".into()));
    assert_eq!(result.rows[1]["middle"], QueryValueV2::Utf8("b".into()));
    assert_eq!(result.rows[1]["end"], QueryValueV2::Utf8("c".into()));
    assert!(
        matches!(&result.rows[0]["first_edge"], QueryValueV2::Entity(edge) if edge.id == "ab-1")
    );
    assert!(
        matches!(&result.rows[1]["first_edge"], QueryValueV2::Entity(edge) if edge.id == "ab-2")
    );
    assert!(
        matches!(&result.rows[0]["second_edge"], QueryValueV2::Entity(edge) if edge.id == "bc")
    );
    assert!(
        matches!(&result.rows[0]["path"], QueryValueV2::Path(path) if path.vertices.len() == 3 && path.edges.len() == 2)
    );
    assert_eq!(result.columns[4].data_type, "Path");
}

#[test]
fn ir_root_sequence_match_binds_each_segment_and_shortest_selects_one_path() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    path_fixture(&storage);

    let all = run(&storage, "root-sequence-match", None, root_match(false));
    assert_eq!(all.rows.len(), 2);
    assert_eq!(all.rows[0]["start"], QueryValueV2::Utf8("a".into()));
    assert_eq!(all.rows[0]["end"], QueryValueV2::Utf8("c".into()));
    assert_eq!(all.rows[0]["first_edge"], QueryValueV2::Utf8("ab-1".into()));
    assert!(
        matches!(&all.rows[0]["path"], QueryValueV2::Path(path) if path.vertices.len() == 3 && path.edges.len() == 2)
    );
    let hql = run(
        &storage,
        "root-sequence-match-hql",
        Some("USE default MATCH (a)-[e1:R]->(b)-[e2:S]->(c) AS p TRAIL |> RETURN a.id AS start, c.id AS end, e1.id AS first_edge, e2.id AS second_edge, p AS path"),
        Value::Null,
    );
    assert_eq!(hql.rows, all.rows);
    assert_eq!(hql.columns, all.columns);

    let shortest = run(
        &storage,
        "root-sequence-shortest-match",
        None,
        root_match(true),
    );
    assert_eq!(shortest.rows.len(), 1);
    assert_eq!(
        shortest.rows[0]["first_edge"],
        QueryValueV2::Utf8("ab-1".into())
    );
    let hql_shortest = run(
        &storage,
        "root-sequence-shortest-match-hql",
        Some("USE default MATCH SHORTEST (a)-[e1:R]->(b)-[e2:S]->(c) AS p TRAIL |> RETURN a.id AS start, c.id AS end, e1.id AS first_edge, e2.id AS second_edge, p AS path"),
        Value::Null,
    );
    assert_eq!(hql_shortest.rows, shortest.rows);
    assert_eq!(hql_shortest.columns, shortest.columns);
}

#[test]
fn hql_sequence_uses_the_same_kernel_and_path_ordering_as_typed_ir() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    path_fixture(&storage);

    let query = "USE default FROM NODES AS a |> FILTER a.id = \"a\" |> EXPAND (a)-[e1:R]->(b)-[e2:S]->(c) AS p TRAIL |> RETURN b.id AS middle, c.id AS end, e1 AS first_edge, e2 AS second_edge, p AS path";
    let hql = run(&storage, "hql-sequence", Some(query), Value::Null);
    let typed = run(&storage, "ir-sequence", None, ir(sequence("trail"), false));
    assert_eq!(hql.rows, typed.rows);
    assert_eq!(hql.columns, typed.columns);
}

#[test]
fn sequence_uniqueness_mode_applies_across_all_steps() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "a");
    add_node(&storage, "b");
    add_edge(&storage, "ab", "a", "b", "R");

    let mut pattern = sequence("walk");
    pattern["steps"][0]["edge"]["direction"] = json!("both");
    pattern["steps"][1]["edge"]["relations"] = json!(["R"]);
    pattern["steps"][1]["edge"]["direction"] = json!("both");
    pattern["steps"][1]["edge"]["alias"] = json!("e2");
    pattern["steps"][1]["node"]["alias"] = json!("c");
    let walk = run(&storage, "walk-sequence", None, ir(pattern.clone(), false));
    assert_eq!(walk.rows.len(), 1);
    assert!(
        matches!(&walk.rows[0]["path"], QueryValueV2::Path(path) if path.vertices.iter().map(|v| v.id.as_str()).collect::<Vec<_>>() == ["a", "b", "a"] && path.edges[0].id == path.edges[1].id)
    );

    pattern["mode"] = json!("trail");
    let trail = run(&storage, "trail-sequence", None, ir(pattern.clone(), false));
    assert!(trail.rows.is_empty());

    pattern["mode"] = json!("simple");
    let simple = run(&storage, "simple-sequence", None, ir(pattern, false));
    assert!(simple.rows.is_empty());
}

#[test]
fn optional_sequence_null_extends_every_new_alias_and_matches_hql() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    path_fixture(&storage);

    let mut request = ir(sequence("trail"), true);
    request["nodes"][1]["config"]["predicate"]["right"]["literal"] = json!("d");
    let typed = run(&storage, "optional-ir-sequence", None, request);
    let query = "USE default FROM NODES AS a |> FILTER a.id = \"d\" |> OPTIONAL EXPAND (a)-[e1:R]->(b)-[e2:S]->(c) AS p TRAIL |> RETURN b.id AS middle, c.id AS end, e1 AS first_edge, e2 AS second_edge, p AS path";
    let hql = run(&storage, "optional-hql-sequence", Some(query), Value::Null);

    assert_eq!(typed.rows, hql.rows);
    assert_eq!(typed.rows.len(), 1);
    for alias in ["middle", "end", "first_edge", "second_edge", "path"] {
        assert_eq!(typed.rows[0][alias], QueryValueV2::Null, "{alias}");
    }
    assert!(typed.columns.iter().all(|column| column.nullable));
}

#[test]
fn sequence_node_label_constraints_filter_candidates() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    path_fixture(&storage);
    let mut pattern = sequence("trail");
    pattern["steps"][0]["node"]["labels"] = json!(["Person"]);
    let result = run(
        &storage,
        "constrained-sequence-label",
        None,
        ir(pattern, false),
    );
    assert!(result.rows.is_empty());
}

#[test]
fn sequence_budget_exhaustion_returns_no_partial_result() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    path_fixture(&storage);
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"sequence-edge-budget",
        "namespace":"default",
        "budget":{"max_expanded_edges":1},
        "ir":ir(sequence("trail"), false),
        "params":{}
    }))
    .unwrap();

    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
}

#[test]
fn analyze_reports_sequence_graph_work() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    path_fixture(&storage);
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"sequence-analyze",
        "namespace":"default",
        "explain":"analyze",
        "ir":ir(sequence("trail"), false),
        "params":{}
    }))
    .unwrap();

    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("ANALYZE must execute the sequence query")
    };
    let expand = result
        .explain
        .unwrap()
        .plan
        .into_iter()
        .find(|node| node.logical_op == genesis_block_native::uee_v2::QueryOpV2::Expand)
        .unwrap();
    let actual = expand.actual.unwrap();
    assert!(actual.expanded_nodes.unwrap_or_default() > 0);
    assert!(actual.expanded_edges.unwrap_or_default() > 0);
}
