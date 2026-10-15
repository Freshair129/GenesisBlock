#![allow(dead_code)]
#[path = "../src/query/hql2/ast.rs"]
mod ast;
#[path = "../src/query/hql2/error.rs"]
mod error;
#[path = "../src/query/hql2/request.rs"]
mod request;
#[path = "../src/query/hql2/syntax.rs"]
mod syntax;
#[path = "../src/uee_v2.rs"]
mod uee_v2;
#[path = "../src/query/hql2/wire.rs"]
mod wire;
use serde_json::json;
use uee_v2::*;

fn hql(source: &str) -> QueryRequestV2 {
    serde_json::from_value(json!({"contract_version":"genesis.api.v2","request_id":"t","namespace":"default","hql":source,"language_version":"hql.v2","params":{"xs":{"type":"List<I64>","value":[]}}})).unwrap()
}
fn norm(q: &QueryRequestV2) -> Result<request::ReadOptionsV2, error::QueryErrorV2> {
    let s = q.hql.as_deref().map(syntax::parse_hql2).transpose()?;
    request::normalize(q, s.as_ref(), 5, 1, chrono::Utc::now())
}
#[test]
fn plain_hql_allows_envelope_explain_but_explicit_text_must_agree() {
    for mode in [ExplainV2::Plan, ExplainV2::Analyze] {
        let mut q = hql("VALUES $xs AS x |> RETURN x");
        q.explain = Some(mode.clone());
        assert_eq!(norm(&q).unwrap().mode, mode);
    }
    let mut q = hql("EXPLAIN VALUES $xs AS x |> RETURN x");
    q.explain = Some(ExplainV2::None);
    assert_eq!(norm(&q).err().unwrap().code, "BIND_ERROR");
}
#[test]
fn version_frontier_and_policy_matrix_fail_closed() {
    let q = hql("VALUES $xs AS x |> RETURN x");
    for frontier in ["18446744073709551616", "01", "-1"] {
        let mut q = q.clone();
        q.required_frontier = Some(frontier.into());
        assert!(norm(&q).is_err());
    }
    let mut future = q.clone();
    future.required_frontier = Some("6".into());
    assert_eq!(norm(&future).err().unwrap().code, "INDEX_COVERAGE_TIMEOUT");
    let mut version = q.clone();
    version.contract_version = "genesis.api.v3".into();
    assert_eq!(norm(&version).err().unwrap().code, "VERSION_UNSUPPORTED");
    for field in ["format", "allow_partial", "index_policy"] {
        let mut raw = serde_json::to_value(&q).unwrap();
        raw[field] = match field {
            "format" => json!("stream"),
            "allow_partial" => json!(true),
            _ => json!("eventual"),
        };
        assert_eq!(
            norm(&serde_json::from_value(raw).unwrap())
                .err()
                .unwrap()
                .code,
            "CAPABILITY_UNSUPPORTED"
        );
    }
}
#[test]
fn temporal_namespace_conflicts_and_write_forms_reject() {
    assert_eq!(
        norm(&hql("USE other VALUES $xs AS x |> RETURN x"))
            .err()
            .unwrap()
            .code,
        "BIND_ERROR"
    );
    let mut q = hql("USE default AT VALID \"2026-01-01T00:00:00Z\" VALUES $xs AS x |> RETURN x");
    q.temporal = Some(QueryTemporalV2 {
        tx_as_of: None,
        valid_at: Some("2026-01-02T00:00:00Z".parse().unwrap()),
    });
    assert_eq!(norm(&q).err().unwrap().code, "BIND_ERROR");
    for source in [
        "CHECKPOINT",
        "RETRACT NODE \"a\"",
        "CREATE TABLE orders SCHEMA $schema",
    ] {
        assert_eq!(
            norm(&hql(source)).err().unwrap().code,
            "CAPABILITY_UNSUPPORTED"
        );
    }
}
#[test]
fn request_size_is_checked_before_typed_decoding() {
    let mut q = hql("VALUES $xs AS x |> RETURN x");
    q.budget = Some(serde_json::from_value(json!({"max_memory_bytes":8192})).unwrap());
    q.params.insert(
        "large".into(),
        json!({"type":"Utf8","value":"a".repeat(8192)}),
    );
    assert_eq!(
        request::input_bytes(&q).unwrap_err().code,
        "QUERY_BUDGET_EXCEEDED"
    );
}

#[test]
fn structural_id_copies_and_expression_types_are_reserved_before_validation() {
    let mut q = hql("VALUES $xs AS x |> RETURN x");
    q.hql = None;
    q.language_version = None;
    let id = "s".repeat(262_144);
    q.ir = Some(QueryIrV2 {
        contract_version: "query-ir.v2".into(),
        root: id.clone(),
        parameter_types: Default::default(),
        nodes: vec![QueryNodeV2 {
            id,
            op: QueryOpV2::Values,
            inputs: vec![],
            config: std::collections::BTreeMap::from([
                ("param".into(), json!("xs")),
                ("as".into(), json!("x")),
            ]),
        }],
    });
    q.budget = Some(serde_json::from_value(json!({"max_memory_bytes":524288})).unwrap());
    assert_eq!(
        request::input_bytes(&q).unwrap_err().code,
        "QUERY_BUDGET_EXCEEDED"
    );
    let ty = format!("{}I64{}", "List<".repeat(64), ">".repeat(64));
    let config = wire::Config::Filter {
        predicate: wire::Expr::In {
            expression: Box::new(wire::Expr::Param { param: "p".into() }),
            values: (0..4000)
                .map(|_| wire::Expr::Param { param: "p".into() })
                .collect(),
            negated: false,
        },
    };
    let logical = wire::LogicalRequestV2 {
        root: "n".into(),
        nodes: vec![wire::LogicalNodeV2 {
            id: "n".into(),
            op: QueryOpV2::Filter,
            inputs: vec!["s".into()],
            config,
        }],
        parameter_types: std::collections::BTreeMap::from([("p".into(), ty)]),
        from_hql: false,
    };
    assert!(request::planning_bytes(&logical, &q) > 3145728);
}

#[test]
fn deeply_nested_rust_request_is_refused_and_disposed_without_recursive_drop() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let mut q = hql("VALUES $xs AS x |> RETURN x");
            let mut nested = serde_json::Value::Null;
            for _ in 0..10_000 {
                nested = serde_json::Value::Array(vec![nested]);
            }
            q.params.insert("deep".into(), nested);
            assert_eq!(request::input_bytes(&q).unwrap_err().code, "BIND_ERROR");
            request::discard_request(q);
        })
        .unwrap()
        .join()
        .unwrap();
}
