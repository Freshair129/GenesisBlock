use genesis_block_native::{query::hql2::QueryOutcomeV2, uee_v2::QueryRequestV2, *};
use serde_json::json;
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
fn values_request() -> QueryRequestV2 {
    serde_json::from_value(json!({"contract_version":"genesis.api.v2","request_id":"p8","namespace":"default",
        "ir":{"contract_version":"query-ir.v2","nodes":[{"id":"v","op":"Values","inputs":[],"config":{"param":"xs","as":"x"}}],"root":"v","parameter_types":{"xs":"List<I64>"}},
        "params":{"xs":{"type":"List<I64>","value":["2","1","2"]}}})).unwrap()
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

#[test]
fn explain_never_publishes_generation_or_changes_files_on_fresh_or_changed_store() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    for changed in [false, true] {
        if changed {
            db.add_node(NodeInput {
                id: Some("n".into()),
                labels: vec![],
                props: None,
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
            db.flush_index();
        }
        let frontier = db.stable_frontier();
        let files = disk_tree(dir.path());
        let mut request = values_request();
        request.explain = Some(uee_v2::ExplainV2::Plan);
        let QueryOutcomeV2::Plan(plan) = db.query_v2(access(), request).unwrap() else {
            panic!("must not execute")
        };
        assert_eq!(plan.catalog.observed_frontier, frontier);
        assert!(plan
            .plan
            .iter()
            .all(|node| node.actual.is_none() && node.estimates.is_none()));
        assert_eq!(db.stable_frontier(), frontier);
        let after = disk_tree(dir.path());
        assert_eq!(
            after.keys().collect::<Vec<_>>(),
            files.keys().collect::<Vec<_>>()
        );
        for (name, bytes) in files {
            assert!(after[&name] == bytes, "EXPLAIN changed {name}");
        }
    }
}

#[test]
fn values_execute_with_real_snapshot_bags_and_closed_encoding() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let before = db.stable_frontier();
    let QueryOutcomeV2::Rows(result) = db.query_v2(access(), values_request()).unwrap() else {
        panic!()
    };
    let encoded = serde_json::to_value(&result).unwrap();
    assert_eq!(
        encoded["rows"],
        json!([{"x":{"type":"I64","value":"2"}},{"x":{"type":"I64","value":"1"}},{"x":{"type":"I64","value":"2"}}])
    );
    assert_eq!(encoded["snapshot"]["tx"], before.to_string());
    assert_ne!(encoded["snapshot"]["catalog_generation"], "0");
    assert!(encoded["snapshot"]["database_id"].as_str().unwrap().len() == 64);
    assert_eq!(encoded["cursor"], json!(null));
    assert_eq!(encoded["semantics"]["candidate_search"], "exact");
    assert_eq!(encoded["completeness"]["status"], "complete");
    assert!(encoded.get("timings").is_none());
}

#[test]
fn namespace_authorization_precedes_even_malformed_hql_or_ir() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    db.replace_access_policy(
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
            grants: vec![AccessGrant {
                principal: "reader".into(),
                action: AccessAction::Read,
                resource: AccessResource::Node("n".into()),
            }],
        },
    )
    .unwrap();
    let before = db.stable_frontier();
    let mut request = values_request();
    request.ir = None;
    request.hql = Some("not a query".into());
    request.language_version = Some(uee_v2::HqlLanguageVersionV2::HqlV2);
    assert_eq!(
        db.query_v2(access(), request).unwrap_err().code,
        "FORBIDDEN"
    );
    assert_eq!(db.stable_frontier(), before);
}

#[test]
fn unsupported_source_and_future_frontier_fail_before_publication() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let before = db.stable_frontier();
    let mut request = values_request();
    request.required_frontier = Some(u64::MAX.to_string());
    let error = db.query_v2(access(), request).unwrap_err();
    assert_eq!(error.code, "INDEX_COVERAGE_TIMEOUT");
    assert!(error.retryable);
    let mut request = values_request();
    request.ir.as_mut().unwrap().nodes[0].op = uee_v2::QueryOpV2::AnnotationScan;
    request.ir.as_mut().unwrap().nodes[0].config = BTreeMap::from([("as".into(), json!("a"))]);
    assert_eq!(
        db.query_v2(access(), request).unwrap_err().code,
        "CAPABILITY_UNSUPPORTED"
    );
    assert_eq!(db.stable_frontier(), before);
}

#[test]
fn hql_and_ir_use_the_same_values_pipeline_and_no_parameter_substitution() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let QueryOutcomeV2::Rows(ir) = db.query_v2(access(), values_request()).unwrap() else {
        panic!()
    };
    let mut request = values_request();
    request.ir = None;
    request.hql = Some("VALUES $xs AS x |> RETURN x".into());
    request.language_version = Some(uee_v2::HqlLanguageVersionV2::HqlV2);
    let QueryOutcomeV2::Rows(hql) = db.query_v2(access(), request).unwrap() else {
        panic!()
    };
    assert_eq!(ir.rows, hql.rows);
    assert_eq!(ir.columns, hql.columns);
}

