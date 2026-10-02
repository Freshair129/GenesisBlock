//! Parser-to-logical lowering safety, without Storage or shared Cargo builds.
#![allow(dead_code)]
use std::collections::BTreeMap;
#[path = "../src/query/hql2/ast.rs"]
mod ast;
#[path = "../src/query/hql2/error.rs"]
mod error;
#[path = "../src/query/hql2/lower.rs"]
mod lower;
#[path = "../src/query/hql2/syntax.rs"]
mod syntax;
#[path = "../src/uee_v2.rs"]
mod uee_v2;
#[path = "../src/query/hql2/wire.rs"]
mod wire;

fn lowered(source: &str) -> Result<wire::LogicalRequestV2, error::QueryErrorV2> {
    lower::lower_hql2(syntax::parse_hql2(source)?)
}

fn lowered_with_unsigned(
    source: &str,
    parameters: &BTreeMap<String, u64>,
) -> Result<wire::LogicalRequestV2, error::QueryErrorV2> {
    lower::lower_hql2_with_unsigned_resolver(syntax::parse_hql2(source)?, |name| {
        parameters
            .get(name)
            .copied()
            .ok_or_else(|| error::QueryErrorV2::new("BIND_ERROR", "bind", "undeclared_parameter"))
    })
}

#[test]
fn nested_expression_lowers_on_one_mib_caller_stack() {
    std::thread::Builder::new().stack_size(1024*1024).spawn(|| {
        let source=format!("VALUES $xs AS x |> RETURN {}true AS result", "NOT ".repeat(99));
        let parsed=syntax::parse_hql2(&source).unwrap();
        eprintln!("parser returned; starting lower");
        let logical=lower::lower_hql2(parsed).unwrap();
        assert_eq!(logical.nodes.len(),2);
        let wire::Config::Project{fields}=&logical.nodes[1].config else{panic!("project")};
        let mut expression=&fields[0].expression;let mut count=0;
        while let wire::Expr::Unary{unary:wire::UnaryOp::Not,arg}=expression {count+=1;expression=arg;}
        assert_eq!(count,99);
        assert!(matches!(expression,wire::Expr::Literal{literal,ty} if literal==&serde_json::json!(true)&&ty=="Bool"));
    }).unwrap().join().unwrap();
}

#[test]
fn scalar_pipeline_keeps_shapes_scope_and_parameter_identity() {
    let logical=lowered("VALUES $xs AS x |> FILTER x > 0 |> PROJECT x + $delta AS y |> DISTINCT |> ORDER BY y DESC NULLS LAST |> TAKE 2 |> SKIP 1 |> RETURN *").unwrap();
    let ops: Vec<_> = logical.nodes.iter().map(|n| n.op.clone()).collect();
    use uee_v2::QueryOpV2::*;
    assert_eq!(
        ops,
        vec![Values, Filter, Project, Distinct, Sort, Take, Offset, Project]
    );
    assert!(logical.from_hql);
    assert!(logical.parameter_types.is_empty());
    assert_eq!(logical.root, "h7");
    for (i, n) in logical.nodes.iter().enumerate().skip(1) {
        assert_eq!(n.inputs, vec![format!("h{}", i - 1)]);
    }
    let wire::Config::Project { fields } = &logical.nodes[2].config else {
        panic!()
    };
    assert_eq!(fields[0].alias, "y");
    assert!(
        matches!(&fields[0].expression,wire::Expr::Binary{binary:wire::BinaryOp::Add,right,..} if matches!(right.as_ref(),wire::Expr::Param{param} if param=="delta"))
    );
    let wire::Config::Project { fields } = &logical.nodes[7].config else {
        panic!()
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].alias, "y");
}

#[test]
fn checked_remainder_lowers_to_typed_ir_rem() {
    let logical = lowered("VALUES $xs AS x |> RETURN x % 4 AS remainder").unwrap();
    let wire::Config::Project { fields } = &logical.nodes[1].config else {
        panic!("project config")
    };
    assert!(matches!(
        &fields[0].expression,
        wire::Expr::Binary {
            binary: wire::BinaryOp::Rem,
            ..
        }
    ));
}

