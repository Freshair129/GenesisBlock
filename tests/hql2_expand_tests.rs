use genesis_block_native::{
    query::hql2::{value::QueryValueV2, QueryErrorV2, QueryOutcomeV2, QueryResultV2},
    uee_v2::QueryRequestV2,
    AccessContext, EdgeInput, NodeInput, OpenOptions, Storage,
};
use serde_json::json;
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

fn run(
    storage: &Storage,
    id: &str,
    hql: Option<&str>,
    ir: serde_json::Value,
) -> genesis_block_native::query::hql2::QueryResultV2 {
    run_with_budget(storage, id, hql, ir, json!({}))
}

fn query_with_params(
    storage: &Storage,
    id: &str,
    ir: serde_json::Value,
    params: serde_json::Value,
) -> Result<QueryResultV2, QueryErrorV2> {
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":id,
        "namespace":"default",
        "params":params,
        "ir":ir
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request)? else {
        panic!("query must return rows")
    };
    Ok(result)
}

fn run_with_budget(
    storage: &Storage,
    id: &str,
    hql: Option<&str>,
    ir: serde_json::Value,
    budget: serde_json::Value,
) -> genesis_block_native::query::hql2::QueryResultV2 {
    let mut body = json!({
        "contract_version":"genesis.api.v2",
        "request_id":id,
        "namespace":"default",
        "budget":budget,
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

fn graph(storage: &Storage) {
    for id in ["a", "b", "c", "d"] {
        add_node(storage, id);
    }
    add_edge(storage, "ab-1", "a", "b", "LINK");
    add_edge(storage, "ab-2", "a", "b", "LINK");
    add_edge(storage, "bc", "b", "c", "LINK");
    add_edge(storage, "ad", "a", "d", "OTHER");
}

fn compact_ir() -> serde_json::Value {
    json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"a"}},
            {"id":"start","op":"Filter","inputs":["scan"],"config":{"predicate":{"binary":"eq","left":{"field":{"alias":"a","path":["id"]}},"right":{"type":"Utf8","literal":"a"}}}},
            {"id":"expand","op":"Expand","inputs":["start"],"config":{"optional":false,"pattern":{"start_alias":"a","end_alias":"b","edge_alias":"e","relations":["LINK"],"direction":"out","min_hops":1,"max_hops":2,"mode":"trail","path_alias":"p"}}},
            {"id":"project","op":"Project","inputs":["expand"],"config":{"fields":[
                {"as":"end","expression":{"field":{"alias":"b","path":["id"]}}},
                {"as":"edges","expression":{"field":{"alias":"e","path":[]}}},
                {"as":"path","expression":{"field":{"alias":"p","path":[]}}}
            ]}}
        ],
        "root":"project",
        "parameter_types":{}
    })
}

fn match_ir(shortest: bool) -> serde_json::Value {
    json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"match","op":"Match","inputs":[],"config":{
                "pattern":{"start_alias":"a","end_alias":"b","edge_alias":"e","relations":["LINK"],"direction":"out","min_hops":1,"max_hops":1,"mode":"walk","path_alias":"p"},
                "anchors":{},"shortest":shortest
            }},
            {"id":"project","op":"Project","inputs":["match"],"config":{"fields":[
                {"as":"from","expression":{"field":{"alias":"a","path":["id"]}}},
                {"as":"to","expression":{"field":{"alias":"b","path":["id"]}}},
                {"as":"edge","expression":{"field":{"alias":"e","path":["id"]}}},
                {"as":"path","expression":{"field":{"alias":"p","path":[]}}}
            ]}}
        ],
        "root":"project",
        "parameter_types":{}
    })
}

