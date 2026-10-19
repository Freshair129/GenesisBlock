use genesis_block_native::{
    query::hql2::{QueryOutcomeV2, QueryResultV2},
    uee_v2::{ExplainV2, QueryRequestV2},
    AccessContext, OpenOptions, Storage,
};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;

fn open(path: &Path) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string_lossy().into(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        retention: Some("full".into()),
        vector_dim: Some(2),
    })
    .unwrap()
}

fn access() -> AccessContext {
    AccessContext {
        principal: "g4-reader".into(),
        namespace: "default".into(),
    }
}

fn values_request(request_id: &str, explain: ExplainV2) -> QueryRequestV2 {
    serde_json::from_value(json!({
        "contract_version":"genesis.api.v2",
        "request_id":request_id,
        "namespace":"default",
        "explain": explain,
        "ir": {
            "contract_version":"query-ir.v2",
            "nodes":[{
                "id":"values",
                "op":"Values",
                "inputs":[],
                "config":{"param":"xs","as":"x"}
            }],
            "root":"values",
            "parameter_types":{"xs":"List<I64>"}
        },
        "params":{"xs":{"type":"List<I64>","value":["2","1","2"]}}
    }))
    .unwrap()
}

fn explain(result: QueryResultV2) -> genesis_block_native::query::hql2::ExplainResultV2 {
    result.explain.expect("ANALYZE must include an explanation")
}

#[test]
fn g4_explain_plan_identity_is_stable_and_shared_with_analyze() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());

    let QueryOutcomeV2::Plan(plan_a) = db
        .query_v2(access(), values_request("g4-plan-a", ExplainV2::Plan))
        .unwrap()
    else {
        panic!("EXPLAIN must return a plan")
    };
    let QueryOutcomeV2::Plan(plan_b) = db
        .query_v2(access(), values_request("g4-plan-b", ExplainV2::Plan))
        .unwrap()
    else {
        panic!("EXPLAIN must return a plan")
    };

    assert_eq!(plan_a.contract_version, "genesis.api.v2");
    assert_eq!(plan_a.planner_version, "hql2-rule-v1");
    assert_eq!(plan_a.plan_hash, plan_b.plan_hash);
    assert_eq!(plan_a.plan_hash.len(), 64);
    assert!(plan_a
        .plan_hash
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
    assert!(plan_a.plan.iter().all(|node| node.actual.is_none()));

    let QueryOutcomeV2::Rows(analyzed) = db
        .query_v2(access(), values_request("g4-analyze", ExplainV2::Analyze))
        .unwrap()
    else {
        panic!("ANALYZE must return rows")
    };
    let analyzed_plan = explain(analyzed);
    assert_eq!(analyzed_plan.contract_version, plan_a.contract_version);
    assert_eq!(analyzed_plan.planner_version, plan_a.planner_version);
    assert_eq!(analyzed_plan.plan_hash, plan_a.plan_hash);
    assert!(analyzed_plan
        .plan
        .iter()
        .all(|node| node.actual.as_ref().is_some_and(|actual| matches!(
            actual.rows_out.measurement,
            genesis_block_native::query::hql2::result::CounterMeasurementV2::Measured { .. }
        ))));
}

#[test]
fn g4_analyze_serializes_the_closed_typed_counter_contract() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let QueryOutcomeV2::Rows(result) = db
        .query_v2(
            access(),
            values_request("g4-counter-schema", ExplainV2::Analyze),
        )
        .unwrap()
    else {
        panic!("ANALYZE must execute");
    };
    let plan = explain(result);
    let actual = serde_json::to_value(plan.plan[0].actual.as_ref().unwrap()).unwrap();
    let expected_keys: std::collections::BTreeSet<_> = [
        "rows_in",
        "rows_out",
        "bytes_read",
        "work_units",
        "index_probes",
        "distance_evaluations",
        "graph_expansions",
        "memory_peak_bytes",
        "spill_bytes",
        "elapsed_ns",
        "sampling",
        "clock_source",
        "elapsed_scope",
    ]
    .into_iter()
    .collect();
    assert_eq!(
        actual
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        expected_keys
    );
    assert_eq!(
        actual["rows_in"],
        json!({"unit":"rows","measurement":{"status":"measured","value":0}})
    );
    assert_eq!(
        actual["rows_out"],
        json!({"unit":"rows","measurement":{"status":"measured","value":3}})
    );
    for (key, unit, reason) in [
        ("bytes_read", "bytes", "not_instrumented"),
        ("work_units", "work_units", "not_instrumented"),
        ("index_probes", "index_probes", "not_applicable"),
        (
            "distance_evaluations",
            "distance_evaluations",
            "not_applicable",
        ),
        ("graph_expansions", "graph_expansions", "not_applicable"),
        ("memory_peak_bytes", "bytes", "not_instrumented"),
        ("spill_bytes", "bytes", "not_applicable"),
    ] {
        assert_eq!(
            actual[key],
            json!({"unit":unit,"measurement":{"status":"unknown","reason":reason}})
        );
    }
    assert_eq!(actual["elapsed_ns"]["unit"], "nanoseconds");
    assert_eq!(actual["elapsed_ns"]["measurement"]["status"], "measured");
    assert!(actual["elapsed_ns"]["measurement"]["value"]
        .as_u64()
        .is_some());
    assert_eq!(actual["sampling"], "complete");
    assert_eq!(actual["clock_source"], "monotonic_instant");
    assert_eq!(actual["elapsed_scope"], "operator_execution");
}
