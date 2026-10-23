//! G4 scalar diagnostics through Storage/P6, with independent P7 result oracles.
#[path = "support/hql2_reference.rs"]
mod reference;

use genesis_block_native::{
    query::hql2::{value::QueryValueV2 as V, QueryOutcomeV2},
    uee_v2::{ExplainV2, HqlLanguageVersionV2, QueryOpV2, QueryRequestV2},
    AccessAction, AccessContext, AccessGrant, AccessPolicy, AccessPolicyMode, AccessResource,
    OpenOptions, PolicyAdminActor, Storage,
};
use reference::{Expr as E, Plan as P, Value as R};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
use tempfile::TempDir;

fn open(directory: &TempDir) -> Storage {
    Storage::open(OpenOptions {
        path: directory.path().to_string_lossy().into_owned(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        retention: Some("full".into()),
        vector_dim: Some(2),
    })
    .unwrap()
}

fn access() -> AccessContext {
    AccessContext {
        principal: "g4-scalar-reader".into(),
        namespace: "default".into(),
    }
}

fn node(id: &str, op: &str, inputs: &[&str], config: Value) -> Value {
    json!({"id":id,"op":op,"inputs":inputs,"config":config})
}

fn field(alias: &str) -> Value {
    json!({"field":{"alias":alias,"path":[]}})
}

fn values(id: &str, param: &str, alias: &str) -> Value {
    node(id, "Values", &[], json!({"param":param,"as":alias}))
}

fn parameter(values: &[Option<i64>]) -> Value {
    let encoded: Vec<_> = values
        .iter()
        .map(|v| v.map(|n| json!(n.to_string())).unwrap_or(Value::Null))
        .collect();
    json!({"type":"List<Nullable<I64>>","value":encoded})
}

fn request(nodes: Vec<Value>, root: &str, params: Value) -> QueryRequestV2 {
    let declarations: BTreeMap<_, _> = params
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, value)| (name.clone(), value["type"].clone()))
        .collect();
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2","request_id":"g4-scalar-matrix",
        "namespace":"default","params":params,
        "ir":{"contract_version":"query-ir.v2","nodes":nodes,"root":root,
              "parameter_types":declarations}
    }))
    .unwrap()
}

fn p7_values(alias: &str, values: &[Option<i64>]) -> P {
    P::Values(
        values
            .iter()
            .map(|v| BTreeMap::from([(alias.into(), v.map(R::I64).unwrap_or(R::Null))]))
            .collect(),
    )
}

fn from_p7(value: &R) -> V {
    match value {
        R::Null => V::Null,
        R::I64(v) => V::I64(*v),
        R::F64(v) => V::F64(*v),
        R::List(v) => V::List(v.iter().map(from_p7).collect()),
        other => panic!("unexpected scalar oracle value: {other:?}"),
    }
}

