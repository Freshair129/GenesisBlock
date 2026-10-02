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
fn unsigned(
    value: &a::Unsigned,
    resolve_parameter: &dyn Fn(&str) -> Result<u64, QueryErrorV2>,
) -> Result<u64, QueryErrorV2> {
    match &value.value {
        a::UnsignedKind::Literal(v) => Ok(*v),
        a::UnsignedKind::Parameter(name) => resolve_parameter(name),
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
                A::Binary { left, right, .. } => {
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
            A::Null => E::Literal {
                literal: serde_json::Value::Null,
                ty: "Null".into(),
            },
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
                        a::BinaryOp::Remainder => w::BinaryOp::Rem,
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
            A::Object(_) => E::Literal {
                literal: json_literal(value)?,
                ty: "Json".into(),
            },
            A::List(_) => return Err(unsupported("list_literal_context")),
        };
        results.push(expression);
    }
    if results.len() != 1 {
        return Err(invalid("expression_stack"));
    }
    results.pop().ok_or_else(|| invalid("expression_stack"))
}

const JSON_LITERAL_LIMIT_BYTES: usize = 1024 * 1024;

fn charge_json_literal_size(total: &mut usize, bytes: usize) -> Result<(), QueryErrorV2> {
    *total = total
        .checked_add(bytes)
        .ok_or_else(|| invalid("json_literal_size"))?;
    if *total > JSON_LITERAL_LIMIT_BYTES {
        return Err(invalid("json_literal_size"));
    }
    Ok(())
}

fn ensure_json_literal_minimum(total: usize, minimum: usize) -> Result<(), QueryErrorV2> {
    if total
        .checked_add(minimum)
        .is_none_or(|size| size > JSON_LITERAL_LIMIT_BYTES)
    {
        return Err(invalid("json_literal_size"));
    }
    Ok(())
}

fn charge_json_string_size(total: &mut usize, value: &str) -> Result<(), QueryErrorV2> {
    charge_json_literal_size(total, 2)?;
    for character in value.chars() {
        let bytes = match character {
            '"' | '\\' | '\u{0008}' | '\t' | '\n' | '\u{000c}' | '\r' => 2,
            value if value <= '\u{001f}' => 6,
            value => value.len_utf8(),
        };
        charge_json_literal_size(total, bytes)?;
    }
    Ok(())
}

fn json_literal_size(expression: &a::Expr) -> Result<usize, QueryErrorV2> {
    enum Task<'a> {
        Visit(&'a a::Expr, usize),
    }
    let mut tasks = vec![Task::Visit(expression, 1)];
    let mut literal_bytes = 0usize;
    while let Some(Task::Visit(expression, depth)) = tasks.pop() {
        if depth > 128 {
            return Err(invalid("json_literal_depth"));
        }
        match &expression.value {
            a::ExprKind::Null => charge_json_literal_size(&mut literal_bytes, 4)?,
            a::ExprKind::Bool(true) => charge_json_literal_size(&mut literal_bytes, 4)?,
            a::ExprKind::Bool(false) => charge_json_literal_size(&mut literal_bytes, 5)?,
            a::ExprKind::I64(value) => {
                charge_json_literal_size(&mut literal_bytes, value.to_string().len())?
            }
            a::ExprKind::F64(value) => {
                let number = serde_json::Number::from_f64(*value)
                    .ok_or_else(|| invalid("json_literal_number"))?;
                charge_json_literal_size(&mut literal_bytes, number.to_string().len())?;
            }
            a::ExprKind::Utf8(value) => charge_json_string_size(&mut literal_bytes, value)?,
            a::ExprKind::List(items) => {
                charge_json_literal_size(
                    &mut literal_bytes,
                    2usize
                        .checked_add(items.len().saturating_sub(1))
                        .ok_or_else(|| invalid("json_literal_size"))?,
                )?;
                ensure_json_literal_minimum(literal_bytes, items.len())?;
                tasks.extend(items.iter().rev().map(|item| Task::Visit(item, depth + 1)));
            }
            a::ExprKind::Object(fields) => {
                charge_json_literal_size(
                    &mut literal_bytes,
                    2usize
                        .checked_add(fields.len().saturating_sub(1))
                        .ok_or_else(|| invalid("json_literal_size"))?,
                )?;
                let mut unique = std::collections::BTreeSet::new();
                for field in fields {
                    if !unique.insert(field.key.value.as_str()) {
                        return Err(invalid("duplicate_json_key"));
                    }
                    charge_json_string_size(&mut literal_bytes, &field.key.value)?;
                    charge_json_literal_size(&mut literal_bytes, 1)?;
                }
                ensure_json_literal_minimum(literal_bytes, fields.len())?;
                tasks.extend(
                    fields
                        .iter()
                        .rev()
                        .map(|field| Task::Visit(&field.expression, depth + 1)),
                );
            }
            _ => return Err(unsupported("json_literal_constant")),
        }
    }
    Ok(literal_bytes)
}

