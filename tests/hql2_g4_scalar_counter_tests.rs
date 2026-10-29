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

fn arithmetic_request(
    input: &[Option<i64>],
    op: &str,
    rhs: i64,
    hql: bool,
    valid_at: &str,
) -> QueryRequestV2 {
    let mut req = request(
        vec![
            values("v", "xs", "x"),
            node(
                "p",
                "Project",
                &["v"],
                json!({"fields":[{"expression":{"binary":op,"left":field("x"),"right":{"param":"rhs"}},"as":"y"}]}),
            ),
        ],
        "p",
        json!({"xs":parameter(input),"rhs":{"type":"I64","value":rhs.to_string()}}),
    );
    req.temporal = Some(serde_json::from_value(json!({"valid_at":valid_at})).unwrap());
    if hql {
        req.ir = None;
        req.language_version = Some(HqlLanguageVersionV2::HqlV2);
        let symbol = match op {
            "add" => "+",
            "div" => "/",
            "rem" => "%",
            _ => panic!("fixture operator"),
        };
        req.hql = Some(format!("VALUES $xs AS x |> RETURN x {symbol} $rhs AS y"));
    }
    req
}

/// Two-node cardinalities are literal fixture sizes; IDs only locate lineage.
fn assert_arithmetic_counters(
    plan: &mut genesis_block_native::query::hql2::ExplainResultV2,
    size: u64,
) {
    assert_eq!(plan.plan.len(), 2);
    let root = plan.plan.iter().find(|n| n.id == plan.root).unwrap();
    assert_eq!(root.logical_op, QueryOpV2::Project);
    assert_eq!(root.inputs.len(), 1);
    let source = plan.plan.iter().find(|n| n.id == root.inputs[0]).unwrap();
    assert_eq!(source.logical_op, QueryOpV2::Values);
    assert!(source.inputs.is_empty());
    let counts = [(source.id.clone(), 0, size), (root.id.clone(), size, size)];
    assert_ne!(counts[0].0, counts[1].0);
    for (id, input, output) in counts {
        let n = plan.plan.iter_mut().find(|n| n.id == id).unwrap();
        let actual = serde_json::to_value(n.actual.take().unwrap()).unwrap();
        for (name, count) in [("rows_in", input), ("rows_out", output)] {
            assert_eq!(
                actual[name],
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
                actual[name],
                json!({"unit":unit,"measurement":{"status":"unknown","reason":reason}})
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
}

fn qualify_arithmetic_frontends(
    input: &[Option<i64>],
    op: &str,
    rhs: i64,
    expected: Result<Vec<Option<i64>>, (&str, &str)>,
) {
    let lhs = Box::new(E::Field("x".into()));
    let rhs_expr = Box::new(E::Literal(R::I64(rhs)));
    let expression = match op {
        "add" => E::Add(lhs, rhs_expr),
        "div" => E::Div(lhs, rhs_expr),
        "rem" => E::Rem(lhs, rhs_expr),
        _ => panic!("fixture operator"),
    };
    let oracle = reference::execute(&P::Project(
        Box::new(p7_values("x", input)),
        vec![("y".into(), expression)],
    ));
    let expected_rows = match &expected {
        Ok(values) => {
            let literal: Vec<BTreeMap<String, V>> = values
                .iter()
                .map(|v| BTreeMap::from([("y".into(), v.map(V::I64).unwrap_or(V::Null))]))
                .collect();
            let p7: Vec<BTreeMap<String, V>> = oracle
                .unwrap()
                .iter()
                .map(|row| row.iter().map(|(k, v)| (k.clone(), from_p7(v))).collect())
                .collect();
            assert_eq!(
                p7, literal,
                "independent P7 and literal ordered arithmetic rows"
            );
            Some(literal)
        }
        Err((p7_reason, _)) => {
            assert_eq!(oracle.unwrap_err(), *p7_reason);
            None
        }
    };
    let directory = TempDir::new().unwrap();
    let storage = open(&directory);
    scalar_policy(&storage);
    let before_plan = state(&storage, directory.path());
    let valid_at = chrono::Utc::now().to_rfc3339();
    let mut cases = Vec::new();
    // Error operands are well typed: planning must not evaluate arithmetic.
    for hql in [false, true] {
        let request = arithmetic_request(input, op, rhs, hql, &valid_at);
        let mut planned = request.clone();
        planned.explain = Some(ExplainV2::Plan);
        let QueryOutcomeV2::Plan(plan) = storage.query_v2(access(), planned.clone()).unwrap()
        else {
            panic!("Plan must not execute arithmetic")
        };
        assert_eq!(plan.catalog.policy_revision, 1);
        assert_eq!(plan.catalog.observed_frontier, before_plan.0);
        assert_eq!(plan.plan.len(), 2);
        assert!(plan
            .plan
            .iter()
            .all(|n| n.actual.is_none() && n.estimates.is_none()));
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
    // Lazy pinning may publish; establish the generation before guarding errors.
    storage.publish_generation().unwrap();
    for (request, plan) in cases {
        if let Some(rows) = &expected_rows {
            let QueryOutcomeV2::Rows(normal) = storage.query_v2(access(), request.clone()).unwrap()
            else {
                panic!("normal rows")
            };
            assert!(normal.explain.is_none());
            assert_eq!(&normal.rows, rows);
            let mut analyzed = request.clone();
            analyzed.explain = Some(ExplainV2::Analyze);
            let QueryOutcomeV2::Rows(analyzed) = storage.query_v2(access(), analyzed).unwrap()
            else {
                panic!("Analyze rows")
            };
            assert_eq!(&analyzed.rows, rows);
            assert_eq!(analyzed.columns, normal.columns);
            assert_eq!(analyzed.snapshot, normal.snapshot);
            assert_eq!(analyzed.semantics, normal.semantics);
            assert_eq!(analyzed.completeness, normal.completeness);
            assert_eq!(
                normal.completeness.status,
                genesis_block_native::query::hql2::result::CompletionStatusV2::Complete
            );
            assert_eq!(analyzed.index_frontiers, normal.index_frontiers);
            assert_eq!(analyzed.cursor, normal.cursor);
            let mut actual = analyzed.explain.unwrap();
            assert_eq!(actual.plan_hash, plan.plan_hash);
            assert_eq!(actual.planner_version, plan.planner_version);
            assert_arithmetic_counters(&mut actual, input.len() as u64);
            assert_eq!(actual.plan, plan.plan);
        } else {
            let (_, reason) = expected.as_ref().unwrap_err();
            for mode in [None, Some(ExplainV2::Analyze)] {
                let mut failing = request.clone();
                failing.explain = mode;
                let before = state(&storage, directory.path());
                let error = storage.query_v2(access(), failing).unwrap_err();
                // Current published code/stage/reason: no new error taxonomy.
                assert_eq!(error.code, "BIND_ERROR");
                assert_eq!(error.stage, "execute");
                assert_eq!(error.detail.unwrap()["reason"], *reason);
                assert_state(&storage, directory.path(), &before);
            }
        }
        // Authorization wins even when a numeric operand would fail execution.
        for mode in [None, Some(ExplainV2::Analyze)] {
            let mut denied = request.clone();
            denied.explain = mode;
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
        }
    }
}

#[test]
fn checked_arithmetic_frontends_match_p7_and_diagnostics() {
    for (op, rhs, input, expected) in [
        (
            "add",
            3,
            vec![Some(-2), None, Some(0), Some(2), Some(2)],
            vec![Some(1), None, Some(3), Some(5), Some(5)],
        ),
        (
            "div",
            4,
            vec![Some(-7), None, Some(0), Some(7), Some(7)],
            vec![Some(-1), None, Some(0), Some(1), Some(1)],
        ),
        (
            "rem",
            4,
            vec![Some(-7), None, Some(0), Some(7), Some(7)],
            vec![Some(-3), None, Some(0), Some(3), Some(3)],
        ),
    ] {
        qualify_arithmetic_frontends(&input, op, rhs, Ok(expected));
    }
    for op in ["add", "div", "rem"] {
        qualify_arithmetic_frontends(&[], op, 0, Ok(vec![]));
        qualify_arithmetic_frontends(&[None, None], op, 0, Ok(vec![None, None]));
    }
}

#[test]
fn checked_arithmetic_frontends_fail_closed_without_observed_state_change() {
    for (op, input, rhs, p7_reason, reason) in [
        (
            "add",
            vec![Some(1), Some(i64::MAX)],
            1,
            "INTEGER_OVERFLOW",
            "integer_overflow",
        ),
        (
            "div",
            vec![Some(7), Some(i64::MIN)],
            -1,
            "INTEGER_OVERFLOW",
            "integer_overflow",
        ),
        (
            "rem",
            vec![Some(7), Some(i64::MIN)],
            -1,
            "INTEGER_OVERFLOW",
            "integer_overflow",
        ),
        (
            "div",
            vec![None, Some(7)],
            0,
            "DIVISION_BY_ZERO",
            "division_by_zero",
        ),
        (
            "rem",
            vec![None, Some(7)],
            0,
            "DIVISION_BY_ZERO",
            "division_by_zero",
        ),
    ] {
        qualify_arithmetic_frontends(&input, op, rhs, Err((p7_reason, reason)));
    }
}

// Independent expectations: never seed metadata from a prior query result.
fn qualify_metadata_fixture(
    storage: &Storage,
    path: &Path,
    mut request: QueryRequestV2,
    columns: Value,
    expected: Vec<BTreeMap<String, V>>,
    bag: bool,
) {
    use sha2::{Digest, Sha256};
    // Presence-sensitive checks reject missing and accept explicit null.
    assert_ne!(json!({}).get("cursor"), Some(&Value::Null));
    assert_eq!(json!({"cursor":null}).get("cursor"), Some(&Value::Null));
    let mut identity = Sha256::new();
    identity.update(b"genesis.api.v2.database:");
    identity.update(storage.verifying_key.as_bytes());
    let database_id = hex::encode(identity.finalize());
    let valid_at = "2030-01-01T00:00:00+00:00";
    request.temporal = Some(serde_json::from_value(json!({"valid_at":valid_at})).unwrap());
    let before = state(storage, path);
    let mut planned = request.clone();
    planned.explain = Some(ExplainV2::Plan);
    let QueryOutcomeV2::Plan(plan) = storage.query_v2(access(), planned).unwrap() else {
        panic!("Plan")
    };
    assert_eq!(plan.catalog.policy_revision, 1);
    assert_eq!(plan.catalog.observed_frontier, before.0);
    assert_eq!(plan.request_id, request.request_id);
    let root = plan.plan.iter().find(|n| n.id == plan.root).unwrap();
    assert_eq!(serde_json::to_value(&root.columns).unwrap(), columns);
    assert!(plan
        .plan
        .iter()
        .all(|n| n.actual.is_none() && n.estimates.is_none()));
    assert_state(storage, path, &before);
    // Publication is explicit setup, not another query or a pre-Plan receipt.
    let generation = storage.publish_generation().unwrap();
    // Publication appends a control event beyond the selected generation tx.
    let execution_catalog_frontier = storage.stable_frontier();
    request.temporal = Some(
        serde_json::from_value(
            json!({"valid_at":valid_at,"tx_as_of":generation.wal_frontier.to_string()}),
        )
        .unwrap(),
    );
    let snapshot = json!({"database_id":database_id,"tx":generation.wal_frontier.to_string(),"valid_at":valid_at,"catalog_generation":generation.generation_id.to_string(),"policy_version":generation.acl_revision.to_string()});
    for mode in [None, Some(ExplainV2::Analyze)] {
        request.explain = mode.clone();
        let QueryOutcomeV2::Rows(result) = storage.query_v2(access(), request.clone()).unwrap()
        else {
            panic!("rows")
        };
        let actual = serde_json::to_value(&result).unwrap();
        assert_eq!(actual["request_id"], request.request_id);
        assert_eq!(actual["columns"], columns);
        assert_eq!(actual["snapshot"], snapshot);
        assert_eq!(
            actual["semantics"],
            json!({"candidate_search":"exact","distance_fidelity":"not_applicable","index_coverage":"not_applicable","scope":"whole_input"})
        );
        assert_eq!(
            actual["completeness"],
            json!({"status":"complete","reason":null,"eligible_count_known":true})
        );
        assert_eq!(actual["index_frontiers"], json!({}));
        assert_eq!(actual.get("cursor"), Some(&Value::Null));
        if bag {
            let sorted = |rows: &[BTreeMap<String, V>]| {
                let mut rows: Vec<_> = rows
                    .iter()
                    .map(|r| serde_json::to_string(r).unwrap())
                    .collect();
                rows.sort();
                rows
            };
            assert_eq!(sorted(&result.rows), sorted(&expected));
        } else {
            assert_eq!(result.rows, expected);
        }
        if mode.is_none() {
            assert!(result.explain.is_none());
        } else {
            let explain = result.explain.unwrap();
            assert_eq!(explain.catalog.policy_revision, 1);
            assert_eq!(
                explain.catalog.observed_frontier,
                execution_catalog_frontier
            );
            for node in explain.plan {
                assert!(node.estimates.is_none());
                let counters = serde_json::to_value(node.actual.unwrap()).unwrap();
                for name in ["rows_in", "rows_out", "elapsed_ns"] {
                    assert_eq!(counters[name]["measurement"]["status"], "measured");
                    assert!(counters[name]["measurement"]["value"].as_u64().is_some());
                }
                assert_eq!(counters["elapsed_ns"]["unit"], "nanoseconds");
                assert_eq!(counters["elapsed_scope"], "operator_execution");
                assert_eq!(counters["clock_source"], "monotonic_instant");
                assert_eq!(counters["sampling"], "complete");
            }
        }
    }
}

#[test]
fn scalar_result_metadata_has_independent_fixture_expectations() {
    for (case, input, take, literal) in [
        (
            "nullable",
            vec![Some(2), None, Some(1), Some(2), None],
            9,
            vec![V::I64(1), V::I64(2), V::Null],
        ),
        ("empty", vec![], 9, vec![]),
        ("all-null", vec![None, None], 9, vec![V::Null]),
        ("take-zero", vec![Some(2), None, Some(1)], 0, vec![]),
    ] {
        let oracle = P::Take(
            Box::new(P::Offset(
                Box::new(P::Sort(
                    Box::new(P::Distinct(Box::new(P::Project(
                        Box::new(p7_values("x", &input)),
                        vec![("y".into(), E::Field("x".into()))],
                    )))),
                    vec![reference::SortKey {
                        expr: E::Field("y".into()),
                        descending: false,
                        nulls_first: false,
                    }],
                )),
                0,
            )),
            take,
        );
        let expected: Vec<BTreeMap<String, V>> = literal
            .into_iter()
            .map(|v| BTreeMap::from([("y".into(), v)]))
            .collect();
        let oracle: Vec<BTreeMap<String, V>> = reference::execute(&oracle)
            .unwrap()
            .iter()
            .map(|row| row.iter().map(|(k, v)| (k.clone(), from_p7(v))).collect())
            .collect();
        assert_eq!(oracle, expected);
        for hql in [false, true] {
            let directory = TempDir::new().unwrap();
            let storage = open(&directory);
            scalar_policy(&storage);
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
                node("o", "Offset", &["s"], json!({"count":0})),
                node("t", "Take", &["o"], json!({"count":take})),
                node(
                    "r",
                    "Project",
                    &["t"],
                    json!({"fields":[{"expression":field("y"),"as":"y"}]}),
                ),
            ];
            let mut req = request(nodes, "r", json!({"xs":parameter(&input)}));
            req.request_id = format!("metadata-scalar-{case}-{hql}");
            if hql {
                req.ir = None;
                req.language_version = Some(HqlLanguageVersionV2::HqlV2);
                req.hql=Some(format!("VALUES $xs AS x |> PROJECT x AS y |> DISTINCT |> ORDER BY y ASC NULLS LAST |> SKIP 0 |> TAKE {take} |> RETURN y"));
            }
            qualify_metadata_fixture(
                &storage,
                directory.path(),
                req,
                json!([{"name":"y","type":"Nullable<I64>","nullable":true}]),
                expected.clone(),
                false,
            );
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum B1RowKernel {
    Values,
    Project,
    Distinct,
    Sort,
    Take,
    Offset,
}

struct B1ValueCase {
    type_name: &'static str,
    encoded_rows: Value,
    literal_values: Vec<V>,
    p7_values: Option<Vec<R>>,
    nullable: bool,
    ordered: bool,
}

fn b1_hql_request(params: Value, hql: &str) -> QueryRequestV2 {
    let mut req = request(Vec::new(), "", params);
    req.ir = None;
    req.language_version = Some(HqlLanguageVersionV2::HqlV2);
    req.hql = Some(hql.to_owned());
    req
}

fn b1_value_kernel_requests(
    type_name: &str,
    encoded_rows: Value,
    kernel: B1RowKernel,
) -> (QueryRequestV2, QueryRequestV2, String, String) {
    let params = json!({
        "xs": {
            "type": format!("List<{type_name}>"),
            "value": encoded_rows
        }
    });
    let mut nodes = vec![values("v", "xs", "x")];
    let (root, alias, hql) = match kernel {
        B1RowKernel::Values => ("v", "x", "VALUES $xs AS x |> RETURN x".to_owned()),
        B1RowKernel::Project => {
            nodes.push(node(
                "p",
                "Project",
                &["v"],
                json!({"fields":[{"expression":field("x"),"as":"y"}]}),
            ));
            (
                "p",
                "y",
                "VALUES $xs AS x |> PROJECT x AS y |> RETURN y".to_owned(),
            )
        }
        B1RowKernel::Distinct => {
            nodes.push(node("d", "Distinct", &["v"], json!({})));
            (
                "d",
                "x",
                "VALUES $xs AS x |> DISTINCT |> RETURN x".to_owned(),
            )
        }
        B1RowKernel::Sort => {
            nodes.push(node(
                "s",
                "Sort",
                &["v"],
                json!({"keys":[{"expression":field("x"),"direction":"asc","nulls":"last"}]}),
            ));
            (
                "s",
                "x",
                "VALUES $xs AS x |> ORDER BY x ASC NULLS LAST |> RETURN x".to_owned(),
            )
        }
        B1RowKernel::Take => {
            nodes.push(node("t", "Take", &["v"], json!({"count":10})));
            (
                "t",
                "x",
                "VALUES $xs AS x |> TAKE 10 |> RETURN x".to_owned(),
            )
        }
        B1RowKernel::Offset => {
            nodes.push(node("o", "Offset", &["v"], json!({"count":0})));
            ("o", "x", "VALUES $xs AS x |> SKIP 0 |> RETURN x".to_owned())
        }
    };
    (
        request(nodes, root, params.clone()),
        b1_hql_request(params, &hql),
        alias.to_owned(),
        hql,
    )
}

fn b1_project_requests(
    params: Value,
    ir_expression: Value,
    hql_expression: &str,
) -> (QueryRequestV2, QueryRequestV2) {
    let ir = request(
        vec![
            values("v", "xs", "x"),
            node(
                "p",
                "Project",
                &["v"],
                json!({"fields":[{"expression":ir_expression,"as":"y"}]}),
            ),
        ],
        "p",
        params.clone(),
    );
    let hql = b1_hql_request(
        params,
        &format!("VALUES $xs AS x |> RETURN {hql_expression} AS y"),
    );
    (ir, hql)
}

fn b1_filter_requests(
    params: Value,
    ir_predicate: Value,
    hql_predicate: &str,
) -> (QueryRequestV2, QueryRequestV2) {
    let ir = request(
        vec![
            values("v", "xs", "x"),
            node("f", "Filter", &["v"], json!({"predicate":ir_predicate})),
        ],
        "f",
        params.clone(),
    );
    let hql = b1_hql_request(
        params,
        &format!("VALUES $xs AS x |> FILTER {hql_predicate} |> RETURN x"),
    );
    (ir, hql)
}

fn b1_from_p7(value: &R) -> V {
    match value {
        R::Null => V::Null,
        R::Bool(value) => V::Bool(*value),
        R::I64(value) => V::I64(*value),
        R::F64(value) => V::F64(*value),
        R::Text(value) => V::Utf8(value.clone()),
        R::List(values) => V::List(values.iter().map(b1_from_p7).collect()),
        other => panic!("P7 scalar-type case used an unsupported fixture value: {other:?}"),
    }
}

fn b1_literal_rows(alias: &str, values: Vec<V>) -> Vec<BTreeMap<String, V>> {
    values
        .into_iter()
        .map(|value| BTreeMap::from([(alias.to_owned(), value)]))
        .collect()
}

fn b1_p7_values(alias: &str, values: Vec<R>) -> P {
    P::Values(
        values
            .into_iter()
            .map(|value| BTreeMap::from([(alias.to_owned(), value)]))
            .collect(),
    )
}

fn b1_p7_kernel(kernel: B1RowKernel, values: Vec<R>) -> P {
    let rows = values.len();
    let plan = b1_p7_values("x", values);
    match kernel {
        B1RowKernel::Values => plan,
        B1RowKernel::Project => {
            P::Project(Box::new(plan), vec![("y".into(), E::Field("x".into()))])
        }
        B1RowKernel::Distinct => P::Distinct(Box::new(plan)),
        B1RowKernel::Sort => P::Sort(
            Box::new(plan),
            vec![reference::SortKey {
                expr: E::Field("x".into()),
                descending: false,
                nulls_first: false,
            }],
        ),
        B1RowKernel::Take => P::Take(Box::new(plan), rows.max(1)),
        B1RowKernel::Offset => P::Offset(Box::new(plan), 0),
    }
}

fn b1_assert_rows(
    db: &Storage,
    ir_request: QueryRequestV2,
    hql_request: QueryRequestV2,
    expected_columns: Value,
    expected_rows: Vec<BTreeMap<String, V>>,
    p7: Option<P>,
) {
    let QueryOutcomeV2::Rows(ir) = db.query_v2(access(), ir_request).unwrap() else {
        panic!("typed IR request must return rows");
    };
    let QueryOutcomeV2::Rows(hql) = db.query_v2(access(), hql_request).unwrap() else {
        panic!("HQL request must return rows");
    };
    assert!(ir.explain.is_none());
    assert!(hql.explain.is_none());
    assert_eq!(serde_json::to_value(&ir.columns).unwrap(), expected_columns);
    assert_eq!(
        serde_json::to_value(&hql.columns).unwrap(),
        expected_columns
    );
    assert_eq!(ir.rows, expected_rows);
    assert_eq!(hql.rows, expected_rows);
    assert_eq!(ir.rows, hql.rows);
    if let Some(plan) = p7 {
        let expected_p7 = reference::execute(&plan)
            .unwrap()
            .iter()
            .map(|row| {
                row.iter()
                    .map(|(name, value)| (name.clone(), b1_from_p7(value)))
                    .collect::<BTreeMap<_, _>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(expected_p7, expected_rows);
    }
}

fn b1_assert_error(db: &Storage, req: QueryRequestV2, code: &str, stage: &str, reason: &str) {
    let error = db.query_v2(access(), req).unwrap_err();
    assert_eq!(error.code, code);
    assert_eq!(error.stage, stage);
    assert_eq!(error.detail.unwrap()["reason"], reason);
}

fn b1_assert_paired_error(
    db: &Storage,
    ir_request: QueryRequestV2,
    hql_request: QueryRequestV2,
    code: &str,
    stage: &str,
    reason: &str,
) {
    b1_assert_error(db, ir_request, code, stage, reason);
    b1_assert_error(db, hql_request, code, stage, reason);
}

fn c1_sort_key(expression: Value, direction: &str, nulls: &str) -> Value {
    json!({"expression":expression,"direction":direction,"nulls":nulls})
}

fn c1_sort_requests(
    params: Value,
    keys: Vec<Value>,
    hql_order: &str,
) -> (QueryRequestV2, QueryRequestV2) {
    let ir = request(
        vec![
            values("v", "xs", "x"),
            node("s", "Sort", &["v"], json!({"keys":keys})),
            node(
                "p",
                "Project",
                &["s"],
                json!({"fields":[{"expression":field("x"),"as":"y"}]}),
            ),
        ],
        "p",
        params.clone(),
    );
    let hql = b1_hql_request(
        params,
        &format!("VALUES $xs AS x |> ORDER BY {hql_order} |> RETURN x AS y"),
    );
    (ir, hql)
}

fn c1_pagination_requests(params: Value, skip: u64, take: u64) -> (QueryRequestV2, QueryRequestV2) {
    let ir = request(
        vec![
            values("v", "xs", "x"),
            node(
                "p0",
                "Project",
                &["v"],
                json!({"fields":[{"expression":field("x"),"as":"y"}]}),
            ),
            node(
                "s",
                "Sort",
                &["p0"],
                json!({"keys":[c1_sort_key(field("y"),"asc","last")]}),
            ),
            node("o", "Offset", &["s"], json!({"count":skip})),
            node("t", "Take", &["o"], json!({"count":take})),
            node(
                "p1",
                "Project",
                &["t"],
                json!({"fields":[{"expression":field("y"),"as":"y"}]}),
            ),
        ],
        "p1",
        params.clone(),
    );
    let hql = b1_hql_request(
        params,
        &format!(
            "VALUES $xs AS x |> PROJECT x AS y |> ORDER BY y ASC NULLS LAST |> SKIP {skip} |> TAKE {take} |> RETURN y"
        ),
    );
    (ir, hql)
}

fn c1_p7_sort_project(values: Vec<R>, keys: Vec<reference::SortKey>) -> P {
    P::Project(
        Box::new(P::Sort(Box::new(b1_p7_values("x", values)), keys)),
        vec![("y".into(), E::Field("x".into()))],
    )
}

fn c1_assert_plan_nodes(
    explain: &genesis_block_native::query::hql2::ExplainResultV2,
    expected: &[(QueryOpV2, u64, u64)],
    analyzed: bool,
) {
    assert_eq!(explain.plan.len(), expected.len());
    let mut id = explain.root.clone();
    for (index, (op, rows_in, rows_out)) in expected.iter().enumerate().rev() {
        let node = explain.plan.iter().find(|node| node.id == id).unwrap();
        assert_eq!(&node.logical_op, op);
        if index == 0 {
            assert!(node.inputs.is_empty());
        } else {
            assert_eq!(node.inputs.len(), 1);
            id = node.inputs[0].clone();
        }
        if !analyzed {
            assert!(node.actual.is_none());
            assert!(node.estimates.is_none());
            continue;
        }

        let actual = serde_json::to_value(node.actual.as_ref().unwrap()).unwrap();
        for (name, count) in [("rows_in", rows_in), ("rows_out", rows_out)] {
            assert_eq!(
                actual[name],
                json!({"unit":"rows","measurement":{"status":"measured","value":count}}),
                "{op:?}.{name}"
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
                "{op:?}.{name}"
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
}

struct C1OrderExpectation<'a> {
    type_name: &'a str,
    nullable: bool,
    values: Vec<V>,
    p7: Option<P>,
    ops: &'a [(QueryOpV2, u64, u64)],
}

fn c1_assert_order_modes(
    db: &Storage,
    ir_request: QueryRequestV2,
    hql_request: QueryRequestV2,
    expected: C1OrderExpectation<'_>,
) {
    let expected_columns =
        json!([{"name":"y","type":expected.type_name,"nullable":expected.nullable}]);
    let expected_rows = b1_literal_rows("y", expected.values);
    if let Some(oracle) = expected.p7 {
        let p7_rows = reference::execute(&oracle)
            .unwrap()
            .iter()
            .map(|row| {
                row.iter()
                    .map(|(name, value)| (name.clone(), b1_from_p7(value)))
                    .collect::<BTreeMap<_, _>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(p7_rows, expected_rows, "literal C1 rows vs unchanged P7");
    }

    for (frontend, request) in [("typed IR", ir_request), ("HQL2", hql_request)] {
        let QueryOutcomeV2::Rows(normal) = db.query_v2(access(), request.clone()).unwrap() else {
            panic!("{frontend} normal execution must return rows");
        };
        assert!(normal.explain.is_none());
        assert_eq!(
            serde_json::to_value(&normal.columns).unwrap(),
            expected_columns
        );
        assert_eq!(
            normal.rows, expected_rows,
            "{frontend} literal ordered rows"
        );
        assert_eq!(
            normal.completeness.status,
            genesis_block_native::query::hql2::result::CompletionStatusV2::Complete
        );

        let mut planned_request = request.clone();
        planned_request.explain = Some(ExplainV2::Plan);
        let QueryOutcomeV2::Plan(plan) = db.query_v2(access(), planned_request).unwrap() else {
            panic!("{frontend} Plan must return a plan");
        };
        c1_assert_plan_nodes(&plan, expected.ops, false);

        let mut analyzed_request = request;
        analyzed_request.explain = Some(ExplainV2::Analyze);
        let QueryOutcomeV2::Rows(analyzed) = db.query_v2(access(), analyzed_request).unwrap()
        else {
            panic!("{frontend} Analyze must return rows");
        };
        assert_eq!(analyzed.rows, expected_rows, "{frontend} Analyze rows");
        assert_eq!(
            serde_json::to_value(&analyzed.columns).unwrap(),
            expected_columns
        );
        assert_eq!(normal.rows, analyzed.rows);
        assert_eq!(normal.columns, analyzed.columns);
        assert_eq!(normal.semantics, analyzed.semantics);
        assert_eq!(normal.completeness, analyzed.completeness);
        assert_eq!(normal.index_frontiers, analyzed.index_frontiers);
        assert_eq!(normal.cursor, analyzed.cursor);

        let mut analyzed_plan = analyzed.explain.unwrap();
        assert_eq!(analyzed_plan.plan_hash, plan.plan_hash);
        assert_eq!(analyzed_plan.planner_version, plan.planner_version);
        c1_assert_plan_nodes(&analyzed_plan, expected.ops, true);
        for node in &mut analyzed_plan.plan {
            node.actual.take();
        }
        assert_eq!(analyzed_plan.plan, plan.plan);
    }
}

/// Contract-derived row-shaping admission only: every successful cell has a
/// literal schema/row oracle, with the unchanged P7 interpreter where it has an
/// independent representation. Sort inputs have at most one row so this does
/// not qualify ASC/DESC, NULL placement, tie, or multi-key ordering semantics.
#[test]
fn scalar_type_admissibility_matrix_accepts_row_shaping_kernels() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);
    let cases = vec![
        B1ValueCase {
            type_name: "Bool",
            encoded_rows: json!([true]),
            literal_values: vec![V::Bool(true)],
            p7_values: Some(vec![R::Bool(true)]),
            nullable: false,
            ordered: true,
        },
        B1ValueCase {
            type_name: "I64",
            encoded_rows: json!(["-9223372036854775808"]),
            literal_values: vec![V::I64(i64::MIN)],
            p7_values: Some(vec![R::I64(i64::MIN)]),
            nullable: false,
            ordered: true,
        },
        B1ValueCase {
            type_name: "I64",
            encoded_rows: json!(["9223372036854775807"]),
            literal_values: vec![V::I64(i64::MAX)],
            p7_values: Some(vec![R::I64(i64::MAX)]),
            nullable: false,
            ordered: true,
        },
        B1ValueCase {
            type_name: "DecimalU64",
            encoded_rows: json!(["0"]),
            literal_values: vec![V::DecimalU64(0)],
            p7_values: None,
            nullable: false,
            ordered: true,
        },
        B1ValueCase {
            type_name: "DecimalU64",
            encoded_rows: json!(["18446744073709551615"]),
            literal_values: vec![V::DecimalU64(u64::MAX)],
            p7_values: None,
            nullable: false,
            ordered: true,
        },
        B1ValueCase {
            type_name: "F64Finite",
            encoded_rows: json!([1.25]),
            literal_values: vec![V::F64(1.25)],
            p7_values: Some(vec![R::F64(1.25)]),
            nullable: false,
            ordered: true,
        },
        B1ValueCase {
            type_name: "F64Finite",
            encoded_rows: json!([f64::MAX]),
            literal_values: vec![V::F64(f64::MAX)],
            p7_values: Some(vec![R::F64(f64::MAX)]),
            nullable: false,
            ordered: true,
        },
        B1ValueCase {
            type_name: "Utf8",
            encoded_rows: json!(["λ"]),
            literal_values: vec![V::Utf8("λ".into())],
            p7_values: Some(vec![R::Text("λ".into())]),
            nullable: false,
            ordered: true,
        },
        B1ValueCase {
            type_name: "Nullable<I64>",
            encoded_rows: json!([null]),
            literal_values: vec![V::Null],
            p7_values: Some(vec![R::Null]),
            nullable: true,
            ordered: true,
        },
        B1ValueCase {
            type_name: "Nullable<I64>",
            encoded_rows: json!(["9"]),
            literal_values: vec![V::I64(9)],
            p7_values: Some(vec![R::I64(9)]),
            nullable: true,
            ordered: true,
        },
        B1ValueCase {
            type_name: "Nullable<Bool>",
            encoded_rows: json!([true, null]),
            literal_values: vec![V::Bool(true), V::Null],
            p7_values: Some(vec![R::Bool(true), R::Null]),
            nullable: true,
            ordered: true,
        },
        B1ValueCase {
            type_name: "Nullable<DecimalU64>",
            encoded_rows: json!([null, "18446744073709551615"]),
            literal_values: vec![V::Null, V::DecimalU64(u64::MAX)],
            p7_values: None,
            nullable: true,
            ordered: true,
        },
        B1ValueCase {
            type_name: "Nullable<F64Finite>",
            encoded_rows: json!([null, 1.25]),
            literal_values: vec![V::Null, V::F64(1.25)],
            p7_values: Some(vec![R::Null, R::F64(1.25)]),
            nullable: true,
            ordered: true,
        },
        B1ValueCase {
            type_name: "Nullable<Utf8>",
            encoded_rows: json!([null, "λ"]),
            literal_values: vec![V::Null, V::Utf8("λ".into())],
            p7_values: Some(vec![R::Null, R::Text("λ".into())]),
            nullable: true,
            ordered: true,
        },
        B1ValueCase {
            type_name: "Nullable<Vector<g4-b1-space,2,f64>>",
            encoded_rows: json!([null, [0.5, 1.5]]),
            literal_values: vec![
                V::Null,
                V::Vector(genesis_block_native::query::hql2::value::VectorValueV2 {
                    space_id: "g4-b1-space".into(),
                    scalar: genesis_block_native::query::hql2::value::VectorScalarV2::F64,
                    values: vec![0.5, 1.5],
                }),
            ],
            p7_values: None,
            nullable: true,
            ordered: false,
        },
        B1ValueCase {
            type_name: "List<I64>",
            encoded_rows: json!([["7", "8"]]),
            literal_values: vec![V::List(vec![V::I64(7), V::I64(8)])],
            p7_values: Some(vec![R::List(vec![R::I64(7), R::I64(8)])]),
            nullable: false,
            ordered: false,
        },
        B1ValueCase {
            type_name: "List<Nullable<I64>>",
            encoded_rows: json!([[null, "9"]]),
            literal_values: vec![V::List(vec![V::Null, V::I64(9)])],
            p7_values: Some(vec![R::List(vec![R::Null, R::I64(9)])]),
            nullable: false,
            ordered: false,
        },
        B1ValueCase {
            type_name: "List<List<I64>>",
            encoded_rows: json!([[["1", "2"], []]]),
            literal_values: vec![V::List(vec![
                V::List(vec![V::I64(1), V::I64(2)]),
                V::List(vec![]),
            ])],
            p7_values: Some(vec![R::List(vec![
                R::List(vec![R::I64(1), R::I64(2)]),
                R::List(vec![]),
            ])]),
            nullable: false,
            ordered: false,
        },
        B1ValueCase {
            type_name: "Nullable<List<I64>>",
            encoded_rows: json!([null]),
            literal_values: vec![V::Null],
            p7_values: Some(vec![R::Null]),
            nullable: true,
            ordered: false,
        },
        B1ValueCase {
            type_name: "Vector<g4-b1-space,2,f64>",
            encoded_rows: json!([[0.5, 1.5]]),
            literal_values: vec![V::Vector(
                genesis_block_native::query::hql2::value::VectorValueV2 {
                    space_id: "g4-b1-space".into(),
                    scalar: genesis_block_native::query::hql2::value::VectorScalarV2::F64,
                    values: vec![0.5, 1.5],
                },
            )],
            p7_values: None,
            nullable: false,
            ordered: false,
        },
        B1ValueCase {
            type_name: "Vector<g4-b1-space,2,f32>",
            encoded_rows: json!([[0.5, 1.5]]),
            literal_values: vec![V::Vector(
                genesis_block_native::query::hql2::value::VectorValueV2 {
                    space_id: "g4-b1-space".into(),
                    scalar: genesis_block_native::query::hql2::value::VectorScalarV2::F32,
                    values: vec![0.5, 1.5],
                },
            )],
            p7_values: None,
            nullable: false,
            ordered: false,
        },
        B1ValueCase {
            type_name: "I64",
            encoded_rows: json!([]),
            literal_values: vec![],
            p7_values: Some(vec![]),
            nullable: false,
            ordered: true,
        },
    ];

    for case in cases {
        let kernels = vec![
            B1RowKernel::Values,
            B1RowKernel::Project,
            B1RowKernel::Distinct,
            B1RowKernel::Take,
            B1RowKernel::Offset,
        ];
        for kernel in kernels {
            let (ir, hql, alias, _query) =
                b1_value_kernel_requests(case.type_name, case.encoded_rows.clone(), kernel);
            let expected_rows = b1_literal_rows(&alias, case.literal_values.clone());
            let p7 = case
                .p7_values
                .clone()
                .map(|values| b1_p7_kernel(kernel, values));
            b1_assert_rows(
                &db,
                ir,
                hql,
                json!([{"name":alias,"type":case.type_name,"nullable":case.nullable}]),
                expected_rows,
                p7,
            );
        }
        if case.ordered {
            // Sort is qualified for type admission only. Restrict its input to
            // at most one row so no ordering/null-placement rule is implied.
            let mut encoded_rows = case.encoded_rows.clone();
            if let Some(rows) = encoded_rows.as_array_mut() {
                rows.truncate(1);
            }
            let literal_values = case
                .literal_values
                .first()
                .cloned()
                .into_iter()
                .collect::<Vec<_>>();
            let p7_values = case
                .p7_values
                .clone()
                .map(|values| values.into_iter().take(1).collect::<Vec<_>>());
            let (ir, hql, alias, _) =
                b1_value_kernel_requests(case.type_name, encoded_rows, B1RowKernel::Sort);
            b1_assert_rows(
                &db,
                ir,
                hql,
                json!([{"name":alias,"type":case.type_name,"nullable":case.nullable}]),
                b1_literal_rows(&alias, literal_values),
                p7_values.map(|values| b1_p7_kernel(B1RowKernel::Sort, values)),
            );
        }
    }
}

/// Same-type comparison admission is pinned independently for each ordered
/// scalar. P7 supplies only the equality cells it actually implements; literal
/// expected rows remain the primary oracle for every operator.
#[test]
fn scalar_type_admissibility_matrix_filters_and_compares() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);

    for (type_name, encoded, p7_values, expected) in [
        (
            "Bool",
            json!([true, false]),
            vec![R::Bool(true), R::Bool(false)],
            vec![V::Bool(true)],
        ),
        (
            "Nullable<Bool>",
            json!([true, false, null]),
            vec![R::Bool(true), R::Bool(false), R::Null],
            vec![V::Bool(true)],
        ),
    ] {
        let params = json!({"xs":{"type":format!("List<{type_name}>"),"value":encoded}});
        let (ir, hql) = b1_filter_requests(params, field("x"), "x");
        let p7 = P::Filter(Box::new(b1_p7_values("x", p7_values)), E::Field("x".into()));
        b1_assert_rows(
            &db,
            ir,
            hql,
            json!([{"name":"x","type":type_name,"nullable":type_name.starts_with("Nullable<")}]),
            b1_literal_rows("x", expected),
            Some(p7),
        );
    }

    let nullable_bool = json!({
        "xs":{"type":"List<Nullable<Bool>>","value":[true,false,null]},
        "rhs":{"type":"Bool","value":true}
    });
    for (operation, symbol, expected) in [
        ("and", "AND", vec![V::Bool(true), V::Bool(false), V::Null]),
        (
            "or",
            "OR",
            vec![V::Bool(true), V::Bool(true), V::Bool(true)],
        ),
    ] {
        let (ir, hql) = b1_project_requests(
            nullable_bool.clone(),
            json!({"binary":operation,"left":field("x"),"right":{"param":"rhs"}}),
            &format!("x {symbol} $rhs"),
        );
        let oracle_op = if operation == "and" {
            E::And(
                Box::new(E::Field("x".into())),
                Box::new(E::Literal(R::Bool(true))),
            )
        } else {
            E::Or(
                Box::new(E::Field("x".into())),
                Box::new(E::Literal(R::Bool(true))),
            )
        };
        b1_assert_rows(
            &db,
            ir,
            hql,
            json!([{"name":"y","type":"Nullable<Bool>","nullable":true}]),
            b1_literal_rows("y", expected),
            Some(P::Project(
                Box::new(b1_p7_values(
                    "x",
                    vec![R::Bool(true), R::Bool(false), R::Null],
                )),
                vec![("y".into(), oracle_op)],
            )),
        );
    }
    let (ir, hql) = b1_project_requests(
        json!({"xs":{"type":"List<Nullable<Bool>>","value":[true,false,null]}}),
        json!({"unary":"not","arg":field("x")}),
        "NOT x",
    );
    b1_assert_rows(
        &db,
        ir,
        hql,
        json!([{"name":"y","type":"Nullable<Bool>","nullable":true}]),
        b1_literal_rows("y", vec![V::Bool(false), V::Bool(true), V::Null]),
        Some(P::Project(
            Box::new(b1_p7_values(
                "x",
                vec![R::Bool(true), R::Bool(false), R::Null],
            )),
            vec![("y".into(), E::Not(Box::new(E::Field("x".into()))))],
        )),
    );

    struct ComparisonCase {
        type_name: &'static str,
        encoded: Value,
        rhs_type: &'static str,
        rhs_value: Value,
        rhs_hql: &'static str,
        p7_values: Option<Vec<R>>,
        expected: [&'static [bool]; 6],
    }
    let cases = [
        ComparisonCase {
            type_name: "Bool",
            encoded: json!([false, true]),
            rhs_type: "Bool",
            rhs_value: json!(true),
            rhs_hql: "true",
            p7_values: Some(vec![R::Bool(false), R::Bool(true)]),
            expected: [
                &[false, true],
                &[true, false],
                &[true, false],
                &[true, true],
                &[false, false],
                &[false, true],
            ],
        },
        ComparisonCase {
            type_name: "I64",
            encoded: json!(["-9223372036854775808", "9223372036854775807"]),
            rhs_type: "I64",
            rhs_value: json!("9223372036854775807"),
            rhs_hql: "9223372036854775807",
            p7_values: Some(vec![R::I64(i64::MIN), R::I64(i64::MAX)]),
            expected: [
                &[false, true],
                &[true, false],
                &[true, false],
                &[true, true],
                &[false, false],
                &[false, true],
            ],
        },
        ComparisonCase {
            type_name: "DecimalU64",
            encoded: json!(["0", "18446744073709551615"]),
            rhs_type: "DecimalU64",
            rhs_value: json!("18446744073709551615"),
            rhs_hql: "$rhs",
            p7_values: None,
            expected: [
                &[false, true],
                &[true, false],
                &[true, false],
                &[true, true],
                &[false, false],
                &[false, true],
            ],
        },
        ComparisonCase {
            type_name: "F64Finite",
            encoded: json!([1.25, 2.5]),
            rhs_type: "F64Finite",
            rhs_value: json!(2.0),
            rhs_hql: "2.0",
            p7_values: Some(vec![R::F64(1.25), R::F64(2.5)]),
            expected: [
                &[false, false],
                &[true, true],
                &[true, false],
                &[true, false],
                &[false, true],
                &[false, true],
            ],
        },
        ComparisonCase {
            type_name: "Utf8",
            encoded: json!(["λ", "東京"]),
            rhs_type: "Utf8",
            rhs_value: json!("μ"),
            rhs_hql: "\"μ\"",
            p7_values: Some(vec![R::Text("λ".into()), R::Text("東京".into())]),
            expected: [
                &[false, false],
                &[true, true],
                &[true, false],
                &[true, false],
                &[false, true],
                &[false, true],
            ],
        },
    ];
    let operations = [
        ("eq", "="),
        ("ne", "!="),
        ("lt", "<"),
        ("le", "<="),
        ("gt", ">"),
        ("ge", ">="),
    ];
    for case in cases {
        for (op_index, (operator, symbol)) in operations.into_iter().enumerate() {
            let params = json!({
                "xs":{"type":format!("List<{}>",case.type_name),"value":case.encoded.clone()},
                "rhs":{"type":case.rhs_type,"value":case.rhs_value.clone()}
            });
            let ir_expr = json!({"binary":operator,"left":field("x"),"right":{"param":"rhs"}});
            let (ir, hql) =
                b1_project_requests(params, ir_expr, &format!("x {symbol} {}", case.rhs_hql));
            let expected_rows = case.expected[op_index]
                .iter()
                .map(|value| V::Bool(*value))
                .collect::<Vec<_>>();
            let p7 = if operator == "eq" {
                case.p7_values.clone().map(|values| {
                    P::Project(
                        Box::new(b1_p7_values("x", values)),
                        vec![(
                            "y".into(),
                            E::Eq(
                                Box::new(E::Field("x".into())),
                                Box::new(E::Literal(match case.type_name {
                                    "Bool" => R::Bool(true),
                                    "I64" => R::I64(i64::MAX),
                                    "F64Finite" => R::F64(2.0),
                                    "Utf8" => R::Text("μ".into()),
                                    _ => unreachable!(),
                                })),
                            ),
                        )],
                    )
                })
            } else {
                None
            };
            b1_assert_rows(
                &db,
                ir,
                hql,
                json!([{"name":"y","type":"Bool","nullable":false}]),
                b1_literal_rows("y", expected_rows),
                p7,
            );
        }
    }

    let nullable_params = json!({
        "xs":{"type":"List<Nullable<I64>>","value":["1",null,"3"]},
        "rhs":{"type":"I64","value":"1"}
    });
    let (ir, hql) = b1_project_requests(
        nullable_params,
        json!({"binary":"eq","left":field("x"),"right":{"param":"rhs"}}),
        "x = $rhs",
    );
    let p7 = P::Project(
        Box::new(b1_p7_values("x", vec![R::I64(1), R::Null, R::I64(3)])),
        vec![(
            "y".into(),
            E::Eq(
                Box::new(E::Field("x".into())),
                Box::new(E::Literal(R::I64(1))),
            ),
        )],
    );
    b1_assert_rows(
        &db,
        ir,
        hql,
        json!([{"name":"y","type":"Nullable<Bool>","nullable":true}]),
        b1_literal_rows("y", vec![V::Bool(true), V::Null, V::Bool(false)]),
        Some(p7),
    );
}

#[test]
fn scalar_type_admissibility_matrix_functions_and_checked_arithmetic() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);

    let strings = json!({
        "xs":{"type":"List<Nullable<Utf8>>","value":["Äther","ก😀",null]}
    });
    for (ir_expression, hql_expression, expected_type, expected) in [
        (
            json!({"call":"lower","args":[field("x")]}),
            "lower(x)",
            "Nullable<Utf8>",
            vec![V::Utf8("äther".into()), V::Utf8("ก😀".into()), V::Null],
        ),
        (
            json!({"call":"length","args":[field("x")]}),
            "length(x)",
            "Nullable<I64>",
            vec![V::I64(5), V::I64(2), V::Null],
        ),
    ] {
        let (ir, hql) = b1_project_requests(strings.clone(), ir_expression, hql_expression);
        b1_assert_rows(
            &db,
            ir,
            hql,
            json!([{"name":"y","type":expected_type,"nullable":true}]),
            b1_literal_rows("y", expected),
            None,
        );
    }

    struct ArithmeticCase {
        type_name: &'static str,
        encoded: Value,
        rhs_type: &'static str,
        rhs_value: Value,
        rhs_hql: &'static str,
        p7_values: R,
        p7_rhs: R,
        expected: [V; 5],
    }
    let arithmetic_cases = [
        ArithmeticCase {
            type_name: "I64",
            encoded: json!(["6"]),
            rhs_type: "I64",
            rhs_value: json!("2"),
            rhs_hql: "$rhs",
            p7_values: R::I64(6),
            p7_rhs: R::I64(2),
            expected: [V::I64(8), V::I64(4), V::I64(12), V::I64(3), V::I64(0)],
        },
        ArithmeticCase {
            type_name: "F64Finite",
            encoded: json!([7.5]),
            rhs_type: "F64Finite",
            rhs_value: json!(2.0),
            rhs_hql: "2.0",
            p7_values: R::F64(7.5),
            p7_rhs: R::F64(2.0),
            expected: [
                V::F64(9.5),
                V::F64(5.5),
                V::F64(15.0),
                V::F64(3.75),
                V::F64(1.5),
            ],
        },
    ];
    let arithmetic_ops = [
        ("add", "+"),
        ("sub", "-"),
        ("mul", "*"),
        ("div", "/"),
        ("rem", "%"),
    ];
    for case in arithmetic_cases {
        for (index, (op, symbol)) in arithmetic_ops.into_iter().enumerate() {
            let params = json!({
                "xs":{"type":format!("List<{}>",case.type_name),"value":case.encoded.clone()},
                "rhs":{"type":case.rhs_type,"value":case.rhs_value.clone()}
            });
            let ir_expression = json!({"binary":op,"left":field("x"),"right":{"param":"rhs"}});
            let (ir, hql) = b1_project_requests(
                params,
                ir_expression,
                &format!("x {symbol} {}", case.rhs_hql),
            );
            let p7 = match op {
                "add" => Some(E::Add(
                    Box::new(E::Field("x".into())),
                    Box::new(E::Literal(case.p7_rhs.clone())),
                )),
                "div" => Some(E::Div(
                    Box::new(E::Field("x".into())),
                    Box::new(E::Literal(case.p7_rhs.clone())),
                )),
                "rem" => Some(E::Rem(
                    Box::new(E::Field("x".into())),
                    Box::new(E::Literal(case.p7_rhs.clone())),
                )),
                _ => None,
            }
            .map(|expression| {
                P::Project(
                    Box::new(b1_p7_values("x", vec![case.p7_values.clone()])),
                    vec![("y".into(), expression)],
                )
            });
            b1_assert_rows(
                &db,
                ir,
                hql,
                json!([{"name":"y","type":case.type_name,"nullable":false}]),
                b1_literal_rows("y", vec![case.expected[index].clone()]),
                p7,
            );
        }
    }

    let nullable = json!({
        "xs":{"type":"List<Nullable<I64>>","value":[null,"6"]},
        "rhs":{"type":"I64","value":"2"}
    });
    let (ir, hql) = b1_project_requests(
        nullable,
        json!({"binary":"add","left":field("x"),"right":{"param":"rhs"}}),
        "x + $rhs",
    );
    b1_assert_rows(
        &db,
        ir,
        hql,
        json!([{"name":"y","type":"Nullable<I64>","nullable":true}]),
        b1_literal_rows("y", vec![V::Null, V::I64(8)]),
        Some(P::Project(
            Box::new(b1_p7_values("x", vec![R::Null, R::I64(6)])),
            vec![(
                "y".into(),
                E::Add(
                    Box::new(E::Field("x".into())),
                    Box::new(E::Literal(R::I64(2))),
                ),
            )],
        )),
    );

    let wrong_function = json!({
        "xs":{"type":"List<I64>","value":["1"]}
    });
    let (ir, hql) = b1_project_requests(
        wrong_function,
        json!({"call":"lower","args":[field("x")]}),
        "lower(x)",
    );
    b1_assert_paired_error(&db, ir, hql, "BIND_ERROR", "bind", "function_type");
}

#[test]
fn scalar_type_admissibility_matrix_rejects_unsupported_cells() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);

    let (ir, hql) = b1_filter_requests(
        json!({"xs":{"type":"List<I64>","value":["1"]}}),
        field("x"),
        "x",
    );
    b1_assert_paired_error(&db, ir, hql, "BIND_ERROR", "bind", "boolean_type");

    let (ir, hql) = b1_project_requests(
        json!({
            "xs":{"type":"List<I64>","value":["1"]},
            "rhs":{"type":"Bool","value":true}
        }),
        json!({"binary":"and","left":field("x"),"right":{"param":"rhs"}}),
        "x AND $rhs",
    );
    b1_assert_paired_error(&db, ir, hql, "BIND_ERROR", "bind", "boolean_type");

    for (type_name, encoded) in [
        ("List<I64>", json!([["1"]])),
        ("Vector<g4-b1-space,2,f64>", json!([[0.5, 1.5]])),
    ] {
        // The type_name here is itself a scalar row type: List<T> therefore
        // declares a parameter list whose elements have that non-scalar shape.
        let params = json!({"xs":{"type":format!("List<{type_name}>"),"value":encoded}});
        let (ir, hql) = b1_project_requests(
            params,
            json!({"binary":"eq","left":field("x"),"right":field("x")}),
            "x = x",
        );
        b1_assert_paired_error(
            &db,
            ir,
            hql,
            "CAPABILITY_UNSUPPORTED",
            "bind",
            "equality_unavailable",
        );
    }

    let (ir, hql, _, _) = b1_value_kernel_requests(
        "Vector<g4-b1-space,2,f64>",
        json!([[0.5]]),
        B1RowKernel::Values,
    );
    b1_assert_paired_error(
        &db,
        ir,
        hql,
        "COLLECTION_SPACE_MISMATCH",
        "bind",
        "vector_dimension",
    );

    let params = json!({
        "xs":{"type":"List<I64>","value":["1"]},
        "rhs":{"type":"F64Finite","value":1.0}
    });
    let (ir, hql) = b1_project_requests(
        params,
        json!({"binary":"eq","left":field("x"),"right":{"param":"rhs"}}),
        "x = $rhs",
    );
    b1_assert_paired_error(&db, ir, hql, "BIND_ERROR", "bind", "comparison_type");

    for (type_name, rhs_type, rhs_value, operation, symbol) in [
        ("I64", "F64Finite", json!(2.0), "add", "+"),
        ("DecimalU64", "DecimalU64", json!("2"), "add", "+"),
    ] {
        let params = json!({
            "xs":{"type":format!("List<{type_name}>"),"value":json!(["1"])},
            "rhs":{"type":rhs_type,"value":rhs_value}
        });
        let (ir, hql) = b1_project_requests(
            params,
            json!({"binary":operation,"left":field("x"),"right":{"param":"rhs"}}),
            &format!("x {symbol} $rhs"),
        );
        b1_assert_paired_error(&db, ir, hql, "BIND_ERROR", "bind", "numeric_type");
    }

    for (type_name, encoded) in [
        ("List<I64>", json!([["1"]])),
        ("Vector<g4-b1-space,2,f64>", json!([[0.5, 1.5]])),
    ] {
        let (ir, hql, _, _) = b1_value_kernel_requests(type_name, encoded, B1RowKernel::Sort);
        b1_assert_paired_error(&db, ir, hql, "BIND_ERROR", "bind", "comparator_unavailable");
    }

    for (type_name, encoded, expected_code, expected_reason) in [
        (
            "Json",
            json!([]),
            "CAPABILITY_UNSUPPORTED",
            "json_parameter_unavailable",
        ),
        (
            "Entity",
            json!([]),
            "CAPABILITY_UNSUPPORTED",
            "entity_parameter_scope",
        ),
        // Path has an internal value variant but is intentionally outside the
        // accepted public type spellings, so it fails closed during type parse.
        (
            "Path",
            json!([]),
            "CAPABILITY_UNSUPPORTED",
            "type_unavailable",
        ),
        (
            "Score",
            json!([{}]),
            "CAPABILITY_UNSUPPORTED",
            "domain_parameter_unavailable",
        ),
        (
            "Context",
            json!([{}]),
            "CAPABILITY_UNSUPPORTED",
            "domain_parameter_unavailable",
        ),
        (
            "HistoryRevision",
            json!([{}]),
            "CAPABILITY_UNSUPPORTED",
            "domain_parameter_unavailable",
        ),
        (
            "ChangeEvent",
            json!([{}]),
            "CAPABILITY_UNSUPPORTED",
            "domain_parameter_unavailable",
        ),
        (
            "Nullable<Nullable<I64>>",
            json!([]),
            "BIND_ERROR",
            "nested_nullable",
        ),
    ] {
        let (ir, hql, _, _) = b1_value_kernel_requests(type_name, encoded, B1RowKernel::Values);
        b1_assert_paired_error(&db, ir, hql, expected_code, "bind", expected_reason);
    }

    for type_name in ["Null", "Json", "Nullable<Json>"] {
        let params = json!({
            "xs":{"type":"List<I64>","value":[]},
            "bad":{"type":type_name,"value":null}
        });
        let ir = request(
            vec![
                values("v", "xs", "x"),
                node(
                    "p",
                    "Project",
                    &["v"],
                    json!({"fields":[{"expression":{"param":"bad"},"as":"y"}]}),
                ),
            ],
            "p",
            params.clone(),
        );
        let hql = b1_hql_request(params, "VALUES $xs AS x |> RETURN $bad AS y");
        b1_assert_paired_error(
            &db,
            ir,
            hql,
            "CAPABILITY_UNSUPPORTED",
            "bind",
            "json_parameter_unavailable",
        );
    }
}

#[test]
fn decimal_u64_structural_pagination_accepts_boundary_parameters() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);
    let source_values = vec![V::I64(4), V::I64(5)];
    let p7_values = vec![R::I64(4), R::I64(5)];

    for (op, count) in [
        ("Take", 0_u64),
        ("Take", u64::MAX),
        ("Offset", 0),
        ("Offset", u64::MAX),
    ] {
        let params = json!({
            "xs":{"type":"List<I64>","value":["4","5"]},
            "n":{"type":"DecimalU64","value":count.to_string()}
        });
        let ir = request(
            vec![
                values("v", "xs", "x"),
                node("k", op, &["v"], json!({"count":count})),
            ],
            "k",
            params.clone(),
        );
        let hql_clause = if op == "Take" { "TAKE" } else { "SKIP" };
        let hql = b1_hql_request(
            params,
            &format!("VALUES $xs AS x |> {hql_clause} $n |> RETURN x"),
        );
        let expected = match (op, count) {
            ("Take", 0) | ("Offset", u64::MAX) => vec![],
            _ => source_values.clone(),
        };
        let bounded_count = if op == "Take" {
            if count == 0 {
                0
            } else {
                p7_values.len()
            }
        } else if count == 0 {
            0
        } else {
            3
        };
        let p7 = if op == "Take" {
            P::Take(
                Box::new(b1_p7_values("x", p7_values.clone())),
                bounded_count,
            )
        } else {
            P::Offset(
                Box::new(b1_p7_values("x", p7_values.clone())),
                bounded_count,
            )
        };
        b1_assert_rows(
            &db,
            ir,
            hql,
            json!([{"name":"x","type":"I64","nullable":false}]),
            b1_literal_rows("x", expected),
            Some(p7),
        );
    }

    // Typed IR expresses pagination as a numeric config field, so there is no
    // typed-IR parameter cell corresponding to HQL's DecimalU64 parameter gate.
    let params = json!({
        "xs":{"type":"List<I64>","value":["4"]},
        "n":{"type":"I64","value":"1"}
    });
    let hql = b1_hql_request(params, "VALUES $xs AS x |> TAKE $n |> RETURN x");
    b1_assert_error(&db, hql, "BIND_ERROR", "bind", "parameter_type_mismatch");
}

#[test]
fn finite_float_boundary_and_checked_result_fail_closed() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);

    // JSON has no numeric representation for NaN or infinity. HQL rejects an
    // overflowing numeric token during parse; typed IR rejects a string that is
    // not a JSON number. These are representation-specific guards, not a
    // fabricated cross-frontend NaN value.
    let empty_i64 = json!({"xs":{"type":"List<I64>","value":[]}});
    let hql = b1_hql_request(empty_i64.clone(), "VALUES $xs AS x |> RETURN 1e999 AS y");
    b1_assert_error(&db, hql, "HQL_PARSE_ERROR", "parse", "nonfinite_literal");

    let ir = request(
        vec![
            values("v", "xs", "x"),
            node(
                "p",
                "Project",
                &["v"],
                json!({"fields":[{"expression":{"type":"F64Finite","literal":"NaN"},"as":"y"}]}),
            ),
        ],
        "p",
        empty_i64,
    );
    b1_assert_error(&db, ir, "BIND_ERROR", "bind", "literal_type");

    let overflow = json!({
        "xs":{"type":"List<F64Finite>","value":[1e308]},
        "rhs":{"type":"F64Finite","value":1e308}
    });
    db.publish_generation().unwrap();
    let before = state(&db, directory.path());
    let (ir, hql) = b1_project_requests(
        overflow,
        json!({"binary":"mul","left":field("x"),"right":{"param":"rhs"}}),
        "x * $rhs",
    );
    b1_assert_paired_error(&db, ir, hql, "BIND_ERROR", "execute", "nonfinite_result");
    assert_state(&db, directory.path(), &before);
}

/// Phase C1 selected ORDER BY contract cases: literal rows are primary,
/// unchanged P7 is used only for its supported scalar values, and full-key tie
/// identity is deliberately not asserted because the contract does not define
/// a stable tie order.
#[test]
fn scalar_ordering_matrix_pairs_frontends_with_literal_rows_and_counters() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);

    struct SortCase {
        type_name: &'static str,
        encoded: Value,
        ascending: Vec<V>,
        descending: Vec<V>,
        p7_values: Option<Vec<R>>,
    }
    let cases = [
        SortCase {
            type_name: "Bool",
            encoded: json!([true, false, true]),
            ascending: vec![V::Bool(false), V::Bool(true), V::Bool(true)],
            descending: vec![V::Bool(true), V::Bool(true), V::Bool(false)],
            p7_values: Some(vec![R::Bool(true), R::Bool(false), R::Bool(true)]),
        },
        SortCase {
            type_name: "I64",
            encoded: json!(["2", "-1", "2", "0"]),
            ascending: vec![V::I64(-1), V::I64(0), V::I64(2), V::I64(2)],
            descending: vec![V::I64(2), V::I64(2), V::I64(0), V::I64(-1)],
            p7_values: Some(vec![R::I64(2), R::I64(-1), R::I64(2), R::I64(0)]),
        },
        SortCase {
            type_name: "DecimalU64",
            encoded: json!(["10", "2", "10", "0"]),
            ascending: vec![
                V::DecimalU64(0),
                V::DecimalU64(2),
                V::DecimalU64(10),
                V::DecimalU64(10),
            ],
            descending: vec![
                V::DecimalU64(10),
                V::DecimalU64(10),
                V::DecimalU64(2),
                V::DecimalU64(0),
            ],
            p7_values: None,
        },
        SortCase {
            type_name: "F64Finite",
            encoded: json!([2.5, -0.5, 2.5, 0.0]),
            ascending: vec![V::F64(-0.5), V::F64(0.0), V::F64(2.5), V::F64(2.5)],
            descending: vec![V::F64(2.5), V::F64(2.5), V::F64(0.0), V::F64(-0.5)],
            p7_values: Some(vec![R::F64(2.5), R::F64(-0.5), R::F64(2.5), R::F64(0.0)]),
        },
        SortCase {
            type_name: "Utf8",
            encoded: json!(["b", "a", "b", "c"]),
            ascending: vec![
                V::Utf8("a".into()),
                V::Utf8("b".into()),
                V::Utf8("b".into()),
                V::Utf8("c".into()),
            ],
            descending: vec![
                V::Utf8("c".into()),
                V::Utf8("b".into()),
                V::Utf8("b".into()),
                V::Utf8("a".into()),
            ],
            p7_values: Some(vec![
                R::Text("b".into()),
                R::Text("a".into()),
                R::Text("b".into()),
                R::Text("c".into()),
            ]),
        },
    ];

    for case in cases {
        let params = json!({
            "xs":{"type":format!("List<{}>",case.type_name),"value":case.encoded.clone()}
        });
        let row_count = case.ascending.len() as u64;
        let ops = [
            (QueryOpV2::Values, 0, row_count),
            (QueryOpV2::Sort, row_count, row_count),
            (QueryOpV2::Project, row_count, row_count),
        ];
        for (direction, descending, expected) in [
            ("asc", false, &case.ascending),
            ("desc", true, &case.descending),
        ] {
            let (ir, hql) = c1_sort_requests(
                params.clone(),
                vec![c1_sort_key(field("x"), direction, "last")],
                &format!("x {} NULLS LAST", direction.to_ascii_uppercase()),
            );
            let p7 = case.p7_values.clone().map(|values| {
                c1_p7_sort_project(
                    values,
                    vec![reference::SortKey {
                        expr: E::Field("x".into()),
                        descending,
                        nulls_first: false,
                    }],
                )
            });
            c1_assert_order_modes(
                &db,
                ir,
                hql,
                C1OrderExpectation {
                    type_name: case.type_name,
                    nullable: false,
                    values: expected.clone(),
                    p7,
                    ops: &ops,
                },
            );
        }
    }

    let nullable_input = [Some(2), None, Some(1), Some(2), None];
    let nullable_params = json!({
        "xs":{"type":"List<Nullable<I64>>","value":["2",null,"1","2",null]}
    });
    let nullable_p7 = nullable_input
        .iter()
        .map(|value| value.map(R::I64).unwrap_or(R::Null))
        .collect::<Vec<_>>();
    let nullable_ops = [
        (QueryOpV2::Values, 0, 5),
        (QueryOpV2::Sort, 5, 5),
        (QueryOpV2::Project, 5, 5),
    ];
    for (direction, descending, expected_first, expected_last) in [
        (
            "asc",
            false,
            vec![V::Null, V::Null, V::I64(1), V::I64(2), V::I64(2)],
            vec![V::I64(1), V::I64(2), V::I64(2), V::Null, V::Null],
        ),
        (
            "desc",
            true,
            vec![V::Null, V::Null, V::I64(2), V::I64(2), V::I64(1)],
            vec![V::I64(2), V::I64(2), V::I64(1), V::Null, V::Null],
        ),
    ] {
        for (nulls, nulls_first, expected) in [
            ("first", true, expected_first),
            ("last", false, expected_last.clone()),
        ] {
            let (ir, hql) = c1_sort_requests(
                nullable_params.clone(),
                vec![c1_sort_key(field("x"), direction, nulls)],
                &format!(
                    "x {} NULLS {}",
                    direction.to_ascii_uppercase(),
                    nulls.to_ascii_uppercase()
                ),
            );
            let p7 = c1_p7_sort_project(
                nullable_p7.clone(),
                vec![reference::SortKey {
                    expr: E::Field("x".into()),
                    descending,
                    nulls_first,
                }],
            );
            c1_assert_order_modes(
                &db,
                ir,
                hql,
                C1OrderExpectation {
                    type_name: "Nullable<I64>",
                    nullable: true,
                    values: expected,
                    p7: Some(p7),
                    ops: &nullable_ops,
                },
            );
        }

        // HQL's omitted NULL placement is contract-defined as LAST in either
        // direction; typed IR still supplies its required explicit NullOrder.
        let (ir, hql) = c1_sort_requests(
            nullable_params.clone(),
            vec![c1_sort_key(field("x"), direction, "last")],
            &format!("x {}", direction.to_ascii_uppercase()),
        );
        let p7 = c1_p7_sort_project(
            nullable_p7.clone(),
            vec![reference::SortKey {
                expr: E::Field("x".into()),
                descending,
                nulls_first: false,
            }],
        );
        c1_assert_order_modes(
            &db,
            ir,
            hql,
            C1OrderExpectation {
                type_name: "Nullable<I64>",
                nullable: true,
                values: expected_last,
                p7: Some(p7),
                ops: &nullable_ops,
            },
        );
    }

    for (encoded, values, expected) in [
        (json!([]), vec![], vec![]),
        (json!(["7"]), vec![R::I64(7)], vec![V::I64(7)]),
    ] {
        let params = json!({"xs":{"type":"List<I64>","value":encoded}});
        let count = expected.len() as u64;
        let ops = [
            (QueryOpV2::Values, 0, count),
            (QueryOpV2::Sort, count, count),
            (QueryOpV2::Project, count, count),
        ];
        let (ir, hql) = c1_sort_requests(
            params,
            vec![c1_sort_key(field("x"), "asc", "last")],
            "x ASC NULLS LAST",
        );
        c1_assert_order_modes(
            &db,
            ir,
            hql,
            C1OrderExpectation {
                type_name: "I64",
                nullable: false,
                values: expected,
                p7: Some(c1_p7_sort_project(
                    values,
                    vec![reference::SortKey {
                        expr: E::Field("x".into()),
                        descending: false,
                        nulls_first: false,
                    }],
                )),
                ops: &ops,
            },
        );
    }

    let tie_params = json!({"xs":{"type":"List<I64>","value":["2","1","4","3","2","1","4"]}});
    let primary_ir = json!({
        "binary":"rem","left":field("x"),"right":{"type":"I64","literal":"2"}
    });
    let (ir, hql) = c1_sort_requests(
        tie_params,
        vec![
            c1_sort_key(primary_ir, "asc", "last"),
            c1_sort_key(field("x"), "desc", "last"),
        ],
        "x % 2 ASC NULLS LAST, x DESC NULLS LAST",
    );
    let tie_input = [2, 1, 4, 3, 2, 1, 4];
    let tie_p7 = c1_p7_sort_project(
        tie_input.iter().copied().map(R::I64).collect::<Vec<_>>(),
        vec![
            reference::SortKey {
                expr: E::Rem(
                    Box::new(E::Field("x".into())),
                    Box::new(E::Literal(R::I64(2))),
                ),
                descending: false,
                nulls_first: false,
            },
            reference::SortKey {
                expr: E::Field("x".into()),
                descending: true,
                nulls_first: false,
            },
        ],
    );
    let tie_rows = 7;
    c1_assert_order_modes(
        &db,
        ir,
        hql,
        C1OrderExpectation {
            type_name: "I64",
            nullable: false,
            values: vec![
                V::I64(4),
                V::I64(4),
                V::I64(2),
                V::I64(2),
                V::I64(3),
                V::I64(1),
                V::I64(1),
            ],
            p7: Some(tie_p7),
            ops: &[
                (QueryOpV2::Values, 0, tie_rows),
                (QueryOpV2::Sort, tie_rows, tie_rows),
                (QueryOpV2::Project, tie_rows, tie_rows),
            ],
        },
    );

    let pagination_input = [4, 1, 3, 2, 2];
    for (skip, take, expected_values) in [
        (0_u64, 3_u64, vec![V::I64(1), V::I64(2), V::I64(2)]),
        (2, 2, vec![V::I64(2), V::I64(3)]),
        (9, 1, vec![]),
        (0, 0, vec![]),
        (4, 5, vec![V::I64(4)]),
    ] {
        let params = json!({
            "xs":{"type":"List<I64>","value":["4","1","3","2","2"]}
        });
        let (ir, hql) = c1_pagination_requests(params, skip, take);
        let offset_out = pagination_input.len().saturating_sub(skip as usize) as u64;
        let take_out = offset_out.min(take);
        let p7_source = P::Project(
            Box::new(b1_p7_values(
                "x",
                pagination_input
                    .iter()
                    .copied()
                    .map(R::I64)
                    .collect::<Vec<_>>(),
            )),
            vec![("y".into(), E::Field("x".into()))],
        );
        let p7_sorted = P::Sort(
            Box::new(p7_source),
            vec![reference::SortKey {
                expr: E::Field("y".into()),
                descending: false,
                nulls_first: false,
            }],
        );
        let p7_paged = P::Project(
            Box::new(P::Take(
                Box::new(P::Offset(Box::new(p7_sorted), skip as usize)),
                take as usize,
            )),
            vec![("y".into(), E::Field("y".into()))],
        );
        c1_assert_order_modes(
            &db,
            ir,
            hql,
            C1OrderExpectation {
                type_name: "I64",
                nullable: false,
                values: expected_values,
                p7: Some(p7_paged),
                ops: &[
                    (QueryOpV2::Values, 0, 5),
                    (QueryOpV2::Project, 5, 5),
                    (QueryOpV2::Sort, 5, 5),
                    (QueryOpV2::Offset, 5, offset_out),
                    (QueryOpV2::Take, offset_out, take_out),
                    (QueryOpV2::Project, take_out, take_out),
                ],
            },
        );
    }
}