#[test]
fn optional_annotation_stage_lowers_to_typed_lookup_operator() {
    let logical = lowered(
        "USE kb FROM NODES Document AS d |> OPTIONAL ANNOTATIONS OF d AS a |> RETURN d.id AS document_id, a.id AS annotation_id",
    )
    .unwrap();
    use uee_v2::QueryOpV2::*;
    assert_eq!(
        logical
            .nodes
            .iter()
            .map(|node| node.op.clone())
            .collect::<Vec<_>>(),
        vec![NodeScan, AnnotationLookup, Project]
    );
    let wire::Config::AnnotationLookup {
        target,
        alias,
        optional,
    } = &logical.nodes[1].config
    else {
        panic!("annotation lookup config")
    };
    assert_eq!(target, "d");
    assert_eq!(alias, "a");
    assert!(*optional);
    assert!(logical.from_hql);
}

#[test]
fn aggregate_star_in_and_between_have_closed_wire_shapes() {
    let logical=lowered("VALUES $xs AS x |> FILTER x NOT IN [1,2] |> FILTER x BETWEEN 1 AND 3 |> AGG count(*) AS n |> RETURN n").unwrap();
    assert!(
        matches!(&logical.nodes[1].config,wire::Config::Filter{predicate:wire::Expr::In{values,negated:true,..}} if values.len()==2)
    );
    assert!(matches!(
        &logical.nodes[2].config,
        wire::Config::Filter {
            predicate: wire::Expr::Binary {
                binary: wire::BinaryOp::And,
                ..
            }
        }
    ));
    let wire::Config::Aggregate { aggregates, .. } = &logical.nodes[3].config else {
        panic!()
    };
    assert!(
        matches!(&aggregates[0].expression,wire::Expr::Call{call,args} if call=="count_all"&&args.is_empty())
    );
}

#[test]
fn unimplemented_syntax_keeps_explicit_refusal() {
    let logical = lowered("VALUES $xs AS x |> RETURN null AS n").unwrap();
    let wire::Config::Project { fields } = &logical.nodes[1].config else {
        panic!("null projection config")
    };
    assert!(matches!(
        &fields[0].expression,
        wire::Expr::Literal { literal: serde_json::Value::Null, ty } if ty == "Null"
    ));

    for (query, reason) in [
        (
            "VALUES $xs AS x |> RETURN [1,2] AS v",
            "list_literal_context",
        ),
        ("SHOW CAPABILITIES", "statement_execution"),
    ] {
        let e = lowered(query).unwrap_err();
        assert_eq!(e.code, "CAPABILITY_UNSUPPORTED", "{query}");
        assert_eq!(e.detail.unwrap()["reason"], reason);
    }
}

#[test]
fn hql_order_without_nulls_clause_lowers_to_nulls_last_for_both_directions() {
    for direction in ["ASC", "DESC"] {
        let logical = lowered(&format!(
            "VALUES $xs AS x |> ORDER BY x {direction} |> RETURN x"
        ))
        .unwrap();
        let wire::Config::Sort { keys } = &logical.nodes[1].config else {
            panic!("sort config")
        };
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].nulls, wire::NullOrder::Last);
    }

    let logical = lowered("VALUES $xs AS x |> ORDER BY x ASC NULLS FIRST |> RETURN x").unwrap();
    let wire::Config::Sort { keys } = &logical.nodes[1].config else {
        panic!("sort config")
    };
    assert_eq!(keys[0].nulls, wire::NullOrder::First);
}