fn json_literal(expression: &a::Expr) -> Result<serde_json::Value, QueryErrorV2> {
    json_literal_size(expression)?;
    enum Task<'a> {
        Visit(&'a a::Expr, usize),
        Array(usize),
        Object(Vec<String>),
    }
    let mut tasks = vec![Task::Visit(expression, 1)];
    let mut values = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Visit(expression, depth) => {
                if depth > 128 {
                    return Err(invalid("json_literal_depth"));
                }
                match &expression.value {
                    a::ExprKind::Null => values.push(serde_json::Value::Null),
                    a::ExprKind::Bool(value) => values.push(serde_json::Value::Bool(*value)),
                    a::ExprKind::I64(value) => {
                        values.push(serde_json::Value::Number((*value).into()))
                    }
                    a::ExprKind::F64(value) => values.push(
                        serde_json::Number::from_f64(*value)
                            .map(serde_json::Value::Number)
                            .ok_or_else(|| invalid("json_literal_number"))?,
                    ),
                    a::ExprKind::Utf8(value) => {
                        values.push(serde_json::Value::String(value.clone()))
                    }
                    a::ExprKind::List(items) => {
                        tasks.push(Task::Array(items.len()));
                        tasks.extend(items.iter().rev().map(|item| Task::Visit(item, depth + 1)));
                    }
                    a::ExprKind::Object(fields) => {
                        let keys = fields
                            .iter()
                            .map(|field| field.key.value.clone())
                            .collect::<Vec<_>>();
                        let mut unique = std::collections::BTreeSet::new();
                        if keys.iter().any(|key| !unique.insert(key.clone())) {
                            return Err(invalid("duplicate_json_key"));
                        }
                        tasks.push(Task::Object(keys));
                        tasks.extend(
                            fields
                                .iter()
                                .rev()
                                .map(|field| Task::Visit(&field.expression, depth + 1)),
                        );
                    }
                    _ => return Err(unsupported("json_literal_constant")),
                }
            }
            Task::Array(count) => {
                let start = values
                    .len()
                    .checked_sub(count)
                    .ok_or_else(|| invalid("json_literal_shape"))?;
                let items = values.split_off(start);
                values.push(serde_json::Value::Array(items));
            }
            Task::Object(keys) => {
                let start = values
                    .len()
                    .checked_sub(keys.len())
                    .ok_or_else(|| invalid("json_literal_shape"))?;
                let items = values.split_off(start);
                let mut object = serde_json::Map::new();
                for (key, value) in keys.into_iter().zip(items) {
                    object.insert(key, value);
                }
                values.push(serde_json::Value::Object(object));
            }
        }
    }
    if values.len() != 1 {
        return Err(invalid("json_literal_shape"));
    }
    values.pop().ok_or_else(|| invalid("json_literal_shape"))
}

fn json_expr(expression: &a::Expr) -> Result<w::Expr, QueryErrorV2> {
    Ok(w::Expr::Literal {
        literal: json_literal(expression)?,
        ty: "Json".into(),
    })
}