/// Expected counts are supplied by each fixture, never taken from actual counters.
fn qualify(db: &Storage, request: QueryRequestV2, counts: &[(&str, u64, u64)], oracle: P) {
    let expected: Vec<BTreeMap<String, V>> = reference::execute(&oracle)
        .unwrap()
        .iter()
        .map(|row| row.iter().map(|(k, v)| (k.clone(), from_p7(v))).collect())
        .collect();
    let QueryOutcomeV2::Rows(normal) = db.query_v2(access(), request.clone()).unwrap() else {
        panic!("ordinary execution must return rows");
    };
    assert!(normal.explain.is_none());
    assert_eq!(normal.rows, expected);

    let mut planned = request.clone();
    planned.explain = Some(ExplainV2::Plan);
    let QueryOutcomeV2::Plan(plan) = db.query_v2(access(), planned).unwrap() else {
        panic!("EXPLAIN must return a plan");
    };
    assert!(plan
        .plan
        .iter()
        .all(|node| node.actual.is_none() && node.estimates.is_none()));

    let mut analyzed = request;
    analyzed.explain = Some(ExplainV2::Analyze);
    let QueryOutcomeV2::Rows(analyzed) = db.query_v2(access(), analyzed).unwrap() else {
        panic!("ANALYZE must return rows");
    };
    assert_eq!(normal.rows, analyzed.rows);
    assert_eq!(normal.columns, analyzed.columns);
    assert_eq!(normal.semantics, analyzed.semantics);
    assert_eq!(normal.completeness, analyzed.completeness);
    assert_eq!(
        normal.completeness.status,
        genesis_block_native::query::hql2::result::CompletionStatusV2::Complete
    );
    assert_eq!(normal.index_frontiers, analyzed.index_frontiers);
    assert_eq!(normal.cursor, analyzed.cursor);
    let mut actual_plan = analyzed.explain.unwrap();
    assert_eq!(actual_plan.plan_hash, plan.plan_hash);
    assert_eq!(actual_plan.planner_version, plan.planner_version);
    assert_eq!(actual_plan.plan.len(), counts.len());

    for &(id, rows_in, rows_out) in counts {
        let node = actual_plan.plan.iter_mut().find(|n| n.id == id).unwrap();
        let actual = serde_json::to_value(node.actual.take().unwrap()).unwrap();
        for (name, count) in [("rows_in", rows_in), ("rows_out", rows_out)] {
            assert_eq!(
                actual[name],
                json!({"unit":"rows","measurement":{"status":"measured","value":count}}),
                "{id}.{name}"
            );
        }
        for (name, unit, reason) in [
            ("bytes_read", "bytes", "not_instrumented"),
            ("work_units", "work_units", "not_instrumented"),
            ("memory_peak_bytes", "bytes", "not_instrumented"),
            ("index_probes", "index_probes", "not_applicable"),
            (
                "distance_evaluations",
                "distance_evaluations",
                "not_applicable",
            ),
            ("graph_expansions", "graph_expansions", "not_applicable"),
            ("spill_bytes", "bytes", "not_applicable"),
        ] {
            assert_eq!(
                actual[name],
                json!({"unit":unit,"measurement":{"status":"unknown","reason":reason}}),
                "{id}.{name}"
            );
        }
        assert_eq!(actual["elapsed_ns"]["unit"], "nanoseconds");
        assert_eq!(actual["elapsed_ns"]["measurement"]["status"], "measured");
        assert!(actual["elapsed_ns"]["measurement"]["value"]
            .as_u64()
            .is_some());
        assert_eq!(actual["clock_source"], "monotonic_instant");
        assert_eq!(actual["sampling"], "complete");
        assert_eq!(actual["elapsed_scope"], "operator_execution");
    }
    // Erasing only actual readings must recover the complete plan-node shape.
    assert_eq!(actual_plan.plan, plan.plan);
}

#[test]
fn scalar_pipeline_counts_duplicates_null_filtering_and_empty_inputs() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);
    for (input, sizes) in [
        (
            vec![Some(2), None, Some(1), Some(2), Some(3)],
            [5, 2, 2, 1, 1, 0, 0],
        ),
        (vec![], [0; 7]),
        (vec![None, None], [2, 0, 0, 0, 0, 0, 0]),
    ] {
        let nodes = vec![
            values("v", "xs", "x"),
            node(
                "f",
                "Filter",
                &["v"],
                json!({"predicate":{
                    "binary":"eq","left":field("x"),"right":{"type":"I64","literal":"2"}
                }}),
            ),
            node(
                "p",
                "Project",
                &["f"],
                json!({"fields":[{"expression":field("x"),"as":"y"}]}),
            ),
            node("d", "Distinct", &["p"], json!({})),
            node(
                "s",
                "Sort",
                &["d"],
                json!({"keys":[{"expression":field("y"),"direction":"desc","nulls":"last"}]}),
            ),
            node("o", "Offset", &["s"], json!({"count":1})),
            node("t", "Take", &["o"], json!({"count":1})),
        ];
        let filtered = P::Filter(
            Box::new(p7_values("x", &input)),
            E::Eq(
                Box::new(E::Field("x".into())),
                Box::new(E::Literal(R::I64(2))),
            ),
        );
        let projected = P::Project(Box::new(filtered), vec![("y".into(), E::Field("x".into()))]);
        let sorted = P::Sort(
            Box::new(P::Distinct(Box::new(projected))),
            vec![reference::SortKey {
                expr: E::Field("y".into()),
                descending: true,
                nulls_first: false,
            }],
        );
        let ids = ["v", "f", "p", "d", "s", "o", "t"];
        let counts: Vec<_> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, if i == 0 { 0 } else { sizes[i - 1] }, sizes[i]))
            .collect();
        qualify(
            &db,
            request(nodes, "t", json!({"xs":parameter(&input)})),
            &counts,
            P::Take(Box::new(P::Offset(Box::new(sorted), 1)), 1),
        );
    }
}

