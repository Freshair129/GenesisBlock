//! Independent canonical shape fixtures; no production plan/hash code is reused.
#[allow(dead_code)]
#[path = "support/hql2_diagnostics.rs"]
mod diagnostics;
use genesis_block_native::{
    query::hql2::{value::QueryValueV2 as V, ExplainResultV2, QueryOutcomeV2},
    uee_v2::{ExplainV2, QueryRequestV2},
    AccessContext, OpenOptions, Storage,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use tempfile::TempDir;

// Field order is an independently written canonical fixture, not serde_json::Value's map order.
#[derive(Clone, Serialize)]
struct GoldenColumn {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    nullable: bool,
}
#[derive(Clone, Serialize)]
struct GoldenNode {
    id: String,
    logical_op: String,
    physical_op: String,
    inputs: Vec<String>,
    columns: Vec<GoldenColumn>,
}
#[derive(Serialize)]
struct GoldenIdentity<'a> {
    planner_version: &'static str,
    root: &'a str,
    plan: &'a [GoldenNode],
}
fn column(name: &str, ty: &str, nullable: bool) -> GoldenColumn {
    GoldenColumn {
        name: name.into(),
        ty: ty.into(),
        nullable,
    }
}
fn node(
    id: &str,
    op: &str,
    physical: &str,
    inputs: &[&str],
    columns: Vec<GoldenColumn>,
) -> GoldenNode {
    GoldenNode {
        id: id.into(),
        logical_op: op.into(),
        physical_op: physical.into(),
        inputs: inputs.iter().map(|v| (*v).into()).collect(),
        columns,
    }
}
fn value_node(id: &str, alias: &str, ty: &str, nullable: bool) -> GoldenNode {
    node(
        id,
        "Values",
        "Values",
        &[],
        vec![column(alias, ty, nullable)],
    )
}
fn identity(nodes: &[GoldenNode], root: &str) -> String {
    let bytes = serde_json::to_vec(&GoldenIdentity {
        planner_version: "hql2-rule-v1",
        root,
        plan: nodes,
    })
    .unwrap();
    let mut hash = Sha256::new();
    hash.update(b"genesis.hql2.plan.v2:");
    hash.update(bytes);
    hex::encode(hash.finalize())
}
fn open(dir: &TempDir) -> Storage {
    Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        retention: Some("full".into()),
        vector_dim: Some(2),
    })
    .unwrap()
}
fn actor() -> AccessContext {
    AccessContext {
        principal: "g4-golden-reader".into(),
        namespace: "default".into(),
    }
}
fn ir(nodes: Vec<Value>, root: &str, params: Value) -> QueryRequestV2 {
    let declarations: BTreeMap<_, _> = params
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v["type"].clone()))
        .collect();
    serde_json::from_value(json!({"contract_version":"genesis.api.v2","request_id":"golden","namespace":"default","params":params,"ir":{"contract_version":"query-ir.v2","nodes":nodes,"root":root,"parameter_types":declarations}})).unwrap()
}
fn values(id: &str, param: &str, alias: &str) -> Value {
    json!({"id":id,"op":"Values","inputs":[],"config":{"param":param,"as":alias}})
}
fn rows(alias: &str, values: Vec<V>) -> Vec<BTreeMap<String, V>> {
    values
        .into_iter()
        .map(|v| BTreeMap::from([(alias.into(), v)]))
        .collect()
}
fn qualify(
    db: &Storage,
    request: QueryRequestV2,
    golden: &[GoldenNode],
    root: &str,
    expected: Vec<BTreeMap<String, V>>,
) -> ExplainResultV2 {
    let result = diagnostics::three_modes_with_access(db, actor(), request);
    assert_eq!(result.rows, expected);
    let mut actual = result.explain.unwrap();
    assert_eq!(actual.planner_version, "hql2-rule-v1");
    assert_eq!(actual.root, root);
    assert_eq!(actual.plan_hash, identity(golden, root));
    // Only actual runtime measurements are removed; estimates must remain absent.
    for n in &mut actual.plan {
        assert!(n.actual.take().is_some());
        assert!(n.estimates.is_none());
    }
    assert_eq!(
        serde_json::to_value(&actual.plan).unwrap(),
        serde_json::to_value(golden).unwrap()
    );
    actual
}

#[test]
fn fixed_values_golden_and_changed_parameters_share_shape_but_not_results() {
    let dir = TempDir::new().unwrap();
    let db = open(&dir);
    let golden = vec![value_node("v", "x", "I64", false)];
    assert_eq!(
        identity(&golden, "v"),
        "e0e22fe4309b250eb1d59be18b9baa07c84a707cbcf12396907426f58b88ba20"
    );
    for (id, xs, expected) in [
        (
            "first",
            json!(["2", "1", "2"]),
            vec![V::I64(2), V::I64(1), V::I64(2)],
        ),
        ("second", json!(["9"]), vec![V::I64(9)]),
        ("empty", json!([]), vec![]),
    ] {
        let mut request = ir(
            vec![values("v", "xs", "x")],
            "v",
            json!({"xs":{"type":"List<I64>","value":xs}}),
        );
        request.request_id = id.into();
        qualify(&db, request, &golden, "v", rows("x", expected));
    }
}

