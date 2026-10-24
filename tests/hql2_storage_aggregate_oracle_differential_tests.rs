//! Storage-bound HQL2 and typed-IR aggregate parity against independent P7.
#[path = "support/hql2_reference.rs"]
mod reference;

use genesis_block_native::{
    query::hql2::{value::QueryValueV2 as V, QueryOutcomeV2},
    uee_v2::{ExplainV2, HqlLanguageVersionV2, QueryIrV2, QueryOpV2, QueryRequestV2},
    *,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
use tempfile::TempDir;

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap()
}

fn access() -> AccessContext {
    AccessContext {
        principal: "reader".into(),
        namespace: "default".into(),
    }
}

fn request(request_id: String, values: &[Option<i64>]) -> QueryRequestV2 {
    let parameter_type = "List<Nullable<I64>>";
    let param_values: Vec<Value> = values
        .iter()
        .map(|value| {
            value
                .map(|value| json!(value.to_string()))
                .unwrap_or(Value::Null)
        })
        .collect();
    let field = json!({"field":{"alias":"x","path":[]}});
    let aggregates = [
        ("count_all", "n_all", Vec::new()),
        ("count", "n", vec![field.clone()]),
        ("sum", "sum_x", vec![field.clone()]),
        ("avg", "avg_x", vec![field.clone()]),
        ("min", "min_x", vec![field.clone()]),
        ("max", "max_x", vec![field.clone()]),
        ("collect", "items", vec![field]),
    ]
    .into_iter()
    .map(|(call, alias, args)| json!({"expression":{"call":call,"args":args},"as":alias}))
    .collect::<Vec<_>>();
    let ir: QueryIrV2 = serde_json::from_value(json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"v","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}},
            {"id":"a","op":"Aggregate","inputs":["v"],"config":{"group_by":[],"aggregates":aggregates}}
        ],
        "root":"a",
        "parameter_types":{"xs":parameter_type}
    }))
    .unwrap();

    QueryRequestV2 {
        contract_version: "genesis.api.v2".into(),
        request_id,
        namespace: "default".into(),
        hql: None,
        ir: Some(ir),
        language_version: None,
        params: BTreeMap::from([(
            "xs".into(),
            json!({"type":parameter_type,"value":param_values}),
        )]),
        temporal: None,
        index_policy: None,
        required_frontier: None,
        transaction_id: None,
        budget: None,
        allow_partial: None,
        explain: None,
        format: None,
    }
}

fn bags_up_to_four() -> Vec<Vec<Option<i64>>> {
    let mut bags = vec![Vec::new()];
    for len in 1..=4 {
        for seed in 0..3usize.pow(len as u32) {
            let mut digits = seed;
            let bag = (0..len)
                .map(|_| {
                    let value = match digits % 3 {
                        0 => None,
                        1 => Some(-1),
                        _ => Some(2),
                    };
                    digits /= 3;
                    value
                })
                .collect();
            bags.push(bag);
        }
    }
    bags
}

fn from_p7(value: &reference::Value) -> V {
    match value {
        reference::Value::Null => V::Null,
        reference::Value::I64(value) => V::I64(*value),
        reference::Value::F64(value) => V::F64(*value),
        reference::Value::List(values) => V::List(values.iter().map(from_p7).collect()),
        other => panic!("unexpected P7 aggregate value: {other:?}"),
    }
}

fn expected_rows(values: &[Option<i64>]) -> Vec<BTreeMap<String, V>> {
    use reference::{Aggregate as A, Expr as E, Plan as P, Value as R};

    let input = P::Values(
        values
            .iter()
            .map(|value| BTreeMap::from([("x".into(), value.map(R::I64).unwrap_or(R::Null))]))
            .collect(),
    );
    reference::execute(&P::Aggregate {
        input: Box::new(input),
        keys: vec![],
        aggregates: vec![
            ("n_all".into(), A::CountAll),
            ("n".into(), A::Count(E::Field("x".into()))),
            ("sum_x".into(), A::Sum(E::Field("x".into()))),
            ("avg_x".into(), A::Avg(E::Field("x".into()))),
            ("min_x".into(), A::Min(E::Field("x".into()))),
            ("max_x".into(), A::Max(E::Field("x".into()))),
            ("items".into(), A::Collect(E::Field("x".into()), 4)),
        ],
    })
    .unwrap()
    .iter()
    .map(|row| {
        row.iter()
            .map(|(key, value)| (key.clone(), from_p7(value)))
            .collect()
    })
    .collect()
}