#[test]
fn projection_distinct_sort_offset_take_preserve_nulls_and_nonempty_results() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);
    let input = [Some(2), None, Some(1), Some(2), None];
    for (take, output_size) in [(0, 0), (2, 2)] {
        let nodes = vec![
            values("v", "xs", "x"),
            node(
                "p",
                "Project",
                &["v"],
                json!({"fields":[{"expression":field("x"),"as":"y"}]}),
            ),
            node("d", "Distinct", &["p"], json!({})),
            node(
                "s",
                "Sort",
                &["d"],
                json!({"keys":[{"expression":field("y"),"direction":"asc","nulls":"last"}]}),
            ),
            node("o", "Offset", &["s"], json!({"count":1})),
            node("t", "Take", &["o"], json!({"count":take})),
        ];
        let projected = P::Project(
            Box::new(p7_values("x", &input)),
            vec![("y".into(), E::Field("x".into()))],
        );
        let sorted = P::Sort(
            Box::new(P::Distinct(Box::new(projected))),
            vec![reference::SortKey {
                expr: E::Field("y".into()),
                descending: false,
                nulls_first: false,
            }],
        );
        qualify(
            &db,
            request(nodes, "t", json!({"xs":parameter(&input)})),
            &[
                ("v", 0, 5),
                ("p", 5, 5),
                ("d", 5, 3),
                ("s", 3, 3),
                ("o", 3, 2),
                ("t", 2, output_size),
            ],
            P::Take(Box::new(P::Offset(Box::new(sorted), 1)), take),
        );
    }
}

#[test]
fn union_all_counts_both_inputs_including_repeated_input_edges() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);
    for (left, right) in [
        (vec![Some(1), None, Some(1)], vec![None, Some(2)]),
        (vec![], vec![Some(2)]),
        (vec![None], vec![]),
        (vec![], vec![]),
    ] {
        let (l, r) = (left.len() as u64, right.len() as u64);
        qualify(
            &db,
            request(
                vec![
                    values("l", "xs", "x"),
                    values("r", "ys", "x"),
                    node("u", "UnionAll", &["l", "r"], json!({})),
                ],
                "u",
                json!({"xs":parameter(&left),"ys":parameter(&right)}),
            ),
            &[("l", 0, l), ("r", 0, r), ("u", l + r, l + r)],
            P::UnionAll(
                Box::new(p7_values("x", &left)),
                Box::new(p7_values("x", &right)),
            ),
        );
    }
    let input = [Some(1), None, Some(1)];
    qualify(
        &db,
        request(
            vec![
                values("v", "xs", "x"),
                node("u", "UnionAll", &["v", "v"], json!({})),
            ],
            "u",
            json!({"xs":parameter(&input)}),
        ),
        &[("v", 0, 3), ("u", 6, 6)],
        P::UnionAll(
            Box::new(p7_values("x", &input)),
            Box::new(p7_values("x", &input)),
        ),
    );
}