#[test]
fn textual_explain_cannot_be_silently_overridden_by_envelope() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let before = db.stable_frontier();
    let mut request = values_request();
    request.ir = None;
    request.hql = Some("EXPLAIN VALUES $xs AS x |> RETURN x".into());
    request.language_version = Some(uee_v2::HqlLanguageVersionV2::HqlV2);
    request.explain = Some(uee_v2::ExplainV2::None);
    assert_eq!(
        db.query_v2(access(), request).unwrap_err().code,
        "BIND_ERROR"
    );
    assert_eq!(db.stable_frontier(), before);
}

#[test]
fn analyze_reports_measured_rows_and_budget_failures_never_return_partial_rows() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let mut request = values_request();
    request.explain = Some(uee_v2::ExplainV2::Analyze);
    let QueryOutcomeV2::Rows(result) = db.query_v2(access(), request).unwrap() else {
        panic!()
    };
    let plan = result.explain.unwrap();
    assert_eq!(plan.plan[0].actual.as_ref().unwrap().output_rows, Some(3));
    assert!(plan.plan[0]
        .actual
        .as_ref()
        .unwrap()
        .distance_evaluations
        .is_none());
    for budget in [
        json!({"max_result_rows":1}),
        json!({"max_result_bytes":1}),
        json!({"max_memory_bytes":1}),
    ] {
        let mut request = values_request();
        request.budget = Some(serde_json::from_value(budget).unwrap());
        assert_eq!(
            db.query_v2(access(), request).unwrap_err().code,
            "QUERY_BUDGET_EXCEEDED"
        );
    }
    assert!(db.query_v2(access(), values_request()).is_ok());
}

#[test]
fn deeply_nested_request_refusal_does_not_overflow_on_cleanup() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let mut request = values_request();
    let mut value = serde_json::Value::Null;
    for _ in 0..10_000 {
        value = serde_json::Value::Array(vec![value]);
    }
    request.params.insert("deep".into(), value);
    let before = db.stable_frontier();
    assert_eq!(
        db.query_v2(access(), request).unwrap_err().code,
        "BIND_ERROR"
    );
    assert_eq!(db.stable_frontier(), before);
}

#[test]
fn scalar_hql_filter_project_aggregate_matches_expected() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let mut request = values_request();
    request.ir = None;
    request.language_version = Some(uee_v2::HqlLanguageVersionV2::HqlV2);
    request.hql=Some("VALUES $xs AS x |> FILTER x > 1 |> PROJECT x + 3 AS y |> AGG sum(y) AS total, count(*) AS n |> RETURN total, n".into());
    let QueryOutcomeV2::Rows(result) = db.query_v2(access(), request).unwrap() else {
        panic!()
    };
    assert_eq!(
        serde_json::to_value(result.rows).unwrap(),
        json!([{"total":{"type":"I64","value":"10"},"n":{"type":"I64","value":"2"}}])
    );
}

#[test]
fn explain_reserves_propagated_schema_before_binding() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let before = db.stable_frontier();
    let mut request = values_request();
    request.explain = Some(uee_v2::ExplainV2::Plan);
    request.budget = Some(serde_json::from_value(json!({"max_memory_bytes":1048576})).unwrap());
    let ir = request.ir.as_mut().unwrap();
    ir.nodes[0]
        .config
        .insert("as".into(), json!("x".repeat(16_384)));
    let mut parent = "v".to_string();
    for i in 0..99 {
        let id = format!("n{i}");
        ir.nodes.push(uee_v2::QueryNodeV2 {
            id: id.clone(),
            op: uee_v2::QueryOpV2::Take,
            inputs: vec![parent],
            config: BTreeMap::from([("count".into(), json!(0))]),
        });
        parent = id;
    }
    ir.root = parent;
    let error = db
        .query_v2(access(), request)
        .err()
        .expect("oversized plan must fail");
    assert_eq!(error.code, "QUERY_BUDGET_EXCEEDED");
    assert_eq!(db.stable_frontier(), before);
}

#[test]
fn parser_reservation_obeys_reduced_query_budget_before_publication() {
    let dir = TempDir::new().unwrap();
    let db = open(dir.path());
    let before = db.stable_frontier();
    let mut request = values_request();
    request.ir = None;
    request.hql = Some("VALUES $xs AS x |> RETURN x".into());
    request.language_version = Some(uee_v2::HqlLanguageVersionV2::HqlV2);
    request.budget = Some(serde_json::from_value(json!({"max_memory_bytes":1048576})).unwrap());
    assert_eq!(
        db.query_v2(access(), request).unwrap_err().code,
        "QUERY_BUDGET_EXCEEDED"
    );
    assert_eq!(db.stable_frontier(), before);
}

#[test]
fn read_only_missing_generation_is_unavailable_not_corrupt() {
    let dir = TempDir::new().unwrap();
    drop(open(dir.path()));
    let db = Storage::open(OpenOptions {
        path: dir.path().to_string_lossy().into(),
        page_cache_mb: Some(16),
        read_only: Some(true),
        vector_dim: Some(2),
        retention: Some("full".into()),
    })
    .unwrap();
    let before = db.stable_frontier();
    let mut request = values_request();
    request.explain = Some(uee_v2::ExplainV2::Plan);
    assert!(matches!(
        db.query_v2(access(), request).unwrap(),
        QueryOutcomeV2::Plan(_)
    ));
    assert_eq!(
        db.query_v2(access(), values_request()).unwrap_err().code,
        "CAPABILITY_UNSUPPORTED"
    );
    assert_eq!(db.stable_frontier(), before);
}