#[test]
fn raw_json_map_orders_do_not_change_the_decoded_golden() {
    let dir = TempDir::new().unwrap();
    let db = open(&dir);
    let golden = vec![
        value_node("a", "x", "I64", false),
        value_node("b", "x", "I64", false),
        node(
            "u",
            "UnionAll",
            "UnionAll",
            &["a", "b"],
            vec![column("x", "I64", false)],
        ),
    ];
    let left = r#"{"contract_version":"genesis.api.v2","request_id":"left","namespace":"default","params":{"xs":{"type":"List<I64>","value":["2","1"]},"ys":{"type":"List<I64>","value":["9"]}},"ir":{"contract_version":"query-ir.v2","nodes":[{"id":"a","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}},{"id":"b","op":"Values","inputs":[],"config":{"param":"ys","as":"x"}},{"id":"u","op":"UnionAll","inputs":["a","b"],"config":{}}],"root":"u","parameter_types":{"xs":"List<I64>","ys":"List<I64>"}}}"#;
    let right = r#"{"namespace":"default","request_id":"right","params":{"ys":{"value":["9"],"type":"List<I64>"},"xs":{"value":["2","1"],"type":"List<I64>"}},"ir":{"parameter_types":{"ys":"List<I64>","xs":"List<I64>"},"root":"u","nodes":[{"config":{"as":"x","param":"xs"},"inputs":[],"op":"Values","id":"a"},{"config":{"as":"x","param":"ys"},"inputs":[],"op":"Values","id":"b"},{"config":{},"inputs":["a","b"],"op":"UnionAll","id":"u"}],"contract_version":"query-ir.v2"},"contract_version":"genesis.api.v2"}"#;
    for raw in [left, right] {
        qualify(
            &db,
            serde_json::from_str(raw).unwrap(),
            &golden,
            "u",
            rows("x", vec![V::I64(2), V::I64(1), V::I64(9)]),
        );
    }
}

#[test]
fn emitted_ids_columns_types_and_operators_match_distinct_goldens() {
    let dir = TempDir::new().unwrap();
    let db = open(&dir);
    let base = identity(&[value_node("v", "x", "I64", false)], "v");
    for (id, alias, ty, nullable, param, expected) in [
        (
            "renamed",
            "x",
            "I64",
            false,
            json!({"type":"List<I64>","value":["1"]}),
            V::I64(1),
        ),
        (
            "v",
            "renamed",
            "I64",
            false,
            json!({"type":"List<I64>","value":["1"]}),
            V::I64(1),
        ),
        (
            "v",
            "x",
            "Utf8",
            false,
            json!({"type":"List<Utf8>","value":["one"]}),
            V::Utf8("one".into()),
        ),
        (
            "v",
            "x",
            "Nullable<I64>",
            true,
            json!({"type":"List<Nullable<I64>>","value":[null]}),
            V::Null,
        ),
    ] {
        let golden = vec![value_node(id, alias, ty, nullable)];
        assert_ne!(identity(&golden, id), base);
        qualify(
            &db,
            ir(vec![values(id, "xs", alias)], id, json!({"xs":param})),
            &golden,
            id,
            rows(alias, vec![expected]),
        );
    }
    for (op, physical, config, expected) in [
        (
            "Distinct",
            "Distinct",
            json!({}),
            vec![V::I64(2), V::I64(1)],
        ),
        ("Take", "Take", json!({"count":1}), vec![V::I64(2)]),
    ] {
        let golden = vec![
            value_node("v", "x", "I64", false),
            node(
                "root",
                op,
                physical,
                &["v"],
                vec![column("x", "I64", false)],
            ),
        ];
        assert_ne!(identity(&golden, "root"), base);
        qualify(
            &db,
            ir(
                vec![
                    values("v", "xs", "x"),
                    json!({"id":"root","op":op,"inputs":["v"],"config":config}),
                ],
                "root",
                json!({"xs":{"type":"List<I64>","value":["2","1","2"]}}),
            ),
            &golden,
            "root",
            rows("x", expected),
        );
    }
}