#[test]
fn structural_unsigned_parameters_lower_as_u64_and_checked_u32_ranks() {
    let parameters = BTreeMap::from([
        ("floor".into(), u64::MAX),
        ("skip".into(), u64::MAX),
        ("take".into(), u64::MAX),
        ("knn".into(), 3),
        ("rerank".into(), 1),
    ]);
    let logical = lowered_with_unsigned(
        "USE kb CHANGES SINCE $floor AS c |> SKIP $skip |> TAKE $take |> RETURN c",
        &parameters,
    )
    .unwrap();
    assert!(matches!(
        logical.nodes[0].config,
        wire::Config::ChangeScan {
            after_seq: u64::MAX,
            ..
        }
    ));
    assert!(matches!(
        logical.nodes[1].config,
        wire::Config::Offset { count: u64::MAX }
    ));
    assert!(matches!(
        logical.nodes[2].config,
        wire::Config::Take { count: u64::MAX }
    ));

    let logical = lowered_with_unsigned(
        "USE kb FROM NODES Document AS d |> KNN d IN docs USING $q TOP $knn EXACT AS hit |> RERANK d IN docs USING $q TOP $rerank EXACT AS fine |> RETURN d.id AS id",
        &parameters,
    )
    .unwrap();
    assert!(matches!(
        logical.nodes[1].config,
        wire::Config::Knn { k: 3, .. }
    ));
    assert!(matches!(
        logical.nodes[2].config,
        wire::Config::Rerank { k: 1, .. }
    ));

    let overflow = BTreeMap::from([("knn".into(), u64::from(u32::MAX) + 1)]);
    let error = lowered_with_unsigned(
        "USE kb FROM NODES Document AS d |> KNN d IN docs USING $q TOP $knn EXACT AS hit |> RETURN d.id AS id",
        &overflow,
    )
    .unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "ranking_bound");

    let overflow = BTreeMap::from([("rerank".into(), u64::from(u32::MAX) + 1)]);
    let error = lowered_with_unsigned(
        "USE kb FROM NODES Document AS d |> RERANK d IN docs USING $q TOP $rerank EXACT AS fine |> RETURN d.id AS id",
        &overflow,
    )
    .unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "ranking_bound");

    let error = lowered("VALUES $xs AS x |> TAKE $missing |> RETURN x").unwrap_err();
    assert_eq!(error.code, "BIND_ERROR");
    assert_eq!(error.detail.unwrap()["reason"], "undeclared_parameter");
}