#[test]
fn storage_hql_and_ir_aggregates_match_independent_p7_for_nullable_bags() {
    let directory = TempDir::new().unwrap();
    let storage = open(directory.path());

    for (case, values) in bags_up_to_four().iter().enumerate() {
        let expected = expected_rows(values);
        let mut hql = request(format!("p7-aggregate-{case}-hql"), values);
        hql.ir = None;
        hql.hql = Some(
            "VALUES $xs AS x |> AGG count(*) AS n_all, count(x) AS n, sum(x) AS sum_x, avg(x) AS avg_x, min(x) AS min_x, max(x) AS max_x, collect(x) AS items |> RETURN n_all, n, sum_x, avg_x, min_x, max_x, items"
                .into(),
        );
        hql.language_version = Some(HqlLanguageVersionV2::HqlV2);
        let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql).unwrap() else {
            panic!("HQL aggregate should return rows for case {case}");
        };

        let ir = request(format!("p7-aggregate-{case}-ir"), values);
        let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir).unwrap() else {
            panic!("IR aggregate should return rows for case {case}");
        };

        assert_eq!(
            hql_result.rows, expected,
            "HQL differs from P7 at case {case}"
        );
        assert_eq!(
            ir_result.rows, expected,
            "IR differs from P7 at case {case}"
        );
        assert_eq!(
            hql_result.rows, ir_result.rows,
            "HQL/IR differ at case {case}"
        );
    }
}

