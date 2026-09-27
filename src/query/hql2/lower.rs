//! Lower grammar-checked HQL into the same closed logical DAG used by IR.
use super::{ast as a, error::QueryErrorV2, wire as w};
use crate::uee_v2::QueryOpV2;
use std::collections::BTreeMap;

fn unsupported(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("CAPABILITY_UNSUPPORTED", "bind", reason)
}
fn invalid(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("BIND_ERROR", "bind", reason)
}
fn name(value: &a::Name) -> Result<String, QueryErrorV2> {
    let mut chars = value.value.bytes();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err(unsupported("quoted_symbol_execution"));
    }
    Ok(value.value.clone())
}
fn unsigned(value: &a::Unsigned) -> Result<u64, QueryErrorV2> {
    match value.value {
        a::UnsignedKind::Literal(v) => Ok(v),
        _ => Err(unsupported("parameterized_limit")),
    }
}
/// Bound the expanded wire tree, not just the original syntax tree. This pass
/// borrows the AST and must finish before constructing any lowered expression.
fn preflight_expr(value: &a::Expr) -> Result<(), QueryErrorV2> {
    use a::ExprKind as A;
    fn pop(shapes: &mut Vec<(usize, usize)>) -> Result<(usize, usize), QueryErrorV2> {
        shapes.pop().ok_or_else(|| invalid("expression_stack"))
    }
    fn add(a: usize, b: usize) -> Result<usize, QueryErrorV2> {
        a.checked_add(b)
            .filter(|v| *v <= 10_000)
            .ok_or_else(|| invalid("expanded_expression_nodes"))
    }
    let mut pending = vec![(value, 1usize, false)];
    let mut shapes = Vec::new();
    while let Some((value, depth, visited)) = pending.pop() {
        if depth > 128 {
            return Err(invalid("expression_depth"));
        }
        if !visited {
            pending.push((value, depth, true));
            match &value.value {
                A::Group(e) | A::Not(e) | A::IsNull { expression: e, .. } => {
                    pending.push((e, depth + 1, false))
                }
                A::Binary { left, right, .. } => {
                    pending.push((right, depth + 1, false));
                    pending.push((left, depth + 1, false));
                }
                A::Call { arguments, .. } => {
                    pending.extend(arguments.iter().rev().map(|e| (e, depth + 1, false)))
                }
                A::In {
                    expression, values, ..
                } => {
                    let A::List(values) = &values.value else {
                        return Err(invalid("in_values"));
                    };
                    pending.extend(values.iter().rev().map(|e| (e, depth + 1, false)));
                    pending.push((expression, depth + 1, false));
                }
                A::Between {
                    expression,
                    low,
                    high,
                } => {
                    pending.push((high, depth + 1, false));
                    pending.push((low, depth + 1, false));
                    pending.push((expression, depth + 1, false));
                }
                _ => {}
            }
            continue;
        }
        let (nodes, depth) = match &value.value {
            A::Group(_) => pop(&mut shapes)?,
            A::Not(_) | A::IsNull { .. } => {
                let (n, d) = pop(&mut shapes)?;
                (add(n, 1)?, d + 1)
            }
            A::Binary { .. } => {
                let (r, rd) = pop(&mut shapes)?;
                let (l, ld) = pop(&mut shapes)?;
                (add(add(l, r)?, 1)?, ld.max(rd) + 1)
            }
            A::Between { .. } => {
                let (h, hd) = pop(&mut shapes)?;
                let (l, ld) = pop(&mut shapes)?;
                let (e, ed) = pop(&mut shapes)?;
                let twice = e
                    .checked_mul(2)
                    .ok_or_else(|| invalid("expanded_expression_nodes"))?;
                (add(add(add(twice, l)?, h)?, 3)?, ed.max(ld).max(hd) + 2)
            }
            A::Call { arguments, .. } => {
                let (mut n, mut d) = (1usize, 1usize);
                for _ in arguments {
                    let (c, cd) = pop(&mut shapes)?;
                    n = add(n, c)?;
                    d = d.max(cd + 1);
                }
                (n, d)
            }
            A::In { values, .. } => {
                let A::List(values) = &values.value else {
                    return Err(invalid("in_values"));
                };
                let (mut n, mut d) = (1usize, 1usize);
                for _ in 0..=values.len() {
                    let (c, cd) = pop(&mut shapes)?;
                    n = add(n, c)?;
                    d = d.max(cd + 1);
                }
                (n, d)
            }
            _ => (1, 1),
        };
        if depth > 128 {
            return Err(invalid("expanded_expression_depth"));
        }
        shapes.push((nodes, depth));
    }
    if shapes.len() != 1 {
        return Err(invalid("expression_stack"));
    }
    Ok(())
}