#[test]
fn expression_depth_boundary_128_on_one_mib_stack() {
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let source = format!(
                "VALUES $xs AS x |> RETURN {}true AS result",
                "NOT ".repeat(127)
            );
            assert!(lowered(&source).is_ok());
            let rejected = format!(
                "VALUES $xs AS x |> RETURN {}true AS result",
                "NOT ".repeat(128)
            );
            assert_eq!(lowered(&rejected).unwrap_err().code, "HQL_PARSE_ERROR");
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn nested_function_calls_and_groups_are_stack_safe() {
    std::thread::Builder::new().stack_size(1024*1024).spawn(|| {
        for prefix in ["lower(","("] {
            let source=format!("VALUES $xs AS x |> RETURN {}\"Text\"{} AS result",prefix.repeat(99),")".repeat(99));
            let logical=lowered(&source).unwrap();
            let wire::Config::Project{fields}=&logical.nodes[1].config else{panic!()};
            let mut expression=&fields[0].expression;let mut calls=0;
            while let wire::Expr::Call{call,args}=expression {assert_eq!(call,"lower");assert_eq!(args.len(),1);calls+=1;expression=&args[0];}
            assert_eq!(calls,if prefix=="lower(" {99}else{0});
            assert!(matches!(expression,wire::Expr::Literal{ty,literal} if ty=="Utf8"&&literal==&serde_json::json!("Text")));
        }
    }).unwrap().join().unwrap();
}

#[test]
fn lowering_keeps_noncommutative_operands_and_membership_order() {
    let logical=lowered("VALUES $xs AS x |> RETURN 8 - 3 / 2 AS n, x IN [3,1,2] AS member, x BETWEEN 4 AND 9 AS range, x IS NOT NULL AS present").unwrap();
    let wire::Config::Project { fields } = &logical.nodes[1].config else {
        panic!()
    };
    let literal = |n: &str| wire::Expr::Literal {
        literal: serde_json::json!(n),
        ty: "I64".into(),
    };
    assert_eq!(
        fields[0].expression,
        wire::Expr::Binary {
            binary: wire::BinaryOp::Sub,
            left: Box::new(literal("8")),
            right: Box::new(wire::Expr::Binary {
                binary: wire::BinaryOp::Div,
                left: Box::new(literal("3")),
                right: Box::new(literal("2"))
            })
        }
    );
    let wire::Expr::In { values, .. } = &fields[1].expression else {
        panic!()
    };
    assert_eq!(values, &vec![literal("3"), literal("1"), literal("2")]);
    let wire::Expr::Binary {
        binary: wire::BinaryOp::And,
        left,
        right,
    } = &fields[2].expression
    else {
        panic!()
    };
    assert!(
        matches!(left.as_ref(),wire::Expr::Binary{binary:wire::BinaryOp::Ge,right,..} if right.as_ref()==&literal("4"))
    );
    assert!(
        matches!(right.as_ref(),wire::Expr::Binary{binary:wire::BinaryOp::Le,right,..} if right.as_ref()==&literal("9"))
    );
    assert!(matches!(
        &fields[3].expression,
        wire::Expr::Unary {
            unary: wire::UnaryOp::IsNotNull,
            ..
        }
    ));
}

#[test]
fn list_literals_remain_contextual_outside_in_operators() {
    let error = lowered("VALUES $xs AS x |> RETURN null + [1] AS result").unwrap_err();
    assert_eq!(error.detail.unwrap()["reason"], "list_literal_context");
    let error = lowered("VALUES $xs AS x |> RETURN [1] + null AS result").unwrap_err();
    assert_eq!(error.detail.unwrap()["reason"], "list_literal_context");
}

fn nested_between_source(count: usize) -> String {
    let mut expression = "x".to_owned();
    for _ in 0..count {
        expression = format!("({expression} BETWEEN 0 AND 1)");
    }
    format!("VALUES $xs AS x |> RETURN {expression} AS result")
}

#[test]
fn between_expansion_is_rejected_before_construction() {
    // 13 levels produce 49,147 wire nodes despite a compact source AST.
    // Use this bounded RED case before testing adversarial depth25.
    let error = lowered(&nested_between_source(13))
        .err()
        .expect("expanded expression exceeds bound");
    assert!(matches!(
        error.code.as_str(),
        "BIND_ERROR" | "QUERY_BUDGET_EXCEEDED"
    ));
}

#[test]
fn compact_twenty_five_level_between_refuses_without_expansion() {
    std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let source = nested_between_source(25);
            let parsed = syntax::parse_hql2(&source).expect("bounded source is valid syntax");
            let error = lower::lower_hql2(parsed)
                .err()
                .expect("expanded expression must refuse");
            assert_eq!(error.code, "BIND_ERROR");
            assert_eq!(error.detail.unwrap()["reason"], "expanded_expression_nodes");
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn between_expansion_small_tree_is_preserved_and_next_bound_refuses() {
    let logical = lowered(&nested_between_source(10)).unwrap();
    let wire::Config::Project { fields } = &logical.nodes[1].config else {
        panic!()
    };
    let mut pending = vec![&fields[0].expression];
    let mut nodes = 0;
    while let Some(e) = pending.pop() {
        nodes += 1;
        if let wire::Expr::Binary { left, right, .. } = e {
            pending.push(left);
            pending.push(right);
        }
    }
    assert_eq!(nodes, 6139);
    let error = lowered(&nested_between_source(11))
        .err()
        .expect("12283 nodes exceeds cap");
    assert_eq!(error.detail.unwrap()["reason"], "expanded_expression_nodes");
}

#[test]
fn expanded_depth_is_checked_separately_from_ast_depth() {
    for (levels, reject) in [(63, false), (64, true)] {
        let mut statement = syntax::parse_hql2("VALUES $xs AS x |> RETURN x AS result").unwrap();
        let ast::SelectKind::Expression { expression, .. } =
            &mut statement.query.as_mut().unwrap().returning[0].value
        else {
            panic!()
        };
        let atom = expression.clone();
        for _ in 0..levels {
            let previous = std::mem::replace(expression, atom.clone());
            *expression = ast::Spanned {
                span: atom.span,
                value: ast::ExprKind::Between {
                    expression: Box::new(atom.clone()),
                    low: Box::new(previous),
                    high: Box::new(atom.clone()),
                },
            };
        }
        // AST depth <=65, expanded wire depth is 2*levels+1 (127 or 129).
        let result = lower::lower_hql2(statement);
        if reject {
            assert_eq!(
                result.err().expect("expanded depth bound").detail.unwrap()["reason"],
                "expanded_expression_depth"
            );
        } else {
            assert!(result.is_ok());
        }
    }
}