#[test]
fn all_join_kinds_count_input_bags_even_when_one_side_is_empty() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);
    // Each fixture pins expected INNER/LEFT/SEMI/ANTI outputs independently.
    for (left, right, outputs) in [
        (
            vec![Some(1), Some(1), None, Some(2)],
            vec![Some(1), Some(1), None, Some(99)],
            [4, 6, 2, 2],
        ),
        (vec![], vec![Some(1)], [0, 0, 0, 0]),
        (vec![Some(1), None], vec![], [0, 2, 0, 2]),
        (vec![], vec![], [0, 0, 0, 0]),
    ] {
        for (index, (kind, oracle_kind)) in [
            ("inner", reference::JoinKind::Inner),
            ("left", reference::JoinKind::Left),
            ("semi", reference::JoinKind::Semi),
            ("anti", reference::JoinKind::Anti),
        ]
        .into_iter()
        .enumerate()
        {
            let (l, r) = (left.len() as u64, right.len() as u64);
            let nodes = vec![
                values("l", "xs", "x"),
                values("r", "ys", "y"),
                node(
                    "j",
                    "Join",
                    &["l", "r"],
                    json!({"kind":kind,"condition":{
                    "binary":"eq","left":field("x"),"right":field("y")}}),
                ),
            ];
            qualify(
                &db,
                request(
                    nodes,
                    "j",
                    json!({"xs":parameter(&left),"ys":parameter(&right)}),
                ),
                &[("l", 0, l), ("r", 0, r), ("j", l + r, outputs[index])],
                P::Join {
                    left: Box::new(p7_values("x", &left)),
                    right: Box::new(p7_values("y", &right)),
                    on: E::Eq(
                        Box::new(E::Field("x".into())),
                        Box::new(E::Field("y".into())),
                    ),
                    kind: oracle_kind,
                    right_fields: vec!["y".into()],
                },
            );
        }
    }
}

#[test]
fn aggregate_counts_global_empty_and_nullable_groups() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);
    for (input, group_count) in [
        (vec![Some(1), Some(1), None, Some(2)], 3),
        (vec![], 0),
        (vec![None, None], 1),
    ] {
        for grouped in [false, true] {
            let group_by = if grouped {
                json!([{"expression":field("x"),"as":"key"}])
            } else {
                json!([])
            };
            let aggregates: Vec<_> = ["count_all", "count", "sum", "avg", "min", "max", "collect"]
                .into_iter()
                .map(node_aggregate)
                .collect();
            let nodes = vec![
                values("v", "xs", "x"),
                node(
                    "a",
                    "Aggregate",
                    &["v"],
                    json!({"group_by":group_by,"aggregates":aggregates}),
                ),
            ];
            let keys = if grouped {
                vec![("key".into(), E::Field("x".into()))]
            } else {
                vec![]
            };
            let arg = E::Field("x".into());
            let aggregates = vec![
                ("count_all".into(), reference::Aggregate::CountAll),
                ("count".into(), reference::Aggregate::Count(arg.clone())),
                ("sum".into(), reference::Aggregate::Sum(arg.clone())),
                ("avg".into(), reference::Aggregate::Avg(arg.clone())),
                ("min".into(), reference::Aggregate::Min(arg.clone())),
                ("max".into(), reference::Aggregate::Max(arg.clone())),
                ("collect".into(), reference::Aggregate::Collect(arg, 4)),
            ];
            let n = input.len() as u64;
            qualify(
                &db,
                request(nodes, "a", json!({"xs":parameter(&input)})),
                &[("v", 0, n), ("a", n, if grouped { group_count } else { 1 })],
                P::Aggregate {
                    input: Box::new(p7_values("x", &input)),
                    keys,
                    aggregates,
                },
            );
        }
    }
}

fn node_aggregate(name: &str) -> Value {
    let args = if name == "count_all" {
        vec![]
    } else {
        vec![field("x")]
    };
    json!({"expression":{"call":name,"args":args},"as":name})
}

#[test]
fn hql_frontend_reports_the_same_scalar_contract_for_null_bags_and_empty_input() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);
    for (input, distinct) in [
        (vec![Some(2), None, Some(1), Some(2), None], 3),
        (vec![], 0),
    ] {
        let mut hql = request(
            vec![values("v", "xs", "x")],
            "v",
            json!({"xs":parameter(&input)}),
        );
        hql.ir = None;
        hql.language_version = Some(genesis_block_native::uee_v2::HqlLanguageVersionV2::HqlV2);
        hql.hql =
            Some("VALUES $xs AS x |> DISTINCT |> ORDER BY x ASC NULLS LAST |> RETURN x".into());
        let sorted = P::Sort(
            Box::new(P::Distinct(Box::new(p7_values("x", &input)))),
            vec![reference::SortKey {
                expr: E::Field("x".into()),
                descending: false,
                nulls_first: false,
            }],
        );
        qualify(
            &db,
            hql,
            &[
                ("h0", 0, input.len() as u64),
                ("h1", input.len() as u64, distinct),
                ("h2", distinct, distinct),
                ("h3", distinct, distinct),
            ],
            P::Project(Box::new(sorted), vec![("x".into(), E::Field("x".into()))]),
        );
    }
}

