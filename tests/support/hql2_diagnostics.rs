//! Test-only checks shared by source, history, graph and vector diagnostic fixtures.
use genesis_block_native::{
    query::hql2::{result::CompletionStatusV2, QueryErrorV2, QueryOutcomeV2, QueryResultV2},
    uee_v2::{ExplainV2, QueryOpV2, QueryRequestV2},
    AccessContext, Storage,
};
use serde_json::{json, Value};

fn access() -> AccessContext {
    AccessContext {
        principal: "diagnostic-reader".into(),
        namespace: "default".into(),
    }
}

pub fn measured(result: &QueryResultV2, node: &str, field: &str, unit: &str, expected: u64) {
    assert_eq!(
        reading(result, node)[field],
        json!({"unit":unit,"measurement":{"status":"measured","value":expected}}),
        "{node}.{field}"
    );
}

/// The expectation must come from fixture setup or an independent oracle.
// Each integration target compiles this shared module; graph/vector targets do not use this helper.
#[allow(dead_code)]
pub fn history_source_rows(result: &QueryResultV2, expected: u64) {
    let sources: Vec<_> = result
        .explain
        .as_ref()
        .unwrap()
        .plan
        .iter()
        .filter(|node| {
            matches!(
                node.logical_op,
                QueryOpV2::HistoryScan | QueryOpV2::ChangeScan
            )
        })
        .collect();
    assert_eq!(sources.len(), 1);
    measured(result, &sources[0].id, "rows_in", "rows", 0);
    measured(result, &sources[0].id, "rows_out", "rows", expected);
}

fn reading(result: &QueryResultV2, node: &str) -> Value {
    serde_json::to_value(
        result
            .explain
            .as_ref()
            .unwrap()
            .plan
            .iter()
            .find(|n| n.id == node)
            .unwrap()
            .actual
            .as_ref()
            .unwrap(),
    )
    .unwrap()
}

/// Execute one fixture through the unchanged Storage/P6 boundary in all modes.
pub fn three_modes(db: &Storage, request: QueryRequestV2) -> QueryResultV2 {
    three_modes_with_access(db, access(), request)
}

pub fn three_modes_with_access(
    db: &Storage,
    actor: AccessContext,
    request: QueryRequestV2,
) -> QueryResultV2 {
    try_three_modes(db, actor, request).unwrap()
}

/// Data errors must agree in normal/ANALYZE; plan-only does not open sources.
pub fn try_three_modes(
    db: &Storage,
    actor: AccessContext,
    request: QueryRequestV2,
) -> Result<QueryResultV2, QueryErrorV2> {
    let outcome = match db.query_v2(actor.clone(), request.clone()) {
        Ok(outcome) => outcome,
        Err(error) => {
            let mut analyzed = request;
            analyzed.explain = Some(ExplainV2::Analyze);
            let analyze_error = db.query_v2(actor, analyzed).unwrap_err();
            assert_eq!(analyze_error.code, error.code);
            assert_eq!(analyze_error.stage, error.stage);
            assert_eq!(analyze_error.detail, error.detail);
            return Err(error);
        }
    };
    let QueryOutcomeV2::Rows(normal) = outcome else {
        panic!("normal rows")
    };
    assert!(normal.explain.is_none());
    let mut plan_request = request.clone();
    plan_request.explain = Some(ExplainV2::Plan);
    let QueryOutcomeV2::Plan(plan) = db.query_v2(actor.clone(), plan_request).unwrap() else {
        panic!("plan")
    };
    assert!(plan
        .plan
        .iter()
        .all(|n| n.actual.is_none() && n.estimates.is_none()));
    let mut analyze_request = request;
    analyze_request.explain = Some(ExplainV2::Analyze);
    let QueryOutcomeV2::Rows(analyzed) = db.query_v2(actor, analyze_request).unwrap() else {
        panic!("analyze rows")
    };
    assert_eq!(normal.rows, analyzed.rows);
    assert_eq!(normal.columns, analyzed.columns);
    assert_eq!(normal.semantics, analyzed.semantics);
    assert_eq!(normal.completeness, analyzed.completeness);
    assert_eq!(normal.snapshot.database_id, analyzed.snapshot.database_id);
    assert_eq!(normal.snapshot.tx, analyzed.snapshot.tx);
    assert_eq!(
        normal.snapshot.catalog_generation,
        analyzed.snapshot.catalog_generation
    );
    assert_eq!(
        normal.snapshot.policy_version,
        analyzed.snapshot.policy_version
    );
    assert_eq!(analyzed.completeness.status, CompletionStatusV2::Complete);
    let mut actual_plan = analyzed.explain.clone().unwrap();
    assert_eq!(actual_plan.plan_hash, plan.plan_hash);
    assert_eq!(actual_plan.planner_version, plan.planner_version);
    for node in &mut actual_plan.plan {
        let actual = serde_json::to_value(node.actual.take().unwrap()).unwrap();
        for (field, unit, reason) in [
            ("bytes_read", "bytes", "not_instrumented"),
            ("work_units", "work_units", "not_instrumented"),
            ("memory_peak_bytes", "bytes", "not_instrumented"),
            ("index_probes", "index_probes", "not_applicable"),
            ("spill_bytes", "bytes", "not_applicable"),
        ] {
            assert_eq!(
                actual[field],
                json!({"unit":unit,"measurement":{"status":"unknown","reason":reason}}),
                "{}.{}",
                node.id,
                field
            );
        }
        for field in ["rows_in", "rows_out"] {
            assert_eq!(actual[field]["unit"], "rows");
            assert_eq!(actual[field]["measurement"]["status"], "measured");
            assert!(actual[field]["measurement"]["value"].as_u64().is_some());
        }
        for (field, applicable) in [
            (
                "graph_expansions",
                matches!(node.logical_op, QueryOpV2::Match | QueryOpV2::Expand),
            ),
            (
                "distance_evaluations",
                matches!(node.logical_op, QueryOpV2::Knn | QueryOpV2::Rerank),
            ),
        ] {
            assert_eq!(actual[field]["unit"], field);
            if applicable {
                assert_eq!(actual[field]["measurement"]["status"], "measured");
                assert!(actual[field]["measurement"]["value"].as_u64().is_some());
            } else {
                assert_eq!(
                    actual[field]["measurement"],
                    json!({"status":"unknown","reason":"not_applicable"})
                );
            }
        }
        let source = matches!(
            node.logical_op,
            QueryOpV2::NodeScan
                | QueryOpV2::EdgeScan
                | QueryOpV2::RowScan
                | QueryOpV2::AnnotationScan
                | QueryOpV2::HistoryScan
                | QueryOpV2::ChangeScan
        );
        assert_eq!(actual["elapsed_ns"]["unit"], "nanoseconds");
        if source {
            assert_eq!(
                actual["elapsed_ns"]["measurement"],
                json!({"status":"unknown","reason":"not_instrumented"})
            );
        } else {
            assert_eq!(actual["elapsed_ns"]["measurement"]["status"], "measured");
            assert!(actual["elapsed_ns"]["measurement"]["value"]
                .as_u64()
                .is_some());
        }
        assert_eq!(actual["sampling"], "complete");
        assert_eq!(actual["clock_source"], "monotonic_instant");
        assert_eq!(actual["elapsed_scope"], "operator_execution");
    }
    assert_eq!(actual_plan.plan, plan.plan);
    Ok(analyzed)
}