fn c1b_assert_order_modes_plan_first(
    db: &Storage,
    requests: &[(&str, QueryRequestV2)],
    expected: C1OrderExpectation<'_>,
) {
    let expected_columns =
        json!([{"name":"y","type":expected.type_name,"nullable":expected.nullable}]);
    let expected_rows = b1_literal_rows("y", expected.values);
    if let Some(oracle) = expected.p7 {
        let p7_rows = reference::execute(&oracle)
            .unwrap()
            .iter()
            .map(|row| {
                row.iter()
                    .map(|(name, value)| (name.clone(), b1_from_p7(value)))
                    .collect::<BTreeMap<_, _>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(p7_rows, expected_rows, "C1b literal rows vs unchanged P7");
    }

    let mut planned = Vec::with_capacity(requests.len());
    for (frontend, request) in requests {
        let mut planned_request = request.clone();
        planned_request.explain = Some(ExplainV2::Plan);
        let QueryOutcomeV2::Plan(plan) = db.query_v2(access(), planned_request).unwrap() else {
            panic!("{frontend} Plan must be checked before result queries");
        };
        c1_assert_plan_nodes(&plan, expected.ops, false);
        planned.push((*frontend, request.clone(), plan));
    }

    for (frontend, request, plan) in planned {
        let QueryOutcomeV2::Rows(normal) = db.query_v2(access(), request.clone()).unwrap() else {
            panic!("{frontend} normal execution must return rows");
        };
        assert!(normal.explain.is_none());
        assert_eq!(
            serde_json::to_value(&normal.columns).unwrap(),
            expected_columns
        );
        assert_eq!(
            normal.rows, expected_rows,
            "{frontend} literal ordered rows"
        );
        assert_eq!(
            normal.completeness.status,
            genesis_block_native::query::hql2::result::CompletionStatusV2::Complete
        );

        let mut analyzed_request = request.clone();
        analyzed_request.explain = Some(ExplainV2::Analyze);
        let QueryOutcomeV2::Rows(analyzed) = db.query_v2(access(), analyzed_request).unwrap()
        else {
            panic!("{frontend} Analyze must return rows");
        };
        assert_eq!(analyzed.rows, expected_rows, "{frontend} Analyze rows");
        assert_eq!(
            serde_json::to_value(&analyzed.columns).unwrap(),
            expected_columns
        );
        assert_eq!(normal.rows, analyzed.rows);
        assert_eq!(normal.columns, analyzed.columns);
        assert_eq!(normal.semantics, analyzed.semantics);
        assert_eq!(normal.completeness, analyzed.completeness);
        assert_eq!(normal.index_frontiers, analyzed.index_frontiers);
        assert_eq!(normal.cursor, analyzed.cursor);

        let mut analyzed_plan = analyzed.explain.unwrap();
        assert_eq!(analyzed_plan.plan_hash, plan.plan_hash);
        assert_eq!(analyzed_plan.planner_version, plan.planner_version);
        c1_assert_plan_nodes(&analyzed_plan, expected.ops, true);
        for node in &mut analyzed_plan.plan {
            node.actual.take();
        }
        assert_eq!(analyzed_plan.plan, plan.plan);
    }
}

/// Phase C1b qualifies nullable ordering for the remaining contract-orderable
/// scalar bases. Literal rows are primary; P7 is independent only where its
/// value model represents the type. Full-key tie identity remains unspecified.
#[test]
fn scalar_nullable_ordering_matrix_covers_remaining_orderable_types() {
    let directory = TempDir::new().unwrap();
    let db = open(&directory);

    struct NullableSortCase {
        type_name: &'static str,
        encoded: Value,
        ascending: Vec<V>,
        descending: Vec<V>,
        p7_values: Option<Vec<R>>,
    }
    let cases = [
        NullableSortCase {
            type_name: "Nullable<Bool>",
            encoded: json!([true, null, false, true, null]),
            ascending: vec![V::Bool(false), V::Bool(true), V::Bool(true)],
            descending: vec![V::Bool(true), V::Bool(true), V::Bool(false)],
            p7_values: Some(vec![
                R::Bool(true),
                R::Null,
                R::Bool(false),
                R::Bool(true),
                R::Null,
            ]),
        },
        NullableSortCase {
            type_name: "Nullable<DecimalU64>",
            encoded: json!(["10", null, "2", "10", null]),
            ascending: vec![V::DecimalU64(2), V::DecimalU64(10), V::DecimalU64(10)],
            descending: vec![V::DecimalU64(10), V::DecimalU64(10), V::DecimalU64(2)],
            p7_values: None,
        },
        NullableSortCase {
            type_name: "Nullable<F64Finite>",
            encoded: json!([2.5, null, -0.5, 2.5, 0.0, null]),
            ascending: vec![V::F64(-0.5), V::F64(0.0), V::F64(2.5), V::F64(2.5)],
            descending: vec![V::F64(2.5), V::F64(2.5), V::F64(0.0), V::F64(-0.5)],
            p7_values: Some(vec![
                R::F64(2.5),
                R::Null,
                R::F64(-0.5),
                R::F64(2.5),
                R::F64(0.0),
                R::Null,
            ]),
        },
        NullableSortCase {
            type_name: "Nullable<Utf8>",
            encoded: json!(["b", null, "a", "b", null, "c"]),
            ascending: vec![
                V::Utf8("a".into()),
                V::Utf8("b".into()),
                V::Utf8("b".into()),
                V::Utf8("c".into()),
            ],
            descending: vec![
                V::Utf8("c".into()),
                V::Utf8("b".into()),
                V::Utf8("b".into()),
                V::Utf8("a".into()),
            ],
            p7_values: Some(vec![
                R::Text("b".into()),
                R::Null,
                R::Text("a".into()),
                R::Text("b".into()),
                R::Null,
                R::Text("c".into()),
            ]),
        },
    ];

    for case in &cases {
        let params = json!({
            "xs": {
                "type": format!("List<{}>", case.type_name),
                "value": case.encoded.clone()
            }
        });
        let row_count = case.ascending.len() as u64 + 2;
        let ops = [
            (QueryOpV2::Values, 0, row_count),
            (QueryOpV2::Sort, row_count, row_count),
            (QueryOpV2::Project, row_count, row_count),
        ];

        for (direction, descending) in [("asc", false), ("desc", true)] {
            let sorted = if descending {
                case.descending.clone()
            } else {
                case.ascending.clone()
            };
            for (nulls, nulls_first) in [("first", true), ("last", false)] {
                let mut expected = sorted.clone();
                let null_rows = vec![V::Null; 2];
                if nulls_first {
                    expected.splice(0..0, null_rows);
                } else {
                    expected.extend(null_rows);
                }
                let (ir, hql) = c1_sort_requests(
                    params.clone(),
                    vec![c1_sort_key(field("x"), direction, nulls)],
                    &format!(
                        "x {} NULLS {}",
                        direction.to_ascii_uppercase(),
                        nulls.to_ascii_uppercase()
                    ),
                );
                let p7 = case.p7_values.clone().map(|values| {
                    c1_p7_sort_project(
                        values,
                        vec![reference::SortKey {
                            expr: E::Field("x".into()),
                            descending,
                            nulls_first,
                        }],
                    )
                });
                c1b_assert_order_modes_plan_first(
                    &db,
                    &[("typed IR", ir), ("HQL2", hql)],
                    C1OrderExpectation {
                        type_name: case.type_name,
                        nullable: true,
                        values: expected,
                        p7,
                        ops: &ops,
                    },
                );
            }

            // The HQL-only default control is three requests (Normal, Plan,
            // Analyze); typed IR remains explicit and is covered above.
            let mut expected_default = sorted.clone();
            expected_default.extend(vec![V::Null; 2]);
            let (ir, hql) = c1_sort_requests(
                params.clone(),
                vec![c1_sort_key(field("x"), direction, "last")],
                &format!("x {}", direction.to_ascii_uppercase()),
            );
            drop(ir);
            let p7 = case.p7_values.clone().map(|values| {
                c1_p7_sort_project(
                    values,
                    vec![reference::SortKey {
                        expr: E::Field("x".into()),
                        descending,
                        nulls_first: false,
                    }],
                )
            });
            c1b_assert_order_modes_plan_first(
                &db,
                &[("HQL2 default NULLS LAST", hql)],
                C1OrderExpectation {
                    type_name: case.type_name,
                    nullable: true,
                    values: expected_default,
                    p7,
                    ops: &ops,
                },
            );
        }
    }
}

/// Phase C2 qualifies selected composed scalar row shaping across the base
/// scalar types. Values are distinct before Sort so this does not assert
/// equal-key tie identity or multi-key ordering behavior.
#[test]
fn composed_scalar_pipeline_matches_literals_p7_and_diagnostics() {
    struct C2Case {
        type_name: &'static str,
        encoded: Value,
        p7_values: Option<Vec<R>>,
        expected: Vec<V>,
        sizes: [u64; 8],
    }

    let cases = [
        C2Case {
            type_name: "I64",
            encoded: json!(["2", null, "1", "2", "3", "0"]),
            p7_values: Some(vec![
                R::I64(2),
                R::Null,
                R::I64(1),
                R::I64(2),
                R::I64(3),
                R::I64(0),
            ]),
            expected: vec![V::I64(1), V::I64(2)],
            sizes: [6, 5, 5, 4, 4, 3, 2, 2],
        },
        C2Case {
            type_name: "Bool",
            encoded: json!([true, null, false, true, false]),
            p7_values: Some(vec![
                R::Bool(true),
                R::Null,
                R::Bool(false),
                R::Bool(true),
                R::Bool(false),
            ]),
            expected: vec![V::Bool(true)],
            sizes: [5, 4, 4, 2, 2, 1, 1, 1],
        },
        C2Case {
            type_name: "DecimalU64",
            encoded: json!(["10", null, "2", "10", "2", "7"]),
            p7_values: None,
            expected: vec![V::DecimalU64(7), V::DecimalU64(10)],
            sizes: [6, 5, 5, 3, 3, 2, 2, 2],
        },
        C2Case {
            type_name: "F64Finite",
            encoded: json!([-0.5, null, 2.5, 0.0, 2.5]),
            p7_values: Some(vec![
                R::F64(-0.5),
                R::Null,
                R::F64(2.5),
                R::F64(0.0),
                R::F64(2.5),
            ]),
            expected: vec![V::F64(0.0), V::F64(2.5)],
            sizes: [5, 4, 4, 3, 3, 2, 2, 2],
        },
        C2Case {
            type_name: "Utf8",
            encoded: json!(["b", null, "a", "c", "b"]),
            p7_values: Some(vec![
                R::Text("b".into()),
                R::Null,
                R::Text("a".into()),
                R::Text("c".into()),
                R::Text("b".into()),
            ]),
            expected: vec![V::Utf8("b".into()), V::Utf8("c".into())],
            sizes: [5, 4, 4, 3, 3, 2, 2, 2],
        },
    ];

    for case in cases {
        let params = json!({
            "xs": {
                "type": format!("List<Nullable<{}>>", case.type_name),
                "value": case.encoded.clone()
            }
        });
        let ir = request(
            vec![
                values("v", "xs", "x"),
                node(
                    "f",
                    "Filter",
                    &["v"],
                    json!({"predicate":{"binary":"eq","left":field("x"),"right":field("x")}}),
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
                node("o", "Offset", &["s"], json!({"count":1})),
                node("t", "Take", &["o"], json!({"count":2})),
                node(
                    "r",
                    "Project",
                    &["t"],
                    json!({"fields":[{"expression":field("y"),"as":"y"}]}),
                ),
            ],
            "r",
            params.clone(),
        );
        let mut ir = ir;
        ir.temporal = Some(
            serde_json::from_value(json!({"valid_at":chrono::Utc::now().to_rfc3339()})).unwrap(),
        );
        let mut hql = b1_hql_request(
            params,
            "VALUES $xs AS x |> FILTER x = x |> PROJECT x AS y |> DISTINCT |> ORDER BY y ASC NULLS LAST |> SKIP 1 |> TAKE 2 |> RETURN y",
        );
        hql.temporal = ir.temporal.clone();

        let literal_rows = b1_literal_rows("y", case.expected.clone());
        if let Some(p7_values) = case.p7_values {
            let filtered = P::Filter(
                Box::new(b1_p7_values("x", p7_values)),
                E::Eq(
                    Box::new(E::Field("x".into())),
                    Box::new(E::Field("x".into())),
                ),
            );
            let projected =
                P::Project(Box::new(filtered), vec![("y".into(), E::Field("x".into()))]);
            let distinct = P::Distinct(Box::new(projected));
            let sorted = P::Sort(
                Box::new(distinct),
                vec![reference::SortKey {
                    expr: E::Field("y".into()),
                    descending: false,
                    nulls_first: false,
                }],
            );
            let offset = P::Offset(Box::new(sorted), 1);
            let taken = P::Take(Box::new(offset), 2);
            let p7 = P::Project(Box::new(taken), vec![("y".into(), E::Field("y".into()))]);
            let p7_rows = reference::execute(&p7)
                .unwrap()
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|(name, value)| (name.clone(), b1_from_p7(value)))
                        .collect::<BTreeMap<_, _>>()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                p7_rows, literal_rows,
                "{} P7 vs independent literals",
                case.type_name
            );
        }

        let directory = TempDir::new().unwrap();
        let storage = open(&directory);
        scalar_policy(&storage);
        let before_plan = state(&storage, directory.path());
        let ops = [
            (QueryOpV2::Values, 0, case.sizes[0]),
            (QueryOpV2::Filter, case.sizes[0], case.sizes[1]),
            (QueryOpV2::Project, case.sizes[1], case.sizes[2]),
            (QueryOpV2::Distinct, case.sizes[2], case.sizes[3]),
            (QueryOpV2::Sort, case.sizes[3], case.sizes[4]),
            (QueryOpV2::Offset, case.sizes[4], case.sizes[5]),
            (QueryOpV2::Take, case.sizes[5], case.sizes[6]),
            (QueryOpV2::Project, case.sizes[6], case.sizes[7]),
        ];
        let mut planned = Vec::new();
        for (frontend, request) in [("typed IR", ir.clone()), ("HQL2", hql.clone())] {
            let mut plan_request = request.clone();
            plan_request.explain = Some(ExplainV2::Plan);
            let QueryOutcomeV2::Plan(plan) =
                storage.query_v2(access(), plan_request.clone()).unwrap()
            else {
                panic!("{frontend} Plan must be catalog-only");
            };
            assert_eq!(plan.catalog.policy_revision, 1);
            assert_eq!(plan.catalog.observed_frontier, before_plan.0);
            c1_assert_plan_nodes(&plan, &ops, false);
            assert_state(&storage, directory.path(), &before_plan);

            let denied = storage
                .query_v2(
                    AccessContext {
                        principal: "no-namespace".into(),
                        namespace: "default".into(),
                    },
                    plan_request,
                )
                .unwrap_err();
            assert_eq!(denied.code, "FORBIDDEN");
            assert_eq!(denied.stage, "authorize");
            assert_state(&storage, directory.path(), &before_plan);
            planned.push((frontend, request, plan));
        }

        storage.publish_generation().unwrap();
        let expected_columns = json!([{
            "name":"y",
            "type":format!("Nullable<{}>", case.type_name),
            "nullable":true
        }]);
        for (frontend, request, plan) in planned {
            let QueryOutcomeV2::Rows(normal) = storage.query_v2(access(), request.clone()).unwrap()
            else {
                panic!("{frontend} must return rows");
            };
            assert!(normal.explain.is_none());
            assert_eq!(
                serde_json::to_value(&normal.columns).unwrap(),
                expected_columns
            );
            assert_eq!(normal.rows, literal_rows, "{frontend} literal rows");
            assert_eq!(
                normal.completeness.status,
                genesis_block_native::query::hql2::result::CompletionStatusV2::Complete
            );

            let mut analyze_request = request.clone();
            analyze_request.explain = Some(ExplainV2::Analyze);
            let QueryOutcomeV2::Rows(analyzed) =
                storage.query_v2(access(), analyze_request).unwrap()
            else {
                panic!("{frontend} Analyze must return rows");
            };
            assert_eq!(analyzed.rows, literal_rows);
            assert_eq!(
                serde_json::to_value(&analyzed.columns).unwrap(),
                expected_columns
            );
            assert_eq!(normal.rows, analyzed.rows);
            assert_eq!(normal.columns, analyzed.columns);
            assert_eq!(normal.semantics, analyzed.semantics);
            assert_eq!(normal.completeness, analyzed.completeness);
            assert_eq!(normal.index_frontiers, analyzed.index_frontiers);
            assert_eq!(normal.cursor, analyzed.cursor);

            let mut actual = analyzed.explain.unwrap();
            assert_eq!(actual.plan_hash, plan.plan_hash);
            assert_eq!(actual.planner_version, plan.planner_version);
            c1_assert_plan_nodes(&actual, &ops, true);
            for node in &mut actual.plan {
                node.actual.take();
            }
            assert_eq!(actual.plan, plan.plan);

            for mode in [None, Some(ExplainV2::Analyze)] {
                let mut denied_request = request.clone();
                denied_request.explain = mode.clone();
                let before = state(&storage, directory.path());
                let denied = storage
                    .query_v2(
                        AccessContext {
                            principal: "no-namespace".into(),
                            namespace: "default".into(),
                        },
                        denied_request,
                    )
                    .unwrap_err();
                assert_eq!(denied.code, "FORBIDDEN");
                assert_eq!(denied.stage, "authorize");
                assert_state(&storage, directory.path(), &before);

                let mut limited_request = request.clone();
                limited_request.explain = mode;
                limited_request.budget =
                    Some(serde_json::from_value(json!({"max_result_bytes":1})).unwrap());
                let before = state(&storage, directory.path());
                let error = storage.query_v2(access(), limited_request).unwrap_err();
                assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
                assert_eq!(error.stage, "execute");
                assert_eq!(error.detail.unwrap()["reason"], "query_limit");
                assert_state(&storage, directory.path(), &before);
            }
        }
    }
}