#[test]
fn ir_expand_preserves_parallel_paths_and_orders_by_length_then_edge_identity() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);

    let result = run(&storage, "ir-expand", None, compact_ir());
    assert_eq!(result.rows.len(), 4);
    assert_eq!(result.rows[0]["end"], QueryValueV2::Utf8("b".into()));
    assert_eq!(result.rows[1]["end"], QueryValueV2::Utf8("b".into()));
    assert_eq!(result.rows[2]["end"], QueryValueV2::Utf8("c".into()));
    assert_eq!(result.rows[3]["end"], QueryValueV2::Utf8("c".into()));
    let QueryValueV2::List(edges) = &result.rows[0]["edges"] else {
        panic!("variable-length edge alias must be a list")
    };
    assert!(matches!(&edges[0], QueryValueV2::Entity(edge) if edge.id == "ab-1"));
    let QueryValueV2::Path(path) = &result.rows[3]["path"] else {
        panic!("path alias must retain typed references")
    };
    assert_eq!(path.edges.len(), 2);
    assert_eq!(result.columns[2].data_type, "Path");
}

#[test]
fn graph_operators_use_the_selected_transaction_frontier() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    add_node(&storage, "a");
    add_node(&storage, "b");
    let selected_tx = storage.stable_frontier().to_string();
    add_edge(&storage, "ab", "a", "b", "LINK");

    let hql = format!(
        "USE default AT TX {selected_tx} MATCH (a)-[e:LINK]->(b) AS p WALK |> RETURN a.id AS from, b.id AS to, e.id AS edge, p AS path"
    );
    let hql_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"match-at-tx-hql",
        "namespace":"default",
        "language_version":"hql.v2",
        "hql":hql,
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql_request).unwrap() else {
        panic!("read query must return rows")
    };

    let ir_request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"match-at-tx-ir",
        "namespace":"default",
        "temporal":{"tx_as_of":selected_tx},
        "ir":match_ir(false),
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir_request).unwrap() else {
        panic!("read query must return rows")
    };
    assert_eq!(hql_result.snapshot.tx, selected_tx);
    assert_eq!(ir_result.snapshot.tx, selected_tx);
    assert_eq!(hql_result.columns, ir_result.columns);
    assert_eq!(hql_result.rows, ir_result.rows);
    assert!(hql_result.rows.is_empty());

    let current = run(&storage, "match-at-current-tx-ir", None, match_ir(false));
    assert_eq!(current.rows.len(), 1);
    assert!(current.snapshot.tx.parse::<u64>().unwrap() > selected_tx.parse::<u64>().unwrap());
}

#[test]
fn ir_root_match_enumerates_structural_paths_and_preserves_parallel_edges() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);

    let result = run(&storage, "root-match", None, match_ir(false));
    let tuples = result
        .rows
        .iter()
        .map(|row| (row["from"].clone(), row["to"].clone(), row["edge"].clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        tuples,
        vec![
            (
                QueryValueV2::Utf8("a".into()),
                QueryValueV2::Utf8("b".into()),
                QueryValueV2::Utf8("ab-1".into()),
            ),
            (
                QueryValueV2::Utf8("a".into()),
                QueryValueV2::Utf8("b".into()),
                QueryValueV2::Utf8("ab-2".into()),
            ),
            (
                QueryValueV2::Utf8("b".into()),
                QueryValueV2::Utf8("c".into()),
                QueryValueV2::Utf8("bc".into()),
            ),
        ]
    );
}

#[test]
fn ir_root_shortest_match_keeps_one_deterministic_path_per_endpoint_pair() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);

    let result = run(&storage, "root-shortest-match", None, match_ir(true));
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0]["edge"], QueryValueV2::Utf8("ab-1".into()));
    assert_eq!(result.rows[1]["edge"], QueryValueV2::Utf8("bc".into()));
}

#[test]
fn hql_structural_match_and_ir_use_the_same_root_operator() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);

    let ir = run(&storage, "match-ir-parity", None, match_ir(false));
    let hql = run(
        &storage,
        "match-hql-parity",
        Some("USE default MATCH (a)-[e:LINK]->(b) AS p WALK |> RETURN a.id AS from, b.id AS to, e.id AS edge, p AS path"),
        serde_json::Value::Null,
    );
    assert_eq!(hql.rows, ir.rows);
    assert_eq!(hql.columns, ir.columns);
}

