//! Storage-bound HQL2 and typed-IR set semantics against independent P7.
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

fn parameter(values: &[Option<i64>]) -> Value {
    let values: Vec<Value> = values
        .iter()
        .map(|value| {
            value
                .map(|value| json!(value.to_string()))
                .unwrap_or(Value::Null)
        })
        .collect();
    json!({"type":"List<Nullable<I64>>","value":values})
}

fn request(request_id: String, left: &[Option<i64>], right: &[Option<i64>]) -> QueryRequestV2 {
    let ir: QueryIrV2 = serde_json::from_value(json!({
        "contract_version":"query-ir.v2",
        "nodes":[
            {"id":"l","op":"Values","inputs":[],"config":{"param":"left","as":"x"}},
            {"id":"r","op":"Values","inputs":[],"config":{"param":"right","as":"x"}},
            {"id":"u","op":"UnionAll","inputs":["l","r"],"config":{}},
            {"id":"s","op":"Sort","inputs":["u"],"config":{"keys":[{
                "expression":{"field":{"alias":"x","path":[]}},
                "direction":"asc","nulls":"last"
            }]}}
        ],
        "root":"s",
        "parameter_types":{
            "left":"List<Nullable<I64>>",
            "right":"List<Nullable<I64>>"
        }
    }))
    .unwrap();

    QueryRequestV2 {
        contract_version: "genesis.api.v2".into(),
        request_id,
        namespace: "default".into(),
        hql: None,
        ir: Some(ir),
        language_version: None,
        params: BTreeMap::from([
            ("left".into(), parameter(left)),
            ("right".into(), parameter(right)),
        ]),
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

fn bags_up_to_two() -> Vec<Vec<Option<i64>>> {
    let alphabet = [None, Some(-1), Some(2)];
    let mut bags = vec![Vec::new()];
    for len in 1..=2 {
        for seed in 0..3usize.pow(len as u32) {
            let mut digits = seed;
            let bag = (0..len)
                .map(|_| {
                    let value = alphabet[digits % alphabet.len()];
                    digits /= alphabet.len();
                    value
                })
                .collect();
            bags.push(bag);
        }
    }
    bags
}

fn p7_rows(values: &[Option<i64>]) -> Vec<BTreeMap<String, reference::Value>> {
    values
        .iter()
        .map(|value| {
            BTreeMap::from([(
                "x".into(),
                value
                    .map(reference::Value::I64)
                    .unwrap_or(reference::Value::Null),
            )])
        })
        .collect()
}

fn expected_rows(left: &[Option<i64>], right: &[Option<i64>]) -> Vec<BTreeMap<String, V>> {
    use reference::{Expr, Plan, SortKey};

    reference::execute(&Plan::Sort(
        Box::new(Plan::UnionAll(
            Box::new(Plan::Values(p7_rows(left))),
            Box::new(Plan::Values(p7_rows(right))),
        )),
        vec![SortKey {
            expr: Expr::Field("x".into()),
            descending: false,
            nulls_first: false,
        }],
    ))
    .unwrap()
    .iter()
    .map(|row| {
        BTreeMap::from([(
            "x".into(),
            match row.get("x").unwrap() {
                reference::Value::Null => V::Null,
                reference::Value::I64(value) => V::I64(*value),
                other => panic!("unexpected P7 set value: {other:?}"),
            },
        )])
    })
    .collect()
}

#[test]
fn storage_hql_and_ir_union_all_match_p7_for_nullable_bag_pairs() {
    let directory = TempDir::new().unwrap();
    let storage = open(directory.path());
    let bags = bags_up_to_two();

    for (case, (left, right)) in bags
        .iter()
        .flat_map(|left| bags.iter().map(move |right| (left, right)))
        .enumerate()
    {
        let expected = expected_rows(left, right);
        let mut hql = request(format!("p7-union-{case}-hql"), left, right);
        hql.ir = None;
        hql.hql = Some(
            "UNION ALL { VALUES $left AS x |> RETURN x }{ VALUES $right AS x |> RETURN x } |> ORDER BY x ASC NULLS LAST |> RETURN x".into(),
        );
        hql.language_version = Some(HqlLanguageVersionV2::HqlV2);
        let QueryOutcomeV2::Rows(hql_result) = storage.query_v2(access(), hql).unwrap() else {
            panic!("HQL UNION ALL should return rows for case {case}");
        };

        let ir = request(format!("p7-union-{case}-ir"), left, right);
        let QueryOutcomeV2::Rows(ir_result) = storage.query_v2(access(), ir).unwrap() else {
            panic!("typed-IR UnionAll should return rows for case {case}");
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

fn diagnostic_request(left: &[Option<i64>], right: &[Option<i64>], hql: bool) -> QueryRequestV2 {
    let mut req = request("g4-union-diagnostics".into(), left, right);
    let mut ir = serde_json::to_value(req.ir.take().unwrap()).unwrap();
    let mut nodes = ir["nodes"].as_array().unwrap().clone();
    let field = json!({"expression":{"field":{"alias":"x","path":[]}},"as":"x"});
    let project = |id: &str, input: &str| json!({"id":id,"op":"Project","inputs":[input],"config":{"fields":[field]}});
    nodes[2]["inputs"] = json!(["lp", "rp"]);
    ir["nodes"] = json!([
        nodes[0],
        nodes[1],
        project("lp", "l"),
        project("rp", "r"),
        nodes[2],
        nodes[3],
        project("p", "s")
    ]);
    ir["root"] = json!("p");
    req.ir = Some(serde_json::from_value(ir).unwrap());
    if hql {
        req.ir = None;
        req.language_version = Some(HqlLanguageVersionV2::HqlV2);
        req.hql = Some("UNION ALL { VALUES $left AS x |> RETURN x } { VALUES $right AS x |> RETURN x } |> ORDER BY x ASC NULLS LAST |> RETURN x".into());
    }
    // Parameter Values do not depend on record insertion time. Reuse per mode.
    req.temporal =
        Some(serde_json::from_value(json!({"valid_at":"2026-10-23T00:00:00Z"})).unwrap());
    req
}

/// Locate each branch by input lineage, not generated HQL node IDs.
fn expected_counts(
    plan: &genesis_block_native::query::hql2::ExplainResultV2,
    left: u64,
    right: u64,
    total: u64,
) -> Vec<(String, QueryOpV2, u64, u64)> {
    let find = |id: &str| plan.plan.iter().find(|node| node.id == id).unwrap();
    let root = find(&plan.root);
    assert_eq!(root.logical_op, QueryOpV2::Project);
    assert_eq!(root.inputs.len(), 1);
    let sort = find(&root.inputs[0]);
    assert_eq!(sort.logical_op, QueryOpV2::Sort);
    assert_eq!(sort.inputs.len(), 1);
    let union = find(&sort.inputs[0]);
    assert_eq!(union.logical_op, QueryOpV2::UnionAll);
    assert_eq!(union.inputs.len(), 2);
    let mut counts = vec![
        (root.id.clone(), QueryOpV2::Project, total, total),
        (sort.id.clone(), QueryOpV2::Sort, total, total),
        (union.id.clone(), QueryOpV2::UnionAll, total, total),
    ];
    for (id, size) in union.inputs.iter().zip([left, right]) {
        let branch = find(id);
        assert_eq!(branch.logical_op, QueryOpV2::Project);
        assert_eq!(branch.inputs.len(), 1);
        let source = find(&branch.inputs[0]);
        assert_eq!(source.logical_op, QueryOpV2::Values);
        assert!(source.inputs.is_empty());
        counts.push((branch.id.clone(), QueryOpV2::Project, size, size));
        counts.push((source.id.clone(), QueryOpV2::Values, 0, size));
    }
    assert_eq!(plan.plan.len(), 7);
    assert_eq!(
        counts
            .iter()
            .map(|(id, _, _, _)| id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        7
    );
    counts
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

fn union_policy(storage: &Storage) {
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

/// Catches duplicate/NULL loss, empty/branch counter drift, Sort order loss,
/// Plan publication and unauthorized/budgeted partial success.
#[test]
fn storage_union_all_diagnostics_preserve_p7_bags_counters_authority_and_state() {
    for (left, right, literal) in [
        (
            vec![Some(1), None, Some(1)],
            vec![None, Some(2)],
            vec![V::I64(1), V::I64(1), V::I64(2), V::Null, V::Null],
        ),
        (vec![], vec![Some(2)], vec![V::I64(2)]),
        (vec![None], vec![], vec![V::Null]),
        (vec![], vec![], vec![]),
    ] {
        let output_count = literal.len() as u64;
        let expected: Vec<_> = literal
            .into_iter()
            .map(|v| BTreeMap::from([("x".into(), v)]))
            .collect();
        assert_eq!(
            expected_rows(&left, &right),
            expected,
            "independent P7 and literal ordered rows"
        );
        let directory = TempDir::new().unwrap();
        let storage = open(directory.path());
        union_policy(&storage);
        let before_plan = state(&storage, directory.path());
        let mut cases = Vec::new();
        // Both frontends' allowed/denied Plan controls precede any query execution
        // or explicit publication; observing files/frontier does not publish.
        for hql in [false, true] {
            let request = diagnostic_request(&left, &right, hql);
            let mut planned = request.clone();
            planned.explain = Some(ExplainV2::Plan);
            let QueryOutcomeV2::Plan(plan) = storage.query_v2(access(), planned.clone()).unwrap()
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
            let counts =
                expected_counts(&actual, left.len() as u64, right.len() as u64, output_count);
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
fn union_result_metadata_has_independent_fixture_expectations() {
    for (case, left, right, literal) in [
        (
            "both",
            vec![Some(2), None, Some(2)],
            vec![Some(1), None],
            vec![V::I64(1), V::I64(2), V::Null],
        ),
        (
            "left-empty",
            vec![],
            vec![None, Some(2), Some(2)],
            vec![V::I64(2), V::Null],
        ),
        (
            "right-empty",
            vec![Some(-1), None],
            vec![],
            vec![V::I64(-1), V::Null],
        ),
        ("both-empty", vec![], vec![], vec![]),
    ] {
        use reference::{Expr as E, Plan as P, SortKey};
        let oracle = P::Sort(
            Box::new(P::Distinct(Box::new(P::UnionAll(
                Box::new(P::Values(p7_rows(&left))),
                Box::new(P::Values(p7_rows(&right))),
            )))),
            vec![SortKey {
                expr: E::Field("x".into()),
                descending: false,
                nulls_first: false,
            }],
        );
        let oracle: Vec<BTreeMap<String, V>> = reference::execute(&oracle)
            .unwrap()
            .iter()
            .map(|row| {
                BTreeMap::from([(
                    "x".into(),
                    match row["x"] {
                        reference::Value::Null => V::Null,
                        reference::Value::I64(n) => V::I64(n),
                        _ => panic!("P7 scalar"),
                    },
                )])
            })
            .collect();
        let expected: Vec<BTreeMap<String, V>> = literal
            .into_iter()
            .map(|v| BTreeMap::from([("x".into(), v)]))
            .collect();
        assert_eq!(oracle, expected);
        for hql in [false, true] {
            let directory = TempDir::new().unwrap();
            let storage = open(directory.path());
            union_policy(&storage);
            let mut req = diagnostic_request(&left, &right, hql);
            req.request_id = format!("metadata-union-{case}-{hql}");
            if hql {
                req.hql=Some("UNION ALL { VALUES $left AS x |> RETURN x } { VALUES $right AS x |> RETURN x } |> DISTINCT |> ORDER BY x ASC NULLS LAST |> RETURN x".into());
            } else {
                let mut ir = serde_json::to_value(req.ir.take().unwrap()).unwrap();
                let nodes = ir["nodes"].as_array_mut().unwrap();
                let sort = nodes.iter_mut().find(|n| n["op"] == "Sort").unwrap();
                sort["inputs"] = json!(["d"]);
                nodes.push(json!({"id":"d","op":"Distinct","inputs":["u"],"config":{}}));
                req.ir = Some(serde_json::from_value(ir).unwrap());
            }
            qualify_metadata_fixture(
                &storage,
                directory.path(),
                req,
                json!([{"name":"x","type":"Nullable<I64>","nullable":true}]),
                expected.clone(),
                false,
            );
        }
    }
}