fn diagnostic_request(values: &[Option<i64>], grouped: bool, hql: bool) -> QueryRequestV2 {
    let mut req = request("g4-aggregate-diagnostics".into(), values);
    let mut ir = serde_json::to_value(req.ir.take().unwrap()).unwrap();
    let names = ["n_all", "n", "sum_x", "avg_x", "min_x", "max_x", "items"];
    let mut fields = Vec::new();
    if grouped {
        ir["nodes"][1]["config"]["group_by"] =
            json!([{"expression":{"field":{"alias":"x","path":[]}},"as":"key"}]);
        fields.push(json!({"expression":{"field":{"alias":"key","path":[]}},"as":"key"}));
    }
    fields.extend(
        names
            .iter()
            .map(|name| json!({"expression":{"field":{"alias":name,"path":[]}},"as":name})),
    );
    ir["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"p","op":"Project","inputs":["a"],"config":{"fields":fields}}));
    ir["root"] = json!("p");
    req.ir = Some(serde_json::from_value(ir).unwrap());
    if hql {
        req.ir = None;
        req.language_version = Some(HqlLanguageVersionV2::HqlV2);
        let grouping = if grouped {
            "GROUP BY x AS key AGG"
        } else {
            "AGG"
        };
        let key = if grouped { "key, " } else { "" };
        req.hql = Some(format!("VALUES $xs AS x |> {grouping} count(*) AS n_all, count(x) AS n, sum(x) AS sum_x, avg(x) AS avg_x, min(x) AS min_x, max(x) AS max_x, collect(x) AS items |> RETURN {key}n_all, n, sum_x, avg_x, min_x, max_x, items"));
    }
    req.temporal =
        Some(serde_json::from_value(json!({"valid_at":"2026-10-23T00:00:00Z"})).unwrap());
    req
}

fn sorted_rows(rows: Vec<BTreeMap<String, V>>) -> Vec<String> {
    let mut rows: Vec<_> = rows
        .iter()
        .map(|row| serde_json::to_string(row).unwrap())
        .collect();
    rows.sort();
    rows
}

fn diagnostic_rows(values: &[Option<i64>], grouped: bool) -> Vec<BTreeMap<String, V>> {
    use reference::{Aggregate as A, Expr as E, Plan as P, Value as R};

    let input = P::Values(
        values
            .iter()
            .map(|value| BTreeMap::from([("x".into(), value.map(R::I64).unwrap_or(R::Null))]))
            .collect(),
    );
    reference::execute(&P::Aggregate {
        input: Box::new(input),
        keys: if grouped {
            vec![("key".into(), E::Field("x".into()))]
        } else {
            vec![]
        },
        aggregates: vec![
            ("n_all".into(), A::CountAll),
            ("n".into(), A::Count(E::Field("x".into()))),
            ("sum_x".into(), A::Sum(E::Field("x".into()))),
            ("avg_x".into(), A::Avg(E::Field("x".into()))),
            ("min_x".into(), A::Min(E::Field("x".into()))),
            ("max_x".into(), A::Max(E::Field("x".into()))),
            ("items".into(), A::Collect(E::Field("x".into()), 4)),
        ],
    })
    .unwrap()
    .iter()
    .map(|row| {
        row.iter()
            .map(|(key, value)| (key.clone(), from_p7(value)))
            .collect()
    })
    .collect()
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

fn aggregate_policy(storage: &Storage) {
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
                        principal: "reader".into(),
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

/// Catches lost NULL groups/duplicate members, empty-global suppression,
/// frontend counter drift, Plan publication and unauthorized/budgeted partial success.
#[test]
fn storage_aggregate_diagnostics_preserve_p7_groups_counters_authority_and_state() {
    // Group counts are literal, independent of actual execution/counters.
    for (values, groups) in [
        (vec![], 0u64),
        (vec![None, None], 1),
        (vec![Some(1), Some(1), None, Some(2)], 3),
        (vec![Some(-1), Some(2), Some(-1), None], 3),
    ] {
        let directory = TempDir::new().unwrap();
        let storage = open(directory.path());
        aggregate_policy(&storage);
        let before_plan = state(&storage, directory.path());
        let mut cases = Vec::new();
        // Both frontends and grouping modes are planned before execution/publication.
        for grouped in [false, true] {
            let output_count = if grouped { groups } else { 1 };
            let expected = sorted_rows(diagnostic_rows(&values, grouped));
            assert_eq!(expected.len() as u64, output_count);
            for hql in [false, true] {
                let request = diagnostic_request(&values, grouped, hql);
                let mut planned = request.clone();
                planned.explain = Some(ExplainV2::Plan);
                let QueryOutcomeV2::Plan(plan) =
                    storage.query_v2(access(), planned.clone()).unwrap()
                else {
                    panic!("Plan")
                };
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
                cases.push((request, plan, expected.clone(), output_count));
            }
        }
        // Execution may legitimately publish lazily; establish that boundary once
        // before guarding normal/Analyze errors, never before the Plan control.
        storage.publish_generation().unwrap();
        for (request, plan, expected, output_count) in cases {
            let QueryOutcomeV2::Rows(normal) = storage.query_v2(access(), request.clone()).unwrap()
            else {
                panic!("rows")
            };
            assert!(normal.explain.is_none());
            assert_eq!(sorted_rows(normal.rows.clone()), expected);
            let mut analyzed_request = request.clone();
            analyzed_request.explain = Some(ExplainV2::Analyze);
            let QueryOutcomeV2::Rows(analyzed) =
                storage.query_v2(access(), analyzed_request).unwrap()
            else {
                panic!("Analyze")
            };
            assert_eq!(sorted_rows(analyzed.rows.clone()), expected);
            assert_eq!(analyzed.columns, normal.columns);
            assert_eq!(analyzed.semantics, normal.semantics);
            assert_eq!(analyzed.snapshot, normal.snapshot);
            assert_eq!(analyzed.completeness, normal.completeness);
            assert_eq!(analyzed.index_frontiers, normal.index_frontiers);
            assert_eq!(analyzed.cursor, normal.cursor);
            let mut actual = analyzed.explain.unwrap();
            assert_eq!(actual.plan_hash, plan.plan_hash);
            assert_eq!(actual.planner_version, plan.planner_version);
            assert_eq!(actual.plan.len(), 3);
            for (op, input, output) in [
                (QueryOpV2::Values, 0, values.len() as u64),
                (QueryOpV2::Aggregate, values.len() as u64, output_count),
                (QueryOpV2::Project, output_count, output_count),
            ] {
                let nodes: Vec<_> = actual
                    .plan
                    .iter_mut()
                    .filter(|node| node.logical_op == op)
                    .collect();
                assert_eq!(nodes.len(), 1, "one {op:?}");
                let node = nodes.into_iter().next().unwrap();
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
                    assert_eq!(error.detail.unwrap()["reason"], "output_limit");
                    assert_state(&storage, directory.path(), &before);
                    limited.budget =
                        Some(serde_json::from_value(json!({"max_result_bytes":1048576})).unwrap());
                    let QueryOutcomeV2::Rows(empty) = storage.query_v2(access(), limited).unwrap()
                    else {
                        panic!("empty rows")
                    };
                    assert!(empty.rows.is_empty());
                    assert_eq!(empty.completeness, normal.completeness);
                    assert_state(&storage, directory.path(), &before);
                }
            }
        }
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
fn aggregate_result_metadata_has_independent_fixture_expectations() {
    let row = |key: Option<V>, n_all, n, sum_x: V, avg_x: V, min_x: V, max_x: V, items: Vec<V>| {
        let mut row = BTreeMap::from([
            ("n_all".into(), V::I64(n_all)),
            ("n".into(), V::I64(n)),
            ("sum_x".into(), sum_x),
            ("avg_x".into(), avg_x),
            ("min_x".into(), min_x),
            ("max_x".into(), max_x),
            ("items".into(), V::List(items)),
        ]);
        if let Some(k) = key {
            row.insert("key".into(), k);
        }
        row
    };
    for (case, input) in [
        ("mixed", vec![Some(2), None, Some(2)]),
        ("all-null", vec![None, None]),
        ("empty", vec![]),
        ("distinct", vec![Some(-1), Some(2)]),
    ] {
        for grouped in [false, true] {
            let expected = match (case, grouped) {
                ("mixed", false) => vec![row(
                    None,
                    3,
                    2,
                    V::I64(4),
                    V::F64(2.0),
                    V::I64(2),
                    V::I64(2),
                    vec![V::I64(2), V::Null, V::I64(2)],
                )],
                ("mixed", true) => vec![
                    row(
                        Some(V::I64(2)),
                        2,
                        2,
                        V::I64(4),
                        V::F64(2.0),
                        V::I64(2),
                        V::I64(2),
                        vec![V::I64(2), V::I64(2)],
                    ),
                    row(
                        Some(V::Null),
                        1,
                        0,
                        V::Null,
                        V::Null,
                        V::Null,
                        V::Null,
                        vec![V::Null],
                    ),
                ],
                ("all-null", g) => vec![row(
                    if g { Some(V::Null) } else { None },
                    2,
                    0,
                    V::Null,
                    V::Null,
                    V::Null,
                    V::Null,
                    vec![V::Null, V::Null],
                )],
                ("empty", false) => {
                    vec![row(None, 0, 0, V::Null, V::Null, V::Null, V::Null, vec![])]
                }
                ("empty", true) => vec![],
                ("distinct", false) => vec![row(
                    None,
                    2,
                    2,
                    V::I64(1),
                    V::F64(0.5),
                    V::I64(-1),
                    V::I64(2),
                    vec![V::I64(-1), V::I64(2)],
                )],
                ("distinct", true) => vec![
                    row(
                        Some(V::I64(-1)),
                        1,
                        1,
                        V::I64(-1),
                        V::F64(-1.0),
                        V::I64(-1),
                        V::I64(-1),
                        vec![V::I64(-1)],
                    ),
                    row(
                        Some(V::I64(2)),
                        1,
                        1,
                        V::I64(2),
                        V::F64(2.0),
                        V::I64(2),
                        V::I64(2),
                        vec![V::I64(2)],
                    ),
                ],
                _ => panic!("fixture"),
            };
            assert_eq!(
                sorted_rows(diagnostic_rows(&input, grouped)),
                sorted_rows(expected.clone())
            );
            let mut columns = vec![];
            if grouped {
                columns.push(json!({"name":"key","type":"Nullable<I64>","nullable":true}));
            }
            for (name, ty, nullable) in [
                ("n_all", "I64", false),
                ("n", "I64", false),
                ("sum_x", "Nullable<I64>", true),
                ("avg_x", "Nullable<F64Finite>", true),
                ("min_x", "Nullable<I64>", true),
                ("max_x", "Nullable<I64>", true),
                ("items", "List<Nullable<I64>>", false),
            ] {
                columns.push(json!({"name":name,"type":ty,"nullable":nullable}));
            }
            for hql in [false, true] {
                let directory = TempDir::new().unwrap();
                let storage = open(directory.path());
                aggregate_policy(&storage);
                let mut req = diagnostic_request(&input, grouped, hql);
                req.request_id = format!("metadata-aggregate-{case}-{grouped}-{hql}");
                qualify_metadata_fixture(
                    &storage,
                    directory.path(),
                    req,
                    json!(columns),
                    expected.clone(),
                    true,
                );
            }
        }
    }
}