#[test]
fn hql_shortest_match_keeps_the_lexically_first_parallel_edge() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);

    let result = run(
        &storage,
        "hql-shortest-match",
        Some("USE default MATCH SHORTEST (a)-[e:LINK]->(b) AS p WALK |> RETURN a.id AS from, b.id AS to, e.id AS edge"),
        serde_json::Value::Null,
    );
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0]["edge"], QueryValueV2::Utf8("ab-1".into()));
    assert_eq!(result.rows[1]["edge"], QueryValueV2::Utf8("bc".into()));
}

#[test]
fn root_match_anchors_compare_exact_record_refs_and_reject_unmatched_revisions() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);

    let mut refs_ir = match_ir(false);
    refs_ir["nodes"][1]["config"]["fields"] = json!([
        {"as":"a_ref","expression":{"field":{"alias":"a","path":[]}}},
        {"as":"b_ref","expression":{"field":{"alias":"b","path":[]}}}
    ]);
    let refs = run(&storage, "match-anchor-source-refs", None, refs_ir);
    let row = refs
        .rows
        .iter()
        .find(|row| {
            matches!((&row["a_ref"], &row["b_ref"]),
                (QueryValueV2::Entity(a), QueryValueV2::Entity(b))
                    if a.id == "a" && b.id == "b")
        })
        .unwrap();
    let QueryValueV2::Entity(start_ref) = &row["a_ref"] else {
        unreachable!()
    };
    let QueryValueV2::Entity(end_ref) = &row["b_ref"] else {
        unreachable!()
    };
    let start_ref = serde_json::to_value(start_ref).unwrap();
    let end_ref = serde_json::to_value(end_ref).unwrap();

    let mut anchored = match_ir(false);
    anchored["nodes"][0]["config"]["anchors"] = json!({
        "a":{"param":"start_ref"},
        "b":{"param":"end_ref"}
    });
    anchored["parameter_types"] = json!({
        "start_ref":"Entity",
        "end_ref":"Entity"
    });
    let params = json!({
        "start_ref":{"type":"Entity","value":start_ref},
        "end_ref":{"type":"Entity","value":end_ref}
    });
    let result = query_with_params(
        &storage,
        "match-anchor-exact",
        anchored.clone(),
        params.clone(),
    )
    .unwrap();
    assert_eq!(result.rows.len(), 2, "parallel edges remain distinct");
    assert!(result.rows.iter().all(|row| {
        row["from"] == QueryValueV2::Utf8("a".into()) && row["to"] == QueryValueV2::Utf8("b".into())
    }));

    let mut stale_params = params.clone();
    stale_params["end_ref"]["value"]["revision"] = json!("00000000-0000-4000-8000-000000000099");
    assert!(query_with_params(
        &storage,
        "match-anchor-stale",
        anchored.clone(),
        stale_params
    )
    .unwrap()
    .rows
    .is_empty());

    let mut foreign_params = params.clone();
    foreign_params["start_ref"]["value"]["database_id"] = json!("b".repeat(64));
    assert!(
        query_with_params(&storage, "match-anchor-foreign", anchored, foreign_params)
            .unwrap()
            .rows
            .is_empty()
    );

    let mut edge_anchor = match_ir(false);
    edge_anchor["nodes"][0]["config"]["anchors"] = json!({"e":{"param":"start_ref"}});
    edge_anchor["parameter_types"] = json!({"start_ref":"Entity"});
    let error = query_with_params(
        &storage,
        "match-anchor-edge-alias",
        edge_anchor,
        json!({"start_ref":params["start_ref"].clone()}),
    )
    .unwrap_err();
    assert_eq!(error.detail.unwrap()["reason"], "match_anchor_alias");

    let mut entity_echo = match_ir(false);
    entity_echo["nodes"][1]["config"]["fields"] = json!([
        {"as":"echo","expression":{"param":"start_ref"}}
    ]);
    entity_echo["parameter_types"] = json!({"start_ref":"Entity"});
    let error = query_with_params(
        &storage,
        "match-entity-parameter-outside-anchor",
        entity_echo,
        json!({"start_ref":params["start_ref"].clone()}),
    )
    .unwrap_err();
    assert_eq!(error.code, "CAPABILITY_UNSUPPORTED");
    assert_eq!(error.detail.unwrap()["reason"], "entity_parameter_scope");

    let mut constrained = match_ir(false);
    constrained["nodes"][0]["config"]["anchors"] = json!({
        "a":{"type":"Utf8","literal":"a"}
    });
    let error =
        query_with_params(&storage, "match-anchor-wrong-type", constrained, json!({})).unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "match_anchor_type");

    let mut constrained = match_ir(false);
    constrained["nodes"][0]["config"]["pattern"] = json!({
        "form":"sequence",
        "start":{"alias":"a","labels":["Node"],"properties":{}},
        "steps":[{
            "edge":{"alias":"e","relations":["LINK"],"direction":"out","min_hops":1,"max_hops":1,"properties":{}},
            "node":{"alias":"b","labels":[],"properties":{}}
        }],
        "mode":"walk",
        "path_alias":"p"
    });
    let result =
        query_with_params(&storage, "match-pattern-node-label", constrained, json!({})).unwrap();
    assert_eq!(result.rows.len(), 3);
}

