//! Test-only, deliberately unoptimized interpreter. Never import a production evaluator here.
#![allow(dead_code)] // Shared test library: each independent test binary exercises a subset.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[path = "hql2_graph_reference.rs"]
pub mod graph;
#[path = "hql2_rank_reference.rs"]
pub mod rank;

pub type Row = BTreeMap<String, Value>;
pub type Outcome<T> = Result<T, &'static str>;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    Text(String),
    Vector(Vec<f64>),
    List(Vec<Value>),
    Graph(graph::Binding),
    Context(Box<rank::ContextPackage>),
}
#[derive(Clone, Debug)]
pub enum Expr {
    Literal(Value),
    Field(String),
    Property(String),
    HasProperty(String),
    Eq(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Rem(Box<Expr>, Box<Expr>),
}
#[derive(Clone, Copy, Debug)]
pub enum JoinKind {
    Inner,
    Left,
    Semi,
    Anti,
}
#[derive(Clone, Copy, Debug)]
pub enum Metric {
    L2Squared,
    Cosine,
    NegDot,
}
#[derive(Clone, Debug)]
pub enum Aggregate {
    CountAll,
    Count(Expr),
    Sum(Expr),
    Avg(Expr),
    Min(Expr),
    Max(Expr),
    Collect(Expr, usize),
}
#[derive(Clone, Debug)]
pub struct SortKey {
    pub expr: Expr,
    pub descending: bool,
    pub nulls_first: bool,
}
#[derive(Clone, Debug)]
pub enum Plan {
    Values(Vec<Row>),
    Filter(Box<Plan>, Expr),
    Project(Box<Plan>, Vec<(String, Expr)>),
    Distinct(Box<Plan>),
    UnionAll(Box<Plan>, Box<Plan>),
    Take(Box<Plan>, usize),
    Offset(Box<Plan>, usize),
    Sort(Box<Plan>, Vec<SortKey>),
    Join {
        left: Box<Plan>,
        right: Box<Plan>,
        on: Expr,
        kind: JoinKind,
        right_fields: Vec<String>,
    },
    Aggregate {
        input: Box<Plan>,
        keys: Vec<(String, Expr)>,
        aggregates: Vec<(String, Aggregate)>,
    },
    Knn {
        input: Box<Plan>,
        vector: String,
        query: Vec<f64>,
        metric: Metric,
        k: usize,
        tie: Vec<String>,
        distance: String,
    },
}
fn boolean(value: &Value) -> Outcome<Option<bool>> {
    match value {
        Value::Bool(v) => Ok(Some(*v)),
        Value::Null => Ok(None),
        _ => Err("TYPE_MISMATCH"),
    }
}
fn truth(value: Option<bool>) -> Value {
    value.map(Value::Bool).unwrap_or(Value::Null)
}
pub fn compare(a: &Value, b: &Value) -> Outcome<Ordering> {
    match (a, b) {
        (Value::Null, Value::Null) => Ok(Ordering::Equal),
        (Value::Bool(a), Value::Bool(b)) => Ok(a.cmp(b)),
        (Value::I64(a), Value::I64(b)) => Ok(a.cmp(b)),
        (Value::F64(a), Value::F64(b)) if a.is_finite() && b.is_finite() => {
            a.partial_cmp(b).ok_or("NONFINITE_VALUE")
        }
        (Value::Text(a), Value::Text(b)) => Ok(a.cmp(b)),
        _ => Err("TYPE_MISMATCH"),
    }
}
pub fn eval(expr: &Expr, row: &Row) -> Outcome<Value> {
    match expr {
        Expr::Literal(value) => {
            validate_value(value, 0)?;
            Ok(value.clone())
        }
        Expr::Field(name) => row.get(name).cloned().ok_or("FIELD_UNKNOWN"),
        Expr::Property(name) => Ok(row.get(name).cloned().unwrap_or(Value::Null)),
        Expr::HasProperty(name) => Ok(Value::Bool(row.contains_key(name))),
        Expr::Not(a) => Ok(truth(boolean(&eval(a, row)?)?.map(|v| !v))),
        Expr::Eq(a, b) => {
            let (a, b) = (eval(a, row)?, eval(b, row)?);
            if a == Value::Null || b == Value::Null {
                Ok(Value::Null)
            } else {
                let equal = match (&a, &b) {
                    (Value::Context(a), Value::Context(b)) => a == b,
                    (
                        Value::Graph(graph::Binding::Entity(_)),
                        Value::Graph(graph::Binding::Entity(_)),
                    )
                    | (
                        Value::Graph(graph::Binding::Edges(_)),
                        Value::Graph(graph::Binding::Edges(_)),
                    )
                    | (
                        Value::Graph(graph::Binding::Path(_)),
                        Value::Graph(graph::Binding::Path(_)),
                    ) => a == b,
                    _ => compare(&a, &b)? == Ordering::Equal,
                };
                Ok(Value::Bool(equal))
            }
        }
        Expr::And(a, b) | Expr::Or(a, b) => {
            // Preserve left-to-right error behavior rather than silently dropping operands.
            let a = boolean(&eval(a, row)?)?;
            let b = boolean(&eval(b, row)?)?;
            let v = match expr {
                Expr::And(..) => match (a, b) {
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    (Some(true), Some(true)) => Some(true),
                    _ => None,
                },
                _ => match (a, b) {
                    (Some(true), _) | (_, Some(true)) => Some(true),
                    (Some(false), Some(false)) => Some(false),
                    _ => None,
                },
            };
            Ok(truth(v))
        }
        Expr::Add(a, b) | Expr::Div(a, b) | Expr::Rem(a, b) => {
            let (a, b) = (eval(a, row)?, eval(b, row)?);
            if a == Value::Null || b == Value::Null {
                return Ok(Value::Null);
            }
            match (a, b) {
                (Value::I64(a), Value::I64(b)) => {
                    let v = match expr {
                        Expr::Add(..) => a.checked_add(b),
                        Expr::Div(..) => {
                            if b == 0 {
                                return Err("DIVISION_BY_ZERO");
                            }
                            a.checked_div(b)
                        }
                        Expr::Rem(..) => {
                            if b == 0 {
                                return Err("DIVISION_BY_ZERO");
                            }
                            a.checked_rem(b)
                        }
                        _ => unreachable!(),
                    };
                    v.map(Value::I64).ok_or("INTEGER_OVERFLOW")
                }
                (Value::F64(a), Value::F64(b)) => {
                    if !a.is_finite() || !b.is_finite() {
                        return Err("NONFINITE_VALUE");
                    }
                    let v = match expr {
                        Expr::Add(..) => a + b,
                        Expr::Div(..) => {
                            if b == 0.0 {
                                return Err("DIVISION_BY_ZERO");
                            }
                            a / b
                        }
                        Expr::Rem(..) => {
                            if b == 0.0 {
                                return Err("DIVISION_BY_ZERO");
                            }
                            a % b
                        }
                        _ => unreachable!(),
                    };
                    if v.is_finite() {
                        Ok(Value::F64(v))
                    } else {
                        Err("NONFINITE_VALUE")
                    }
                }
                _ => Err("TYPE_MISMATCH"),
            }
        }
    }
}
fn unique_names<'a>(names: impl Iterator<Item = &'a String>) -> Outcome<()> {
    let mut seen = BTreeSet::new();
    for name in names {
        if name.is_empty() || !seen.insert(name) {
            return Err("DUPLICATE_OUTPUT");
        }
    }
    Ok(())
}
fn project(row: &Row, fields: &[(String, Expr)]) -> Outcome<Row> {
    fields
        .iter()
        .map(|(name, expr)| Ok((name.clone(), eval(expr, row)?)))
        .collect()
}
fn combined(left: &Row, right: &Row) -> Outcome<Row> {
    if right.keys().any(|key| left.contains_key(key)) {
        return Err("AMBIGUOUS_FIELD");
    }
    Ok(left
        .iter()
        .chain(right)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect())
}
fn aggregate(rows: &[Row], agg: &Aggregate) -> Outcome<Value> {
    if let Aggregate::Collect(expr, limit) = agg {
        if rows.len() > *limit {
            return Err("RESOURCE_LIMIT_EXCEEDED");
        }
        return Ok(Value::List(
            rows.iter()
                .map(|row| eval(expr, row))
                .collect::<Outcome<Vec<_>>>()?,
        ));
    }
    if matches!(agg, Aggregate::CountAll) {
        return Ok(Value::I64(
            i64::try_from(rows.len()).map_err(|_| "INTEGER_OVERFLOW")?,
        ));
    }
    let expr = match agg {
        Aggregate::Count(e)
        | Aggregate::Sum(e)
        | Aggregate::Avg(e)
        | Aggregate::Min(e)
        | Aggregate::Max(e) => e,
        _ => unreachable!(),
    };
    let values = rows
        .iter()
        .map(|r| eval(expr, r))
        .collect::<Outcome<Vec<_>>>()?
        .into_iter()
        .filter(|v| *v != Value::Null)
        .collect::<Vec<_>>();
    if matches!(agg, Aggregate::Count(_)) {
        return Ok(Value::I64(
            i64::try_from(values.len()).map_err(|_| "INTEGER_OVERFLOW")?,
        ));
    }
    if values.is_empty() {
        return Ok(Value::Null);
    }
    if matches!(agg, Aggregate::Min(_) | Aggregate::Max(_)) {
        for value in &values {
            compare(value, value)?;
        }
    }
    let mut result = values[0].clone();
    for value in &values[1..] {
        match agg {
            Aggregate::Min(_) => {
                if compare(value, &result)? == Ordering::Less {
                    result = value.clone();
                }
            }
            Aggregate::Max(_) => {
                if compare(value, &result)? == Ordering::Greater {
                    result = value.clone();
                }
            }
            _ => {
                result = eval(
                    &Expr::Add(
                        Box::new(Expr::Literal(result)),
                        Box::new(Expr::Literal(value.clone())),
                    ),
                    &Row::new(),
                )?
            }
        }
    }
    if matches!(agg, Aggregate::Sum(_) | Aggregate::Avg(_))
        && !matches!(result, Value::I64(_) | Value::F64(_))
    {
        return Err("TYPE_MISMATCH");
    }
    if matches!(agg, Aggregate::Avg(_)) {
        let n = match result {
            Value::I64(n) => n as f64,
            Value::F64(n) => n,
            _ => return Err("TYPE_MISMATCH"),
        };
        result = Value::F64(n / values.len() as f64);
    }
    Ok(result)
}
fn validate_query(query: &[f64], metric: Metric) -> Outcome<()> {
    if query.is_empty() {
        return Err("DIMENSION_MISMATCH");
    }
    if !query.iter().all(|v| v.is_finite()) {
        return Err("NONFINITE_VECTOR");
    }
    if matches!(metric, Metric::Cosine) && query.iter().all(|v| *v == 0.0) {
        return Err("ZERO_COSINE_NORM");
    }
    Ok(())
}