#[test]
fn analyze_budget_failures_never_publish_partial_scalar_or_aggregate_results() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);
    let input = [Some(1), Some(2), None];
    let scalar = request(
        vec![values("v", "xs", "x")],
        "v",
        json!({"xs":parameter(&input)}),
    );
    let aggregate = request(
        vec![
            values("v", "xs", "x"),
            node(
                "a",
                "Aggregate",
                &["v"],
                json!({"group_by":[],"aggregates":[node_aggregate("collect")]}),
            ),
        ],
        "a",
        json!({"xs":parameter(&input)}),
    );
    for original in [scalar, aggregate] {
        // First establish that the same query succeeds without the restrictive budget.
        assert!(matches!(
            db.query_v2(access(), original.clone()).unwrap(),
            QueryOutcomeV2::Rows(_)
        ));
        for budget in [json!({"max_memory_bytes":1}), json!({"max_result_bytes":1})] {
            for mode in [None, Some(ExplainV2::Analyze)] {
                let mut limited = original.clone();
                limited.explain = mode;
                limited.budget = Some(serde_json::from_value(budget.clone()).unwrap());
                let error = db.query_v2(access(), limited).unwrap_err();
                assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
            }
        }
    }
    for mode in [None, Some(ExplainV2::Analyze)] {
        let mut limited = request(
            vec![values("v", "xs", "x")],
            "v",
            json!({"xs":parameter(&input)}),
        );
        limited.explain = mode;
        limited.budget = Some(serde_json::from_value(json!({"max_result_rows":1})).unwrap());
        assert_eq!(
            db.query_v2(access(), limited).unwrap_err().code,
            "QUERY_BUDGET_EXCEEDED"
        );
    }
}

fn disk_tree(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path
                .file_name()
                .is_some_and(|name| name == "genesis.lock" || name == "projection.sqlite-shm")
            {
                out.insert(
                    format!("{}:size", path.file_name().unwrap().to_string_lossy()),
                    std::fs::metadata(&path)
                        .unwrap()
                        .len()
                        .to_le_bytes()
                        .to_vec(),
                );
                continue;
            }
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_string_lossy().into(),
                    std::fs::read(&path)
                        .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
                );
            }
        }
    }

    let mut out = BTreeMap::new();
    visit(path, path, &mut out);
    out
}

fn state(db: &Storage, path: &Path) -> (u64, BTreeMap<String, Vec<u8>>) {
    (db.stable_frontier(), disk_tree(path))
}
fn assert_state(db: &Storage, path: &Path, before: &(u64, BTreeMap<String, Vec<u8>>)) {
    assert_eq!(
        db.stable_frontier(),
        before.0,
        "query changed durable frontier"
    );
    assert_eq!(disk_tree(path), before.1, "query changed fixture files");
}