#[test]
fn ordered_and_repeated_union_edges_are_preserved_in_shape_and_bag() {
    let dir = TempDir::new().unwrap();
    let db = open(&dir);
    let mut hashes = vec![];
    for (inputs, repeated, expected) in [
        (vec!["a", "b"], false, vec![V::I64(1), V::I64(9)]),
        (vec!["b", "a"], false, vec![V::I64(9), V::I64(1)]),
        (vec!["a", "a"], true, vec![V::I64(1), V::I64(1)]),
    ] {
        let mut nodes = vec![values("a", "xs", "x")];
        let mut golden = vec![value_node("a", "x", "I64", false)];
        let mut params = json!({"xs":{"type":"List<I64>","value":["1"]}});
        if !repeated {
            nodes.push(values("b", "ys", "x"));
            golden.push(value_node("b", "x", "I64", false));
            params["ys"] = json!({"type":"List<I64>","value":["9"]});
        }
        nodes.push(json!({"id":"u","op":"UnionAll","inputs":inputs,"config":{}}));
        golden.push(node(
            "u",
            "UnionAll",
            "UnionAll",
            &inputs,
            vec![column("x", "I64", false)],
        ));
        let plan = qualify(
            &db,
            ir(nodes, "u", params),
            &golden,
            "u",
            rows("x", expected),
        );
        hashes.push(plan.plan_hash);
    }
    assert_ne!(hashes[0], hashes[1]);
    assert_ne!(hashes[0], hashes[2]);
    assert_ne!(hashes[1], hashes[2]);
}

#[test]
fn hql_and_ir_share_hash_only_for_identical_independently_specified_shapes() {
    let dir = TempDir::new().unwrap();
    let db = open(&dir);
    let params = json!({"xs":{"type":"List<I64>","value":["2","1"]}});
    let golden = vec![
        value_node("h0", "x", "I64", false),
        node(
            "h1",
            "Project",
            "Project",
            &["h0"],
            vec![column("x", "I64", false)],
        ),
    ];
    let hql:QueryRequestV2=serde_json::from_value(json!({"contract_version":"genesis.api.v2","request_id":"hql","namespace":"default","hql":"VALUES $xs AS x |> RETURN x","language_version":"hql.v2","params":params})).unwrap();
    let hql_plan = qualify(
        &db,
        hql,
        &golden,
        "h1",
        rows("x", vec![V::I64(2), V::I64(1)]),
    );
    let typed = ir(
        vec![
            values("h0", "xs", "x"),
            json!({"id":"h1","op":"Project","inputs":["h0"],"config":{"fields":[{"as":"x","expression":{"field":{"alias":"x","path":[]}}}]}}),
        ],
        "h1",
        params.clone(),
    );
    let typed_plan = qualify(
        &db,
        typed,
        &golden,
        "h1",
        rows("x", vec![V::I64(2), V::I64(1)]),
    );
    assert_eq!(hql_plan.plan_hash, typed_plan.plan_hash);
    let short = qualify(
        &db,
        ir(vec![values("v", "xs", "x")], "v", params),
        &[value_node("v", "x", "I64", false)],
        "v",
        rows("x", vec![V::I64(2), V::I64(1)]),
    );
    assert_ne!(hql_plan.plan_hash, short.plan_hash);
}

#[test]
fn empty_source_and_lexical_operator_have_independent_no_estimate_goldens() {
    let dir = TempDir::new().unwrap();
    let db = open(&dir);
    let scan = json!({"id":"scan","op":"NodeScan","inputs":[],"config":{"as":"n"}});
    let source = node(
        "scan",
        "NodeScan",
        "AuthorizedNodeScan",
        &[],
        vec![column("n", "Entity", false)],
    );
    qualify(
        &db,
        ir(vec![scan.clone()], "scan", json!({})),
        std::slice::from_ref(&source),
        "scan",
        vec![],
    );
    let lexical = json!({"id":"text","op":"LexicalMatch","inputs":["scan"],"config":{"entity":"n","field":"body","index":"unicode-whitespace-bm25-v1","query":{"literal":"red","type":"Utf8"},"k":1,"as":"score"}});
    let golden = vec![
        source,
        node(
            "text",
            "LexicalMatch",
            "ExactLexicalScan",
            &["scan"],
            vec![
                column("n", "Entity", false),
                column("score", "Score", false),
            ],
        ),
    ];
    let mut request = ir(vec![scan, lexical], "text", json!({}));
    // Plan-only cannot publish a new receipt; data execution remains tested separately.
    request.explain = Some(ExplainV2::Plan);
    let before = db.stable_frontier();
    let QueryOutcomeV2::Plan(plan) = db.query_v2(actor(), request.clone()).unwrap() else {
        panic!("plan")
    };
    assert_eq!(db.stable_frontier(), before);
    assert_eq!(plan.plan_hash, identity(&golden, "text"));
    assert_eq!(
        serde_json::to_value(plan.plan).unwrap(),
        serde_json::to_value(&golden).unwrap()
    );
    request.explain = None;
    qualify(&db, request, &golden, "text", vec![]);
}