fn compensated_sum(values: impl Iterator<Item = f64>) -> f64 {
    // Neumaier compensation retains low terms when larger terms cancel.
    let mut sum = 0.0_f64;
    let mut correction = 0.0_f64;
    for value in values {
        let next = sum + value;
        correction += if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        sum = next;
    }
    sum + correction
}

pub fn distance(a: &[f64], b: &[f64], metric: Metric) -> Outcome<f64> {
    if a.is_empty() || a.len() != b.len() {
        return Err("DIMENSION_MISMATCH");
    }
    if !a.iter().chain(b).all(|v| v.is_finite()) {
        return Err("NONFINITE_VECTOR");
    }
    let result = match metric {
        Metric::L2Squared => a.iter().zip(b).map(|(a, b)| (a - b) * (a - b)).sum(),
        Metric::NegDot => -compensated_sum(a.iter().zip(b).map(|(a, b)| a * b)),
        Metric::Cosine => {
            // Scaling avoids squaring large finite inputs into infinity.
            let scale_a = a.iter().fold(0.0_f64, |n, v| n.max(v.abs()));
            let scale_b = b.iter().fold(0.0_f64, |n, v| n.max(v.abs()));
            if scale_a == 0.0 || scale_b == 0.0 {
                return Err("ZERO_COSINE_NORM");
            }
            let aa = a.iter().map(|v| v / scale_a).collect::<Vec<_>>();
            let bb = b.iter().map(|v| v / scale_b).collect::<Vec<_>>();
            let dot = aa.iter().zip(&bb).map(|(a, b)| a * b).sum::<f64>();
            let norm = aa.iter().map(|v| v * v).sum::<f64>().sqrt()
                * bb.iter().map(|v| v * v).sum::<f64>().sqrt();
            1.0 - (dot / norm).clamp(-1.0, 1.0)
        }
    };
    if result.is_finite() {
        Ok(result)
    } else {
        Err("NONFINITE_DISTANCE")
    }
}
fn order(a: &[Value], b: &[Value], keys: &[SortKey]) -> Outcome<Ordering> {
    for ((a, b), key) in a.iter().zip(b).zip(keys) {
        let mut cmp = match (a, b) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => {
                if key.nulls_first {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (_, Value::Null) => {
                if key.nulls_first {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            _ => compare(a, b)?,
        };
        if key.descending && a != &Value::Null && b != &Value::Null {
            cmp = cmp.reverse();
        }
        if cmp != Ordering::Equal {
            return Ok(cmp);
        }
    }
    Ok(Ordering::Equal)
}

fn sort_rows(rows: Vec<Row>, keys: &[SortKey]) -> Outcome<Vec<Row>> {
    let mut sorted: Vec<(Row, Vec<Value>)> = Vec::new();
    // Insertion sort is independent from production kernels and logical Plan depth.
    for row in rows {
        let values = keys
            .iter()
            .map(|key| eval(&key.expr, &row))
            .collect::<Outcome<Vec<_>>>()?;
        for value in &values {
            compare(value, value)?;
        }
        let mut position = sorted.len();
        for (i, (_, other)) in sorted.iter().enumerate() {
            if order(&values, other, keys)? == Ordering::Less {
                position = i;
                break;
            }
        }
        sorted.insert(position, (row, values));
    }
    Ok(sorted.into_iter().map(|(row, _)| row).collect())
}

pub fn execute(plan: &Plan) -> Outcome<Vec<Row>> {
    execute_depth(plan, 0)
}
fn validate_value(value: &Value, depth: usize) -> Outcome<()> {
    if depth > 128 {
        return Err("VALUE_DEPTH");
    }
    match value {
        // Tokenizer fingerprints are opaque registry keys. PACK owns recounting;
        // a package without that registry can enforce only the declared budget.
        Value::Context(package) if package.token_count > package.token_budget => {
            Err("INVALID_CONTEXT_PACKAGE")
        }
        Value::F64(v) if !v.is_finite() => Err("NONFINITE_VALUE"),
        Value::Vector(v) if !v.iter().all(|v| v.is_finite()) => Err("NONFINITE_VALUE"),
        Value::List(values) => {
            for v in values {
                validate_value(v, depth + 1)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
fn execute_depth(plan: &Plan, depth: usize) -> Outcome<Vec<Row>> {
    if depth > 128 {
        return Err("IR_DEPTH");
    }
    let run = |p: &Plan| execute_depth(p, depth + 1);
    match plan {
        Plan::Values(rows) => {
            for row in rows {
                for value in row.values() {
                    validate_value(value, 0)?;
                }
            }
            Ok(rows.clone())
        }
        Plan::Filter(input, predicate) => run(input)?
            .into_iter()
            .filter_map(
                |row| match eval(predicate, &row).and_then(|v| boolean(&v)) {
                    Ok(Some(true)) => Some(Ok(row)),
                    Ok(_) => None,
                    Err(e) => Some(Err(e)),
                },
            )
            .collect(),
        Plan::Project(input, fields) => {
            unique_names(fields.iter().map(|(name, _)| name))?;
            run(input)?.iter().map(|row| project(row, fields)).collect()
        }
        Plan::Distinct(input) => {
            let mut result = Vec::new();
            for row in run(input)? {
                if !result.contains(&row) {
                    result.push(row);
                }
            }
            Ok(result)
        }
        Plan::UnionAll(a, b) => {
            let mut result = run(a)?;
            result.extend(run(b)?);
            Ok(result)
        }
        Plan::Take(input, n) => Ok(run(input)?.into_iter().take(*n).collect()),
        Plan::Offset(input, n) => Ok(run(input)?.into_iter().skip(*n).collect()),
        Plan::Sort(input, keys) => sort_rows(run(input)?, keys),
        Plan::Join {
            left,
            right,
            on,
            kind,
            right_fields,
        } => {
            unique_names(right_fields.iter())?;
            let (left, right) = (run(left)?, run(right)?);
            let mut result = Vec::new();
            for left_row in left {
                let mut matches = Vec::new();
                for right_row in &right {
                    let joined = combined(&left_row, right_row)?;
                    if boolean(&eval(on, &joined)?)? == Some(true) {
                        matches.push(joined);
                    }
                }
                match kind {
                    JoinKind::Semi => {
                        if !matches.is_empty() {
                            result.push(left_row);
                        }
                    }
                    JoinKind::Anti => {
                        if matches.is_empty() {
                            result.push(left_row);
                        }
                    }
                    JoinKind::Left if matches.is_empty() => {
                        let nulls = right_fields
                            .iter()
                            .map(|k| (k.clone(), Value::Null))
                            .collect();
                        result.push(combined(&left_row, &nulls)?);
                    }
                    _ => result.extend(matches),
                }
            }
            Ok(result)
        }
        Plan::Aggregate {
            input,
            keys,
            aggregates,
        } => {
            unique_names(
                keys.iter()
                    .map(|(n, _)| n)
                    .chain(aggregates.iter().map(|(n, _)| n)),
            )?;
            let rows = run(input)?;
            let mut groups: Vec<(Row, Vec<Row>)> = Vec::new();
            if keys.is_empty() {
                groups.push((Row::new(), rows));
            } else {
                for row in rows {
                    let key = project(&row, keys)?;
                    if let Some((_, rows)) = groups.iter_mut().find(|(k, _)| *k == key) {
                        rows.push(row);
                    } else {
                        groups.push((key, vec![row]));
                    }
                }
            }
            groups
                .into_iter()
                .map(|(mut key, rows)| {
                    for (name, agg) in aggregates {
                        key.insert(name.clone(), aggregate(&rows, agg)?);
                    }
                    Ok(key)
                })
                .collect()
        }
        Plan::Knn {
            input,
            vector,
            query,
            metric,
            k,
            tie,
            distance: distance_name,
        } => {
            validate_query(query, *metric)?;
            let rows = run(input)?;
            let mut scored = Vec::new();
            for mut row in rows {
                let v = match row.get(vector) {
                    Some(Value::Vector(v)) => v,
                    Some(Value::Null) | None => continue,
                    _ => return Err("TYPE_MISMATCH"),
                };
                let d = distance(v, query, *metric)?;
                if row.contains_key(distance_name) {
                    return Err("DUPLICATE_OUTPUT");
                }
                row.insert(distance_name.clone(), Value::F64(d));
                scored.push(row);
            }
            let mut keys = vec![SortKey {
                expr: Expr::Field(distance_name.clone()),
                descending: false,
                nulls_first: false,
            }];
            keys.extend(tie.iter().map(|k| SortKey {
                expr: Expr::Field(k.clone()),
                descending: false,
                nulls_first: false,
            }));
            let sorted = sort_rows(scored, &keys)?;
            Ok(sorted.into_iter().take(*k).collect())
        }
    }
}