fn scalar_policy(storage: &Storage) {
    storage
        .replace_access_policy(
            PolicyAdminActor {
                access: AccessContext {
                    principal: "local-owner".into(),
                    namespace: "default".into(),
                },
            },
            0,
            AccessPolicy {
                revision: 1,
                mode: AccessPolicyMode::Enforced,
                grants: vec![
                    AccessGrant {
                        principal: "g4-scalar-reader".into(),
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
}

fn scalar_diagnostic_request(
    input: &[Option<i64>],
    skip: usize,
    take: usize,
    hql: bool,
) -> QueryRequestV2 {
    let mut req = request(
        vec![
            values("v", "xs", "x"),
            node(
                "f",
                "Filter",
                &["v"],
                json!({"predicate":{"unary":"not","arg":{"binary":"eq","left":field("x"),"right":{"type":"I64","literal":"0"}}}}),
            ),
            node(
                "p",
                "Project",
                &["f"],
                json!({"fields":[{"expression":field("x"),"as":"y"}]}),
            ),
            node("d", "Distinct", &["p"], json!({})),
            node(
                "s",
                "Sort",
                &["d"],
                json!({"keys":[{"expression":field("y"),"direction":"asc","nulls":"last"}]}),
            ),
            node("o", "Offset", &["s"], json!({"count":skip})),
            node("t", "Take", &["o"], json!({"count":take})),
            node(
                "r",
                "Project",
                &["t"],
                json!({"fields":[{"expression":field("y"),"as":"y"}]}),
            ),
        ],
        "r",
        json!({"xs":parameter(input)}),
    );
    // Parameter-only fixture: one selector shared by every request in this case.
    req.temporal =
        Some(serde_json::from_value(json!({"valid_at":chrono::Utc::now().to_rfc3339()})).unwrap());
    if hql {
        req.ir = None;
        req.language_version = Some(HqlLanguageVersionV2::HqlV2);
        req.hql = Some(format!("VALUES $xs AS x |> FILTER NOT (x = 0) |> PROJECT x AS y |> DISTINCT |> ORDER BY y ASC NULLS LAST |> SKIP {skip} |> TAKE {take} |> RETURN y"));
    }
    req
}

fn scalar_expected_counts(
    plan: &genesis_block_native::query::hql2::ExplainResultV2,
    sizes: &[u64; 8],
) -> Vec<(String, QueryOpV2, u64, u64)> {
    let ops = [
        QueryOpV2::Values,
        QueryOpV2::Filter,
        QueryOpV2::Project,
        QueryOpV2::Distinct,
        QueryOpV2::Sort,
        QueryOpV2::Offset,
        QueryOpV2::Take,
        QueryOpV2::Project,
    ];
    assert_eq!(plan.plan.len(), 8);
    let mut id = plan.root.clone();
    let mut counts = Vec::new();
    for i in (0..8).rev() {
        let n = plan.plan.iter().find(|n| n.id == id).unwrap();
        assert_eq!(n.logical_op, ops[i]);
        counts.push((
            id.clone(),
            ops[i].clone(),
            if i == 0 { 0 } else { sizes[i - 1] },
            sizes[i],
        ));
        if i == 0 {
            assert!(n.inputs.is_empty());
        } else {
            assert_eq!(n.inputs.len(), 1);
            id = n.inputs[0].clone();
        }
    }
    assert_eq!(
        counts
            .iter()
            .map(|(id, _, _, _)| id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        8
    );
    counts
}

/// Selected full scalar pipeline: independent P7 AND literal order, counters,
/// namespace authority, catalog-only Plan and observed state around controls.
#[test]
fn scalar_frontends_preserve_p7_pipeline_counters_authority_and_state() {
    for (input, skip, take, literal, sizes) in [
        (
            vec![Some(2), None, Some(1), Some(2), Some(3), Some(0)],
            1,
            2,
            vec![2, 3],
            [6, 4, 4, 3, 3, 2, 2, 2],
        ),
        (
            vec![Some(2), None, Some(1), Some(2), Some(3), Some(0)],
            1,
            0,
            vec![],
            [6, 4, 4, 3, 3, 2, 0, 0],
        ),
        (vec![], 1, 2, vec![], [0; 8]),
        (vec![None, None], 1, 2, vec![], [2, 0, 0, 0, 0, 0, 0, 0]),
        (
            vec![Some(0), Some(0), None],
            1,
            2,
            vec![],
            [3, 0, 0, 0, 0, 0, 0, 0],
        ),
        (
            vec![Some(1), Some(1), None],
            9,
            2,
            vec![],
            [3, 2, 2, 1, 1, 0, 0, 0],
        ),
        (
            vec![Some(-1), Some(2), Some(-1), None, Some(0)],
            0,
            2,
            vec![-1, 2],
            [5, 3, 3, 2, 2, 2, 2, 2],
        ),
    ] {
        let filtered = P::Filter(
            Box::new(p7_values("x", &input)),
            E::Not(Box::new(E::Eq(
                Box::new(E::Field("x".into())),
                Box::new(E::Literal(R::I64(0))),
            ))),
        );
        let projected = P::Project(Box::new(filtered), vec![("y".into(), E::Field("x".into()))]);
        let sorted = P::Sort(
            Box::new(P::Distinct(Box::new(projected))),
            vec![reference::SortKey {
                expr: E::Field("y".into()),
                descending: false,
                nulls_first: false,
            }],
        );
        let oracle = P::Project(
            Box::new(P::Take(Box::new(P::Offset(Box::new(sorted), skip)), take)),
            vec![("y".into(), E::Field("y".into()))],
        );
        let expected: Vec<BTreeMap<String, V>> = literal
            .into_iter()
            .map(|v| BTreeMap::from([("y".into(), V::I64(v))]))
            .collect();
        let p7: Vec<BTreeMap<String, V>> = reference::execute(&oracle)
            .unwrap()
            .iter()
            .map(|row| row.iter().map(|(k, v)| (k.clone(), from_p7(v))).collect())
            .collect();
        assert_eq!(p7, expected, "independent P7 and literal ordered rows");
        let output_count = expected.len() as u64;
        let directory = TempDir::new().unwrap();
        let storage = open(&directory);
        scalar_policy(&storage);
        let before_plan = state(&storage, directory.path());
        let mut cases = Vec::new();
        let valid_at = chrono::Utc::now().to_rfc3339();
        // Both frontends' allowed/denied Plan controls precede any query execution
        // or explicit publication; observing files/frontier does not publish.
        for hql in [false, true] {
            let mut request = scalar_diagnostic_request(&input, skip, take, hql);
            request.temporal = Some(serde_json::from_value(json!({"valid_at":valid_at})).unwrap());
            let mut planned = request.clone();
            planned.explain = Some(ExplainV2::Plan);
            let QueryOutcomeV2::Plan(plan) = storage.query_v2(access(), planned.clone()).unwrap()
            else {
                panic!("Plan")
            };
            // Fixture policy is revision1; this is the captured catalog frontier,
            // not a generation receipt or an authority value inferred from output.
            assert_eq!(plan.catalog.policy_revision, 1);
            assert_eq!(plan.catalog.observed_frontier, before_plan.0);
            assert!(plan
                .plan
                .iter()
                .all(|node| node.actual.is_none() && node.estimates.is_none()));
            assert_state(&storage, directory.path(), &before_plan);
            let error = storage
                .query_v2(
                    AccessContext {
                        principal: "no-namespace".into(),
                        namespace: "default".into(),
                    },
                    planned,
                )
                .unwrap_err();
            assert_eq!(error.code, "FORBIDDEN");
            assert_eq!(error.stage, "authorize");
            assert_state(&storage, directory.path(), &before_plan);
            cases.push((request, plan));
        }
        // Execution may legitimately publish lazily; establish that boundary once
        // before guarding normal/Analyze errors, never before the Plan control.
        storage.publish_generation().unwrap();
        for (request, plan) in cases {
            let QueryOutcomeV2::Rows(normal) = storage.query_v2(access(), request.clone()).unwrap()
            else {
                panic!("rows")
            };
            assert!(normal.explain.is_none());
            assert_eq!(normal.rows, expected);
            let mut analyzed_request = request.clone();
            analyzed_request.explain = Some(ExplainV2::Analyze);
            let QueryOutcomeV2::Rows(analyzed) =
                storage.query_v2(access(), analyzed_request).unwrap()
            else {
                panic!("Analyze")
            };
            assert_eq!(analyzed.rows, expected);
            assert_eq!(analyzed.columns, normal.columns);
            assert_eq!(analyzed.semantics, normal.semantics);
            assert_eq!(analyzed.snapshot, normal.snapshot);
            assert_eq!(analyzed.completeness, normal.completeness);
            assert_eq!(analyzed.index_frontiers, normal.index_frontiers);
            assert_eq!(analyzed.cursor, normal.cursor);
            let mut actual = analyzed.explain.unwrap();
            assert_eq!(actual.plan_hash, plan.plan_hash);
            assert_eq!(actual.planner_version, plan.planner_version);
            let counts = scalar_expected_counts(&actual, &sizes);
            for (id, op, input, output) in counts {
                let node = actual.plan.iter_mut().find(|node| node.id == id).unwrap();
                assert_eq!(node.logical_op, op);
                let counters = serde_json::to_value(node.actual.take().unwrap()).unwrap();
                for (name, count) in [("rows_in", input), ("rows_out", output)] {
                    assert_eq!(
                        counters[name],
                        json!({"unit":"rows","measurement":{"status":"measured","value":count}})
                    );
                }
                for (name, unit, reason) in [
                    ("bytes_read", "bytes", "not_instrumented"),
                    ("work_units", "work_units", "not_instrumented"),
                    ("memory_peak_bytes", "bytes", "not_instrumented"),
                    ("index_probes", "index_probes", "not_applicable"),
                    (
                        "distance_evaluations",
                        "distance_evaluations",
                        "not_applicable",
                    ),
                    ("graph_expansions", "graph_expansions", "not_applicable"),
                    ("spill_bytes", "bytes", "not_applicable"),
                ] {
                    assert_eq!(
                        counters[name],
                        json!({"unit":unit,"measurement":{"status":"unknown","reason":reason}})
                    );
                }
                assert_eq!(counters["elapsed_ns"]["unit"], "nanoseconds");
                assert_eq!(counters["elapsed_ns"]["measurement"]["status"], "measured");
                assert!(counters["elapsed_ns"]["measurement"]["value"]
                    .as_u64()
                    .is_some());
                assert_eq!(counters["elapsed_scope"], "operator_execution");
                assert_eq!(counters["clock_source"], "monotonic_instant");
                assert_eq!(counters["sampling"], "complete");
            }
            assert_eq!(actual.plan, plan.plan);
            for mode in [None, Some(ExplainV2::Analyze)] {
                let mut denied = request.clone();
                denied.explain = mode.clone();
                let before = state(&storage, directory.path());
                let error = storage
                    .query_v2(
                        AccessContext {
                            principal: "no-namespace".into(),
                            namespace: "default".into(),
                        },
                        denied,
                    )
                    .unwrap_err();
                assert_eq!(error.code, "FORBIDDEN");
                assert_eq!(error.stage, "authorize");
                assert_state(&storage, directory.path(), &before);
                let mut limited = request.clone();
                limited.explain = mode;
                limited.budget =
                    Some(serde_json::from_value(json!({"max_result_bytes":1})).unwrap());
                let before = state(&storage, directory.path());
                let error = storage.query_v2(access(), limited.clone()).unwrap_err();
                assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
                assert_eq!(error.stage, "execute");
                assert_eq!(error.detail.unwrap()["reason"], "query_limit");
                assert_state(&storage, directory.path(), &before);
                if output_count == 0 {
                    // The Storage boundary budgets the complete envelope, not only [].
                    limited.budget =
                        Some(serde_json::from_value(json!({"max_result_bytes":2})).unwrap());
                    let error = storage.query_v2(access(), limited.clone()).unwrap_err();
                    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
                    assert_eq!(error.stage, "execute");
                    assert_eq!(error.detail.unwrap()["reason"], "output_limit");
                    assert_state(&storage, directory.path(), &before);
                    limited.budget =
                        Some(serde_json::from_value(json!({"max_result_bytes":1048576})).unwrap());
                    let limited_mode_is_normal = limited.explain.is_none();
                    let QueryOutcomeV2::Rows(empty) = storage.query_v2(access(), limited).unwrap()
                    else {
                        panic!("empty rows")
                    };
                    assert!(empty.rows.is_empty());
                    assert_eq!(empty.completeness, normal.completeness);
                    assert_eq!(empty.snapshot, normal.snapshot);
                    assert_eq!(empty.columns, normal.columns);
                    assert_eq!(empty.semantics, normal.semantics);
                    assert_eq!(empty.index_frontiers, normal.index_frontiers);
                    assert_eq!(empty.cursor, normal.cursor);
                    if let Some(explain) = empty.explain {
                        assert_eq!(explain.plan_hash, plan.plan_hash);
                        for (id, _, input, output) in scalar_expected_counts(&explain, &sizes) {
                            let n = explain.plan.iter().find(|n| n.id == id).unwrap();
                            let measured =
                                serde_json::to_value(n.actual.as_ref().unwrap()).unwrap();
                            for (name, count) in [("rows_in", input), ("rows_out", output)] {
                                assert_eq!(
                                    measured[name],
                                    json!({"unit":"rows","measurement":{"status":"measured","value":count}})
                                );
                            }
                        }
                    } else {
                        assert!(limited_mode_is_normal);
                    }
                    assert_state(&storage, directory.path(), &before);
                }
            }
        }
    }
}