#[test]
fn root_match_applies_internal_node_anchors_before_shortest_deduplication() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);
    add_edge(&storage, "ad-link", "a", "d", "LINK");
    add_edge(&storage, "dc", "d", "c", "LINK");

    let mut refs_ir = match_ir(false);
    refs_ir["nodes"][1]["config"]["fields"] = json!([
        {"as":"a_ref","expression":{"field":{"alias":"a","path":[]}}},
        {"as":"b_ref","expression":{"field":{"alias":"b","path":[]}}}
    ]);
    let refs = run(&storage, "match-shortest-anchor-source-refs", None, refs_ir);
    let row = refs
        .rows
        .iter()
        .find(|row| matches!(&row["b_ref"], QueryValueV2::Entity(record) if record.id == "d"))
        .unwrap();
    let QueryValueV2::Entity(middle_ref) = &row["b_ref"] else {
        unreachable!()
    };
    let middle_ref = serde_json::to_value(middle_ref).unwrap();

    let ir = json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"match","op":"Match","inputs":[],"config":{
                "pattern":{
                    "form":"sequence",
                    "start":{"alias":"a","labels":[],"properties":{}},
                    "steps":[
                        {"edge":{"relations":["LINK"],"direction":"out","min_hops":1,"max_hops":1,"properties":{}},"node":{"alias":"b","labels":[],"properties":{}}},
                        {"edge":{"relations":["LINK"],"direction":"out","min_hops":1,"max_hops":1,"properties":{}},"node":{"alias":"c","labels":[],"properties":{}}}
                    ],
                    "mode":"walk"
                },
                "anchors":{"b":{"param":"middle_ref"}},
                "shortest":true
            }},
            {"id":"project","op":"Project","inputs":["match"],"config":{"fields":[
                {"as":"middle","expression":{"field":{"alias":"b","path":["id"]}}},
                {"as":"end","expression":{"field":{"alias":"c","path":["id"]}}}
            ]}}
        ],
        "root":"project",
        "parameter_types":{"middle_ref":"Entity"}
    });
    let result = query_with_params(
        &storage,
        "match-shortest-internal-anchor",
        ir,
        json!({"middle_ref":{"type":"Entity","value":middle_ref}}),
    )
    .unwrap();

    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0]["middle"], QueryValueV2::Utf8("d".into()));
    assert_eq!(result.rows[0]["end"], QueryValueV2::Utf8("c".into()));
}

#[test]
fn analyze_reports_root_match_graph_work_and_budget_never_returns_partial_rows() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"match-analyze",
        "namespace":"default",
        "explain":"analyze",
        "params":{},
        "ir":match_ir(false)
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("ANALYZE must execute root Match")
    };
    let match_node = result
        .explain
        .unwrap()
        .plan
        .into_iter()
        .find(|node| node.logical_op == genesis_block_native::uee_v2::QueryOpV2::Match)
        .unwrap();
    let actual = match_node.actual.unwrap();
    assert_eq!(actual.expanded_nodes, Some(7));
    assert_eq!(actual.expanded_edges, Some(16));

    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"match-budget",
        "namespace":"default",
        "budget":{"max_expanded_nodes":1},
        "params":{},
        "ir":match_ir(false)
    }))
    .unwrap();
    assert_eq!(
        storage.query_v2(access(), request).unwrap_err().code,
        "QUERY_BUDGET_EXCEEDED"
    );
}