fn pattern_node(
    alias: &a::Name,
    id: Option<&a::Expr>,
    labels: &[a::Name],
    properties: &BTreeMap<String, a::Expr>,
) -> Result<w::PatternNode, QueryErrorV2> {
    Ok(w::PatternNode {
        alias: name(alias)?,
        id: id.map(expr).transpose()?,
        labels: labels.iter().map(name).collect::<Result<_, _>>()?,
        properties: properties
            .iter()
            .map(|(key, value)| Ok((key.clone(), json_expr(value)?)))
            .collect::<Result<_, QueryErrorV2>>()?,
    })
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

struct Lower<'a> {
    nodes: Vec<w::LogicalNodeV2>,
    resolve_unsigned_parameter: &'a dyn Fn(&str) -> Result<u64, QueryErrorV2>,
}
impl Lower<'_> {
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
            a::SourceKind::History { kind, id, alias } => {
                let alias = name(&alias)?;
                let kind = match kind.value.as_str() {
                    "node" => w::EntityKind::Node,
                    "edge" => w::EntityKind::Edge,
                    "row" => w::EntityKind::Row,
                    "vector" => w::EntityKind::Vector,
                    "annotation" => w::EntityKind::Annotation,
                    "artifact" => w::EntityKind::Artifact,
                    _ => return Err(invalid("history_kind")),
                };
                let root = self.add(
                    QueryOpV2::HistoryScan,
                    vec![],
                    w::Config::HistoryScan {
                        kind,
                        id: expr(&id)?,
                        alias: alias.clone(),
                    },
                )?;
                (root, vec![alias])
            }
            a::SourceKind::Changes { after_seq, alias } => {
                let alias = name(&alias)?;
                let root = self.add(
                    QueryOpV2::ChangeScan,
                    vec![],
                    w::Config::ChangeScan {
                        after_seq: unsigned(&after_seq, self.resolve_unsigned_parameter)?,
                        alias: alias.clone(),
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
            a::SourceKind::Match {
                start_alias,
                start_id,
                start_labels,
                start_properties,
                steps,
                mode,
                path_alias,
                shortest,
            } => {
                let wire_start = pattern_node(
                    &start_alias,
                    start_id.as_ref(),
                    &start_labels,
                    &start_properties,
                )?;
                let start_alias = name(&start_alias)?;
                let mut scope = vec![start_alias.clone()];
                let mut wire_steps = Vec::with_capacity(steps.len());
                for step in steps {
                    let end_alias = name(&step.end_alias)?;
                    if !scope.contains(&end_alias) {
                        scope.push(end_alias.clone());
                    }
                    let edge_alias = step.edge_alias.as_ref().map(name).transpose()?;
                    if let Some(alias) = &edge_alias {
                        scope.push(alias.clone());
                    }
                    wire_steps.push(w::PatternStep {
                        edge: w::PatternEdge {
                            alias: edge_alias,
                            relations: step.relations.iter().map(name).collect::<Result<_, _>>()?,
                            direction: match step.direction {
                                a::GraphDirection::Out => w::Direction::Out,
                                a::GraphDirection::In => w::Direction::In,
                                a::GraphDirection::Both => w::Direction::Both,
                            },
                            min_hops: step.min_hops,
                            max_hops: step.max_hops,
                            properties: step
                                .edge_properties
                                .iter()
                                .map(|(key, value)| Ok((key.clone(), json_expr(value)?)))
                                .collect::<Result<_, QueryErrorV2>>()?,
                        },
                        node: pattern_node(
                            &step.end_alias,
                            step.node_id.as_ref(),
                            &step.node_labels,
                            &step.node_properties,
                        )?,
                    });
                }
                let path_alias = path_alias.as_ref().map(name).transpose()?;
                if let Some(alias) = &path_alias {
                    scope.push(alias.clone());
                }
                let root = self.add(
                    QueryOpV2::Match,
                    vec![],
                    w::Config::Match {
                        pattern: w::Pattern::Sequence(w::SequencePattern {
                            form: w::SequenceForm::Sequence,
                            start: wire_start,
                            steps: wire_steps,
                            mode: match mode {
                                a::GraphPathMode::Trail => w::PathMode::Trail,
                                a::GraphPathMode::Simple => w::PathMode::Simple,
                                a::GraphPathMode::Walk => w::PathMode::Walk,
                            },
                            path_alias,
                        }),
                        anchors: BTreeMap::new(),
                        shortest,
                    },
                )?;
                (root, scope)
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
            if let a::StageKind::Join {
                table,
                alias,
                kind,
                condition,
            } = &stage.value
            {
                let alias = name(alias)?;
                let right = self.add(
                    QueryOpV2::RowScan,
                    vec![],
                    w::Config::RowScan {
                        table: name(table)?,
                        alias: alias.clone(),
                    },
                )?;
                let kind = match kind {
                    a::JoinKind::Inner => w::JoinKind::Inner,
                    a::JoinKind::Left => w::JoinKind::Left,
                    a::JoinKind::Semi => w::JoinKind::Semi,
                    a::JoinKind::Anti => w::JoinKind::Anti,
                };
                if !matches!(kind, w::JoinKind::Semi | w::JoinKind::Anti) {
                    scope.push(alias);
                }
                root = self.add(
                    QueryOpV2::Join,
                    vec![root, right],
                    w::Config::Join {
                        kind,
                        condition: expr(condition)?,
                    },
                )?;
                continue;
            }
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
                a::StageKind::Join { .. } => unreachable!("join lowering handled above"),
                a::StageKind::AnnotationLookup {
                    target,
                    alias,
                    optional,
                } => {
                    let alias = name(&alias)?;
                    scope.push(alias.clone());
                    (
                        QueryOpV2::AnnotationLookup,
                        w::Config::AnnotationLookup {
                            target: name(&target)?,
                            alias,
                            optional,
                        },
                    )
                }
                a::StageKind::Knn {
                    entity,
                    collection,
                    query,
                    k,
                    mode,
                    alias,
                } => {
                    let alias = name(&alias)?;
                    scope.push(alias.clone());
                    (
                        QueryOpV2::Knn,
                        w::Config::Knn {
                            entity: name(&entity)?,
                            collection: name(&collection)?,
                            query: expr(&query)?,
                            k: u32::try_from(unsigned(&k, self.resolve_unsigned_parameter)?)
                                .map_err(|_| invalid("ranking_bound"))?,
                            mode: match mode {
                                a::VectorSearchMode::Exact => w::KnnMode::Exact,
                                a::VectorSearchMode::Approx => w::KnnMode::Approx,
                            },
                            alias,
                        },
                    )
                }
                a::StageKind::Rerank {
                    entity,
                    collection,
                    query,
                    k,
                    alias,
                } => {
                    let alias = name(&alias)?;
                    scope.push(alias.clone());
                    (
                        QueryOpV2::Rerank,
                        w::Config::Rerank {
                            entity: name(&entity)?,
                            collection: name(&collection)?,
                            query: expr(&query)?,
                            k: u32::try_from(unsigned(&k, self.resolve_unsigned_parameter)?)
                                .map_err(|_| invalid("ranking_bound"))?,
                            fidelity: w::Fidelity::Original,
                            alias,
                        },
                    )
                }
                a::StageKind::LexicalMatch {
                    entity,
                    field,
                    query,
                    index,
                    k,
                    alias,
                } => {
                    let alias = name(&alias)?;
                    scope.push(alias.clone());
                    (
                        QueryOpV2::LexicalMatch,
                        w::Config::LexicalMatch {
                            entity: name(&entity)?,
                            field: name(&field)?,
                            index: index.value,
                            query: expr(&query)?,
                            k: u32::try_from(unsigned(&k, self.resolve_unsigned_parameter)?)
                                .map_err(|_| invalid("ranking_bound"))?,
                            alias,
                        },
                    )
                }
                a::StageKind::ContextPack {
                    text,
                    evidence,
                    tokens,
                    tokenizer,
                    alias,
                } => {
                    let alias = name(&alias)?;
                    scope = vec![alias.clone()];
                    (
                        QueryOpV2::ContextPack,
                        w::Config::ContextPack {
                            text: expr(&text)?,
                            evidence: expr(&evidence)?,
                            tokens: u32::try_from(unsigned(
                                &tokens,
                                self.resolve_unsigned_parameter,
                            )?)
                            .map_err(|_| invalid("context_token_bound"))?,
                            tokenizer: tokenizer.value,
                            alias,
                        },
                    )
                }
                a::StageKind::Expand {
                    start_alias,
                    end_alias,
                    edge_alias,
                    relations,
                    direction,
                    min_hops,
                    max_hops,
                    mode,
                    path_alias,
                    optional,
                } => {
                    let end_alias = name(&end_alias)?;
                    if !scope.contains(&end_alias) {
                        scope.push(end_alias.clone());
                    }
                    let edge_alias = edge_alias.as_ref().map(name).transpose()?;
                    if let Some(alias) = &edge_alias {
                        scope.push(alias.clone());
                    }
                    let path_alias = path_alias.as_ref().map(name).transpose()?;
                    if let Some(alias) = &path_alias {
                        scope.push(alias.clone());
                    }
                    (
                        QueryOpV2::Expand,
                        w::Config::Expand {
                            pattern: w::Pattern::Compact(w::CompactPattern {
                                start_alias: name(&start_alias)?,
                                end_alias,
                                edge_alias,
                                relations: relations.iter().map(name).collect::<Result<_, _>>()?,
                                direction: match direction {
                                    a::GraphDirection::Out => w::Direction::Out,
                                    a::GraphDirection::In => w::Direction::In,
                                    a::GraphDirection::Both => w::Direction::Both,
                                },
                                min_hops,
                                max_hops,
                                mode: match mode {
                                    a::GraphPathMode::Trail => w::PathMode::Trail,
                                    a::GraphPathMode::Simple => w::PathMode::Simple,
                                    a::GraphPathMode::Walk => w::PathMode::Walk,
                                },
                                path_alias,
                            }),
                            optional,
                        },
                    )
                }
                a::StageKind::ExpandSequence {
                    start_alias,
                    start_id,
                    start_labels,
                    start_properties,
                    steps,
                    mode,
                    path_alias,
                    optional,
                } => {
                    let wire_start = pattern_node(
                        &start_alias,
                        start_id.as_ref(),
                        &start_labels,
                        &start_properties,
                    )?;
                    let mut wire_steps = Vec::with_capacity(steps.len());
                    for step in steps {
                        let end_alias = name(&step.end_alias)?;
                        if !scope.contains(&end_alias) {
                            scope.push(end_alias.clone());
                        }
                        let edge_alias = step.edge_alias.as_ref().map(name).transpose()?;
                        if let Some(alias) = &edge_alias {
                            scope.push(alias.clone());
                        }
                        wire_steps.push(w::PatternStep {
                            edge: w::PatternEdge {
                                alias: edge_alias,
                                relations: step
                                    .relations
                                    .iter()
                                    .map(name)
                                    .collect::<Result<_, _>>()?,
                                direction: match step.direction {
                                    a::GraphDirection::Out => w::Direction::Out,
                                    a::GraphDirection::In => w::Direction::In,
                                    a::GraphDirection::Both => w::Direction::Both,
                                },
                                min_hops: step.min_hops,
                                max_hops: step.max_hops,
                                properties: step
                                    .edge_properties
                                    .iter()
                                    .map(|(key, value)| Ok((key.clone(), json_expr(value)?)))
                                    .collect::<Result<_, QueryErrorV2>>()?,
                            },
                            node: pattern_node(
                                &step.end_alias,
                                step.node_id.as_ref(),
                                &step.node_labels,
                                &step.node_properties,
                            )?,
                        });
                    }
                    let path_alias = path_alias.as_ref().map(name).transpose()?;
                    if let Some(alias) = &path_alias {
                        scope.push(alias.clone());
                    }
                    (
                        QueryOpV2::Expand,
                        w::Config::Expand {
                            pattern: w::Pattern::Sequence(w::SequencePattern {
                                form: w::SequenceForm::Sequence,
                                start: wire_start,
                                steps: wire_steps,
                                mode: match mode {
                                    a::GraphPathMode::Trail => w::PathMode::Trail,
                                    a::GraphPathMode::Simple => w::PathMode::Simple,
                                    a::GraphPathMode::Walk => w::PathMode::Walk,
                                },
                                path_alias,
                            }),
                            optional,
                        },
                    )
                }
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
                                    None => w::NullOrder::Last,
                                },
                            })
                        })
                        .collect::<Result<Vec<_>, QueryErrorV2>>()?;
                    (QueryOpV2::Sort, w::Config::Sort { keys })
                }
                a::StageKind::Take(count) => (
                    QueryOpV2::Take,
                    w::Config::Take {
                        count: unsigned(&count, self.resolve_unsigned_parameter)?,
                    },
                ),
                a::StageKind::Skip(count) => (
                    QueryOpV2::Offset,
                    w::Config::Offset {
                        count: unsigned(&count, self.resolve_unsigned_parameter)?,
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
    lower_hql2_with_unsigned_resolver(statement, |_| Err(invalid("undeclared_parameter")))
}

pub(crate) fn lower_hql2_with_unsigned_resolver(
    statement: a::Hql2Statement,
    resolve_unsigned_parameter: impl Fn(&str) -> Result<u64, QueryErrorV2>,
) -> Result<w::LogicalRequestV2, QueryErrorV2> {
    if !matches!(
        statement.kind,
        a::StatementKind::Read | a::StatementKind::Explain | a::StatementKind::AnalyzeRead
    ) {
        return Err(unsupported("statement_execution"));
    }
    let mut lower = Lower {
        nodes: vec![],
        resolve_unsigned_parameter: &resolve_unsigned_parameter,
    };
    let (root, _) = lower.query(statement.query.ok_or_else(|| invalid("read_query"))?, 1)?;
    Ok(w::LogicalRequestV2 {
        nodes: lower.nodes,
        root,
        parameter_types: BTreeMap::new(),
        from_hql: true,
    })
}

#[cfg(test)]
mod json_literal_size_tests {
    use super::{a, json_literal, json_literal_size};

    fn primitive_array(items: usize) -> a::Expr {
        let span = a::Span {
            start_byte: 0,
            end_byte: 0,
        };
        a::Expr {
            span,
            value: a::ExprKind::List(
                (0..items)
                    .map(|_| a::Expr {
                        span,
                        value: a::ExprKind::Null,
                    })
                    .collect(),
            ),
        }
    }

    #[test]
    fn primitive_json_array_cannot_exceed_the_literal_size_limit() {
        // 210,000 nulls serialize to 1,050,001 bytes including commas/brackets.
        let error = match json_literal(&primitive_array(210_000)) {
            Err(error) => error,
            Ok(_) => panic!("oversized primitive arrays must fail closed"),
        };
        assert_eq!(error.code, "BIND_ERROR");
        assert_eq!(error.detail.unwrap()["reason"], "json_literal_size");
    }

    #[test]
    fn primitive_json_array_below_the_literal_size_limit_is_preserved() {
        let value = json_literal(&primitive_array(200_000))
            .expect("a primitive array below one MiB must remain supported");
        assert_eq!(value.as_array().unwrap().len(), 200_000);
    }

    #[test]
    fn json_string_size_matches_escaped_serialization() {
        let value = "quote: \" slash: \\ control: \u{0001} newline: \n thai: ไทย";
        let expression = a::Expr {
            span: a::Span {
                start_byte: 0,
                end_byte: 0,
            },
            value: a::ExprKind::Utf8(value.into()),
        };
        let serialized = serde_json::to_vec(&serde_json::Value::String(value.into())).unwrap();
        assert_eq!(json_literal_size(&expression).unwrap(), serialized.len());
    }
}