fn expr(value: &a::Expr) -> Result<w::Expr, QueryErrorV2> {
    use a::ExprKind as A;
    use w::Expr as E;
    preflight_expr(value)?;
    let mut pending = vec![(value, 1usize, false)];
    let mut results = Vec::new();
    while let Some((value, depth, visited)) = pending.pop() {
        if depth > 128 {
            return Err(invalid("expression_depth"));
        }
        if !visited {
            // Push children in reverse order so errors retain left-to-right
            // source evaluation order. Never recurse on the caller stack.
            pending.push((value, depth, true));
            match &value.value {
                A::Group(inner) | A::Not(inner) => pending.push((inner, depth + 1, false)),
                A::IsNull { expression, .. } => pending.push((expression, depth + 1, false)),
                A::Binary { op, left, right } => {
                    if matches!(op, a::BinaryOp::Remainder) {
                        return Err(unsupported("remainder_function"));
                    }
                    pending.push((right, depth + 1, false));
                    pending.push((left, depth + 1, false));
                }
                A::Call {
                    name: call,
                    arguments,
                    star,
                } => {
                    let call = name(call)?.to_ascii_lowercase();
                    if star.is_some() && call != "count" {
                        return Err(invalid("function_star"));
                    }
                    pending.extend(arguments.iter().rev().map(|e| (e, depth + 1, false)));
                }
                A::In {
                    expression, values, ..
                } => {
                    let A::List(values) = &values.value else {
                        return Err(invalid("in_values"));
                    };
                    pending.extend(values.iter().rev().map(|e| (e, depth + 1, false)));
                    pending.push((expression, depth + 1, false));
                }
                A::Between {
                    expression,
                    low,
                    high,
                } => {
                    // Lower both occurrences independently, avoiding recursive
                    // cloning of a potentially deep lowered expression tree.
                    pending.push((high, depth + 1, false));
                    pending.push((expression, depth + 1, false));
                    pending.push((low, depth + 1, false));
                    pending.push((expression, depth + 1, false));
                }
                _ => {}
            }
            continue;
        }
        let mut pop = || results.pop().ok_or_else(|| invalid("expression_stack"));
        let expression = match &value.value {
            A::Bool(v) => E::Literal {
                literal: serde_json::json!(v),
                ty: "Bool".into(),
            },
            A::I64(v) => E::Literal {
                literal: serde_json::json!(v.to_string()),
                ty: "I64".into(),
            },
            A::F64(v) => E::Literal {
                literal: serde_json::json!(v),
                ty: "F64Finite".into(),
            },
            A::Utf8(v) => E::Literal {
                literal: serde_json::json!(v),
                ty: "Utf8".into(),
            },
            A::Null => return Err(unsupported("untyped_null_literal")),
            A::Parameter(param) => E::Param {
                param: param.clone(),
            },
            A::Field(parts) => {
                let first = parts.first().ok_or_else(|| invalid("empty_field"))?;
                E::Field {
                    field: w::FieldRef {
                        alias: name(first)?,
                        path: parts.iter().skip(1).map(name).collect::<Result<_, _>>()?,
                    },
                }
            }
            A::Group(_) => pop()?,
            A::Not(_) => E::Unary {
                unary: w::UnaryOp::Not,
                arg: Box::new(pop()?),
            },
            A::IsNull { negated, .. } => E::Unary {
                unary: if *negated {
                    w::UnaryOp::IsNotNull
                } else {
                    w::UnaryOp::IsNull
                },
                arg: Box::new(pop()?),
            },
            A::Binary { op, .. } => {
                let right = pop()?;
                let left = pop()?;
                E::Binary {
                    binary: match op {
                        a::BinaryOp::Or => w::BinaryOp::Or,
                        a::BinaryOp::And => w::BinaryOp::And,
                        a::BinaryOp::Equal => w::BinaryOp::Eq,
                        a::BinaryOp::NotEqual => w::BinaryOp::Ne,
                        a::BinaryOp::Less => w::BinaryOp::Lt,
                        a::BinaryOp::LessEqual => w::BinaryOp::Le,
                        a::BinaryOp::Greater => w::BinaryOp::Gt,
                        a::BinaryOp::GreaterEqual => w::BinaryOp::Ge,
                        a::BinaryOp::Contains => w::BinaryOp::Contains,
                        a::BinaryOp::StartsWith => w::BinaryOp::Startswith,
                        a::BinaryOp::Add => w::BinaryOp::Add,
                        a::BinaryOp::Subtract => w::BinaryOp::Sub,
                        a::BinaryOp::Multiply => w::BinaryOp::Mul,
                        a::BinaryOp::Divide => w::BinaryOp::Div,
                        a::BinaryOp::Remainder => return Err(unsupported("remainder_function")),
                    },
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            A::Call {
                name: call,
                arguments,
                star,
            } => {
                let call = name(call)?.to_ascii_lowercase();
                if star.is_some() && call != "count" {
                    return Err(invalid("function_star"));
                }
                let start = results
                    .len()
                    .checked_sub(arguments.len())
                    .ok_or_else(|| invalid("expression_stack"))?;
                E::Call {
                    call: if star.is_some() {
                        "count_all".into()
                    } else {
                        call
                    },
                    args: results.split_off(start),
                }
            }
            A::In {
                values, negated, ..
            } => {
                let A::List(values) = &values.value else {
                    return Err(invalid("in_values"));
                };
                let start = results
                    .len()
                    .checked_sub(values.len())
                    .ok_or_else(|| invalid("expression_stack"))?;
                let values = results.split_off(start);
                let expression = results.pop().ok_or_else(|| invalid("expression_stack"))?;
                E::In {
                    expression: Box::new(expression),
                    values,
                    negated: *negated,
                }
            }
            A::Between { .. } => {
                let high = pop()?;
                let right = pop()?;
                let low = pop()?;
                let left = pop()?;
                E::Binary {
                    binary: w::BinaryOp::And,
                    left: Box::new(E::Binary {
                        binary: w::BinaryOp::Ge,
                        left: Box::new(left),
                        right: Box::new(low),
                    }),
                    right: Box::new(E::Binary {
                        binary: w::BinaryOp::Le,
                        left: Box::new(right),
                        right: Box::new(high),
                    }),
                }
            }
            A::List(_) | A::Object(_) => return Err(unsupported("constructed_literal")),
        };
        results.push(expression);
    }
    if results.len() != 1 {
        return Err(invalid("expression_stack"));
    }
    results.pop().ok_or_else(|| invalid("expression_stack"))
}

fn selected(items: &[a::SelectItem], scope: &[String]) -> Result<Vec<w::NamedExpr>, QueryErrorV2> {
    let mut fields = Vec::new();
    for item in items {
        match &item.value {
            a::SelectKind::Star => fields.extend(scope.iter().map(|alias| w::NamedExpr {
                expression: w::Expr::Field {
                    field: w::FieldRef {
                        alias: alias.clone(),
                        path: vec![],
                    },
                },
                alias: alias.clone(),
            })),
            a::SelectKind::Expression { expression, alias } => {
                let alias = if let Some(alias) = alias {
                    name(alias)?
                } else if let a::ExprKind::Field(parts) = &expression.value {
                    name(parts.last().ok_or_else(|| invalid("field_alias"))?)?
                } else {
                    return Err(invalid("expression_alias_required"));
                };
                fields.push(w::NamedExpr {
                    expression: expr(expression)?,
                    alias,
                });
            }
        }
    }
    Ok(fields)
}

struct Lower {
    nodes: Vec<w::LogicalNodeV2>,
}
impl Lower {
    fn add(
        &mut self,
        op: QueryOpV2,
        inputs: Vec<String>,
        config: w::Config,
    ) -> Result<String, QueryErrorV2> {
        if self.nodes.len() >= 10_000 {
            return Err(invalid("node_count"));
        }
        let id = format!("h{}", self.nodes.len());
        self.nodes.push(w::LogicalNodeV2 {
            id: id.clone(),
            op,
            inputs,
            config,
        });
        Ok(id)
    }
    fn query(
        &mut self,
        query: a::Query,
        depth: usize,
    ) -> Result<(String, Vec<String>), QueryErrorV2> {
        if depth > 128 {
            return Err(invalid("query_depth"));
        }
        let (mut root, mut scope) = match query.source.value {
            a::SourceKind::Values { parameter, alias } => {
                let alias = name(&alias)?;
                let root = self.add(
                    QueryOpV2::Values,
                    vec![],
                    w::Config::Values {
                        param: name(&parameter)?,
                        alias: alias.clone(),
                    },
                )?;
                (root, vec![alias])
            }
            a::SourceKind::Nodes { label, alias } => {
                let alias = name(&alias)?;
                let root = self.add(
                    QueryOpV2::NodeScan,
                    vec![],
                    w::Config::NodeScan {
                        alias: alias.clone(),
                        label: label.map(|n| n.value),
                    },
                )?;
                (root, vec![alias])
            }
            a::SourceKind::Edges { relation, alias } => {
                let alias = name(&alias)?;
                let root = self.add(
                    QueryOpV2::EdgeScan,
                    vec![],
                    w::Config::EdgeScan {
                        alias: alias.clone(),
                        relation: relation.map(|n| n.value),
                    },
                )?;
                (root, vec![alias])
            }
            a::SourceKind::Rows { table, alias } => {
                let alias = name(&alias)?;
                let root = self.add(
                    QueryOpV2::RowScan,
                    vec![],
                    w::Config::RowScan {
                        table: name(&table)?,
                        alias: alias.clone(),
                    },
                )?;
                (root, vec![alias])
            }
            a::SourceKind::Annotations { alias } => {
                let alias = name(&alias)?;
                let root = self.add(
                    QueryOpV2::AnnotationScan,
                    vec![],
                    w::Config::AnnotationScan {
                        alias: alias.clone(),
                    },
                )?;
                (root, vec![alias])
            }
            a::SourceKind::UnionAll { left, right } => {
                let (left, scope) = self.query(*left, depth + 1)?;
                let (right, _) = self.query(*right, depth + 1)?;
                (
                    self.add(
                        QueryOpV2::UnionAll,
                        vec![left, right],
                        w::Config::UnionAll {},
                    )?,
                    scope,
                )
            }
            _ => return Err(unsupported("source_lowering")),
        };
        for stage in query.stages {
            let (op, config) = match stage.value {
                a::StageKind::Filter(predicate) => (
                    QueryOpV2::Filter,
                    w::Config::Filter {
                        predicate: expr(&predicate)?,
                    },
                ),
                a::StageKind::Project(items) => {
                    let fields = selected(&items, &scope)?;
                    scope = fields.iter().map(|f| f.alias.clone()).collect();
                    (QueryOpV2::Project, w::Config::Project { fields })
                }
                a::StageKind::Distinct => (QueryOpV2::Distinct, w::Config::Distinct {}),
                a::StageKind::Aggregate {
                    group_by,
                    aggregates,
                } => {
                    let group_by = selected(&group_by, &scope)?;
                    let aggregates = selected(&aggregates, &scope)?;
                    scope = group_by
                        .iter()
                        .chain(&aggregates)
                        .map(|f| f.alias.clone())
                        .collect();
                    (
                        QueryOpV2::Aggregate,
                        w::Config::Aggregate {
                            group_by,
                            aggregates,
                        },
                    )
                }
                a::StageKind::Order(items) => {
                    let keys = items
                        .iter()
                        .map(|item| {
                            Ok(w::SortKey {
                                expression: expr(&item.expression)?,
                                direction: match item.direction.as_ref().map(|s| s.value) {
                                    None | Some(a::SortDirection::Asc) => w::SortDirection::Asc,
                                    Some(a::SortDirection::Desc) => w::SortDirection::Desc,
                                },
                                nulls: match item.nulls.as_ref().map(|s| s.value) {
                                    Some(a::NullOrder::First) => w::NullOrder::First,
                                    Some(a::NullOrder::Last) => w::NullOrder::Last,
                                    None => return Err(unsupported("implicit_null_order")),
                                },
                            })
                        })
                        .collect::<Result<Vec<_>, QueryErrorV2>>()?;
                    (QueryOpV2::Sort, w::Config::Sort { keys })
                }
                a::StageKind::Take(count) => (
                    QueryOpV2::Take,
                    w::Config::Take {
                        count: unsigned(&count)?,
                    },
                ),
                a::StageKind::Skip(count) => (
                    QueryOpV2::Offset,
                    w::Config::Offset {
                        count: unsigned(&count)?,
                    },
                ),
                _ => return Err(unsupported("stage_lowering")),
            };
            root = self.add(op, vec![root], config)?;
        }
        let fields = selected(&query.returning, &scope)?;
        scope = fields.iter().map(|f| f.alias.clone()).collect();
        root = self.add(
            QueryOpV2::Project,
            vec![root],
            w::Config::Project { fields },
        )?;
        Ok((root, scope))
    }
}

pub(crate) fn lower_hql2(statement: a::Hql2Statement) -> Result<w::LogicalRequestV2, QueryErrorV2> {
    if !matches!(
        statement.kind,
        a::StatementKind::Read | a::StatementKind::Explain | a::StatementKind::AnalyzeRead
    ) {
        return Err(unsupported("statement_execution"));
    }
    let mut lower = Lower { nodes: vec![] };
    let (root, _) = lower.query(statement.query.ok_or_else(|| invalid("read_query"))?, 1)?;
    Ok(w::LogicalRequestV2 {
        nodes: lower.nodes,
        root,
        parameter_types: BTreeMap::new(),
        from_hql: true,
    })
}