#[test]
fn hql_expand_uses_the_same_graph_operator_and_path_ordering() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);

    let result = run(
        &storage,
        "hql-expand",
        Some("USE default FROM NODES AS a |> FILTER a.id = \"a\" |> EXPAND (a)-[e:LINK*1..2]->(b) AS p TRAIL |> RETURN b.id AS end, e AS edges, p AS path"),
        serde_json::Value::Null,
    );
    assert_eq!(result.rows.len(), 4);
    assert_eq!(result.rows[0]["end"], QueryValueV2::Utf8("b".into()));
    assert_eq!(result.rows[1]["end"], QueryValueV2::Utf8("b".into()));
    assert_eq!(result.rows[2]["end"], QueryValueV2::Utf8("c".into()));
    assert_eq!(result.rows[3]["end"], QueryValueV2::Utf8("c".into()));
    assert!(
        matches!(&result.rows[0]["edges"], QueryValueV2::List(edges) if matches!(&edges[0], QueryValueV2::Entity(edge) if edge.id == "ab-1"))
    );
    assert!(matches!(&result.rows[3]["path"], QueryValueV2::Path(path) if path.edges.len() == 2));
    assert_eq!(result.columns[2].data_type, "Path");
}

#[test]
fn optional_expand_null_extends_only_when_no_path_and_zero_hop_is_a_path() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);

    let optional = run(
        &storage,
        "optional-expand",
        Some("USE default FROM NODES AS a |> FILTER a.id = \"d\" |> OPTIONAL EXPAND (a)-[:LINK*1..2]->(b) AS p TRAIL |> RETURN b.id AS end, p AS path"),
        serde_json::Value::Null,
    );
    assert_eq!(optional.rows.len(), 1);
    assert_eq!(optional.rows[0]["end"], QueryValueV2::Null);
    assert_eq!(optional.rows[0]["path"], QueryValueV2::Null);
    assert_eq!(optional.columns[0].data_type, "Nullable<Utf8>");
    assert_eq!(optional.columns[1].data_type, "Nullable<Path>");

    let zero_hop = run(
        &storage,
        "zero-hop-expand",
        Some("USE default FROM NODES AS a |> FILTER a.id = \"a\" |> EXPAND (a)-[:LINK*0..0]->(b) AS p SIMPLE |> RETURN b.id AS end, p AS path"),
        serde_json::Value::Null,
    );
    assert_eq!(zero_hop.rows.len(), 1);
    assert_eq!(zero_hop.rows[0]["end"], QueryValueV2::Utf8("a".into()));
    assert!(
        matches!(&zero_hop.rows[0]["path"], QueryValueV2::Path(path) if path.edges.is_empty() && path.vertices.len() == 1)
    );
}

#[test]
fn graph_edge_budget_exhaustion_fails_without_returning_partial_rows() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"expand-edge-budget",
        "namespace":"default",
        "budget":{"max_expanded_edges":1},
        "ir":compact_ir(),
        "params":{}
    }))
    .unwrap();
    let error = storage.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
}

#[test]
fn analyze_reports_measured_graph_expansion_work() {
    let dir = TempDir::new().unwrap();
    let storage = open(dir.path());
    graph(&storage);
    let request: QueryRequestV2 = serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":"expand-analyze",
        "namespace":"default",
        "explain":"analyze",
        "ir":compact_ir(),
        "params":{}
    }))
    .unwrap();
    let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request).unwrap() else {
        panic!("ANALYZE must execute the graph query")
    };
    let expand = result
        .explain
        .unwrap()
        .plan
        .into_iter()
        .find(|node| node.logical_op == genesis_block_native::uee_v2::QueryOpV2::Expand)
        .unwrap();
    let actual = expand.actual.unwrap();
    assert_eq!(actual.expanded_nodes, Some(5));
    assert_eq!(actual.expanded_edges, Some(12));
}
