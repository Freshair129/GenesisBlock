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
    assert!(analyzed_plan.plan.iter().all(|node| node
        .actual
        .as_ref()
        .is_some_and(|actual| actual.output_rows.is_some())));
}
