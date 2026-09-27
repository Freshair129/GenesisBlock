//! Native, exact scalar kernels. No Storage access or JSON row protocol.
use super::{
    bind::{Aggregate, AggregateKind, BoundExpr, ExprKind, Kernel},
    error::QueryErrorV2,
    plan::PhysicalPlanV2,
    result::{ActualCountersV2, ExplainNodeV2},
    value::QueryValueV2 as V,
    wire::{BinaryOp, JoinKind, NullOrder, SortDirection, UnaryOp},
};
use crate::uee_v2::QueryBudgetV2;
use std::{
    cmp::Ordering,
    mem::size_of,
    time::{Duration, Instant},
};

fn failure(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("BIND_ERROR", "execute", reason)
}
fn quota() -> QueryErrorV2 {
    QueryErrorV2::new("QUERY_BUDGET_EXCEEDED", "execute", "query_limit")
}

pub(crate) struct ExecutionBudgetV2 {
    started: Instant,
    elapsed: Duration,
    memory: u64,
    accounted: u64,
    rows: u64,
    bytes: u64,
}
impl ExecutionBudgetV2 {
    pub(crate) fn new(options: &Option<QueryBudgetV2>) -> Result<Self, QueryErrorV2> {
        let started = Instant::now();
        fn limit(value: Option<u64>, default: u64) -> Result<u64, QueryErrorV2> {
            match value {
                None => Ok(default),
                Some(v) if v > 0 && v <= default => Ok(v),
                _ => Err(QueryErrorV2::new("BIND_ERROR", "bind", "budget_limit")),
            }
        }
        let mut budget = Self {
            started,
            elapsed: Duration::from_millis(5000),
            memory: 64 * 1024 * 1024,
            accounted: 0,
            rows: 10_000,
            bytes: 32 * 1024 * 1024,
        };
        if let Some(o) = options {
            if o.max_spill_bytes.unwrap_or(0) != 0 {
                return Err(QueryErrorV2::new(
                    "CAPABILITY_UNSUPPORTED",
                    "bind",
                    "spill_unavailable",
                ));
            }
            budget.elapsed = Duration::from_millis(limit(o.max_elapsed_ms, 5000)?);
            budget.memory = limit(o.max_memory_bytes, 64 * 1024 * 1024)?;
            budget.rows = limit(o.max_result_rows, 10_000)?;
            budget.bytes = limit(o.max_result_bytes, 32 * 1024 * 1024)?;
            limit(o.max_expanded_nodes, 100_000)?;
            limit(o.max_expanded_edges, 200_000)?;
            limit(o.max_vector_candidates, 100_000)?;
            limit(o.max_distance_evaluations, 1_000_000)?;
        }
        Ok(budget)
    }
    /// Include authorization/lock latency without allowing deadline extension.
    pub(crate) fn account_elapsed_since(&mut self, start: Instant) {
        self.started = self.started.min(start);
    }
    pub(crate) fn check(&self) -> Result<(), QueryErrorV2> {
        if self.started.elapsed() > self.elapsed {
            Err(quota())
        } else {
            Ok(())
        }
    }
    /// Conservative cumulative allocation charge; no spilling or partial success.
    pub(crate) fn reserve(&mut self, bytes: u64) -> Result<(), QueryErrorV2> {
        self.check()?;
        let next = self.accounted.checked_add(bytes).ok_or_else(quota)?;
        if next > self.memory {
            return Err(quota());
        }
        self.accounted = next;
        Ok(())
    }
    pub(crate) fn check_output(&self, rows: u64, bytes: u64) -> Result<(), QueryErrorV2> {
        self.check()?;
        if rows > self.rows || bytes > self.bytes {
            Err(quota())
        } else {
            Ok(())
        }
    }
}
#[derive(Debug)]
pub(crate) struct ExecutionOutputV2 {
    pub(crate) rows: Vec<Vec<V>>,
    pub(crate) nodes: Vec<ExplainNodeV2>,
}
type Row = Vec<V>;

fn bytes(v: &V) -> u64 {
    let heap = match v {
        V::Utf8(s) => s.len() as u64,
        V::List(v) => v.iter().map(bytes).fold(0u64, u64::saturating_add),
        _ => 0,
    };
    (size_of::<V>() as u64).saturating_add(heap)
}
fn row_bytes(row: &[V]) -> u64 {
    (size_of::<Row>() as u64).saturating_add(row.iter().map(bytes).fold(0, u64::saturating_add))
}
fn clone_value(v: &V, b: &mut ExecutionBudgetV2) -> Result<V, QueryErrorV2> {
    b.reserve(bytes(v))?;
    Ok(v.clone())
}
fn push_clone(
    out: &mut Vec<Row>,
    row: &[V],
    b: &mut ExecutionBudgetV2,
) -> Result<(), QueryErrorV2> {
    b.reserve(row_bytes(row).saturating_add(size_of::<Row>() as u64))?;
    out.push(row.to_vec());
    Ok(())
}
fn finite(v: f64) -> Result<V, QueryErrorV2> {
    if v.is_finite() {
        Ok(V::F64(v))
    } else {
        Err(failure("nonfinite_result"))
    }
}
fn compare(a: &V, b: &V) -> Result<Ordering, QueryErrorV2> {
    match (a, b) {
        (V::Bool(a), V::Bool(b)) => Ok(a.cmp(b)),
        (V::I64(a), V::I64(b)) => Ok(a.cmp(b)),
        (V::F64(a), V::F64(b)) => a.partial_cmp(b).ok_or_else(|| failure("nonfinite_result")),
        (V::Utf8(a), V::Utf8(b)) => Ok(a.cmp(b)),
        _ => Err(failure("comparator_unavailable")),
    }
}
fn equal(a: &V, b: &V) -> bool {
    a == b
}
fn eval(e: &BoundExpr, row: &[V], budget: &mut ExecutionBudgetV2) -> Result<V, QueryErrorV2> {
    budget.check()?;
    budget.reserve(64)?;
    let mut stack = vec![(e, false)];
    let mut ready = std::collections::BTreeMap::new();
    while let Some((current, visited)) = stack.pop() {
        budget.check()?;
        if visited {
            let value = eval_node(current, row, budget, &mut ready)?;
            ready.insert(current as *const BoundExpr as usize, value);
            continue;
        }
        budget.reserve(192)?;
        stack.push((current, true));
        match &current.kind {
            ExprKind::Binary(_, left, right) => {
                budget.reserve(64)?;
                stack.push((right, false));
                stack.push((left, false));
            }
            ExprKind::Unary(_, arg) | ExprKind::Lower(arg) | ExprKind::Length(arg) => {
                budget.reserve(32)?;
                stack.push((arg, false));
            }
            ExprKind::In(expression, values, _) => {
                budget.reserve(((values.len() + 1) as u64).saturating_mul(32))?;
                stack.extend(values.iter().rev().map(|e| (e, false)));
                stack.push((expression, false));
            }
            _ => {}
        }
    }
    ready
        .remove(&(e as *const BoundExpr as usize))
        .ok_or_else(|| failure("bound_expression"))
}
fn eval_node(
    e: &BoundExpr,
    row: &[V],
    budget: &mut ExecutionBudgetV2,
    ready: &mut std::collections::BTreeMap<usize, V>,
) -> Result<V, QueryErrorV2> {
    let mut take = |e: &BoundExpr| {
        ready
            .remove(&(e as *const BoundExpr as usize))
            .ok_or_else(|| failure("bound_expression"))
    };
    match &e.kind {
        ExprKind::Literal(v) => clone_value(v, budget),
        ExprKind::Field(i) => {
            clone_value(row.get(*i).ok_or_else(|| failure("bound_field"))?, budget)
        }
        ExprKind::Lower(e) => {
            let v = take(e)?;
            match v {
                V::Null => Ok(V::Null),
                V::Utf8(s) => {
                    budget.reserve((s.len() as u64).saturating_mul(3))?;
                    Ok(V::Utf8(s.to_lowercase()))
                }
                _ => Err(failure("bound_type")),
            }
        }
        ExprKind::Length(e) => {
            let v = take(e)?;
            match v {
                V::Null => Ok(V::Null),
                V::Utf8(s) => Ok(V::I64(
                    i64::try_from(s.chars().count()).map_err(|_| failure("integer_overflow"))?,
                )),
                _ => Err(failure("bound_type")),
            }
        }
        ExprKind::Unary(op, e) => {
            let v = take(e)?;
            Ok(match (op, v) {
                (UnaryOp::IsNull, v) => V::Bool(v == V::Null),
                (UnaryOp::IsNotNull, v) => V::Bool(v != V::Null),
                (_, V::Null) => V::Null,
                (UnaryOp::Not, V::Bool(v)) => V::Bool(!v),
                (UnaryOp::Neg, V::I64(v)) => {
                    V::I64(v.checked_neg().ok_or_else(|| failure("integer_overflow"))?)
                }
                (UnaryOp::Neg, V::F64(v)) => return finite(-v),
                _ => return Err(failure("bound_type")),
            })
        }
        ExprKind::Binary(op, left, right) => {
            // Deliberately eager and left-to-right, including AND and OR.
            let a = take(left)?;
            let b = take(right)?;
            binary(op, a, b)
        }
        ExprKind::In(needle, values, negated) => {
            let needle = take(needle)?;
            let mut found = false;
            let mut null = false;
            for v in values {
                let v = take(v)?;
                if needle == V::Null || v == V::Null {
                    null = true
                } else if equal(&needle, &v) {
                    found = true
                }
            }
            Ok(if found {
                V::Bool(!negated)
            } else if null {
                V::Null
            } else {
                V::Bool(*negated)
            })
        }
    }
}
fn binary(op: &BinaryOp, a: V, b: V) -> Result<V, QueryErrorV2> {
    use BinaryOp::*;
    if matches!(op, And | Or) {
        let truth = |v: &V| match v {
            V::Bool(b) => Some(*b),
            _ => None,
        };
        let x = truth(&a);
        let y = truth(&b);
        return Ok(match op {
            And => {
                if x == Some(false) || y == Some(false) {
                    V::Bool(false)
                } else if x == Some(true) && y == Some(true) {
                    V::Bool(true)
                } else {
                    V::Null
                }
            }
            _ => {
                if x == Some(true) || y == Some(true) {
                    V::Bool(true)
                } else if x == Some(false) && y == Some(false) {
                    V::Bool(false)
                } else {
                    V::Null
                }
            }
        });
    }
    if a == V::Null || b == V::Null {
        return Ok(V::Null);
    }
    match op {
        Eq => Ok(V::Bool(equal(&a, &b))),
        Ne => Ok(V::Bool(!equal(&a, &b))),
        Lt | Le | Gt | Ge => {
            let order = compare(&a, &b)?;
            Ok(V::Bool(match op {
                Lt => order.is_lt(),
                Le => order.is_le(),
                Gt => order.is_gt(),
                _ => order.is_ge(),
            }))
        }
        Contains | Startswith => match (a, b) {
            (V::Utf8(a), V::Utf8(b)) => Ok(V::Bool(if matches!(op, Contains) {
                a.contains(&b)
            } else {
                a.starts_with(&b)
            })),
            _ => Err(failure("bound_type")),
        },
        Add | Sub | Mul | Div => match (a, b) {
            (V::I64(a), V::I64(b)) => {
                if matches!(op, Div) && b == 0 {
                    return Err(failure("division_by_zero"));
                }
                let v = match op {
                    Add => a.checked_add(b),
                    Sub => a.checked_sub(b),
                    Mul => a.checked_mul(b),
                    _ => a.checked_div(b),
                };
                Ok(V::I64(v.ok_or_else(|| failure("integer_overflow"))?))
            }
            (V::F64(a), V::F64(b)) => {
                if matches!(op, Div) && b == 0.0 {
                    return Err(failure("division_by_zero"));
                }
                finite(match op {
                    Add => a + b,
                    Sub => a - b,
                    Mul => a * b,
                    _ => a / b,
                })
            }
            _ => Err(failure("bound_type")),
        },
        _ => Err(failure("bound_operator")),
    }
}

fn aggregate(
    call: &Aggregate,
    rows: &[&Row],
    b: &mut ExecutionBudgetV2,
) -> Result<V, QueryErrorV2> {
    use AggregateKind::*;
    if matches!(call.kind, CountAll) {
        return Ok(V::I64(
            i64::try_from(rows.len()).map_err(|_| failure("integer_overflow"))?,
        ));
    }
    let arg = call
        .arg
        .as_ref()
        .ok_or_else(|| failure("bound_aggregate"))?;
    let mut count = 0i64;
    let mut sum = None;
    let mut extremum = None;
    let mut collected = Vec::new();
    for row in rows {
        b.check()?;
        let value = eval(arg, row, b)?;
        if matches!(call.kind, Collect) {
            if collected.len() as u64 >= b.rows {
                return Err(quota());
            }
            b.reserve((size_of::<V>() * 2) as u64)?;
            collected.push(value);
            continue;
        }
        if value == V::Null {
            continue;
        }
        count = count
            .checked_add(1)
            .ok_or_else(|| failure("integer_overflow"))?;
        match call.kind {
            Sum | Avg => {
                sum = Some(if let Some(old) = sum {
                    binary(&BinaryOp::Add, old, value)?
                } else {
                    value
                });
            }
            Min | Max => {
                let replace = if let Some(ref old) = extremum {
                    let c = compare(&value, old)?;
                    if matches!(call.kind, Min) {
                        c.is_lt()
                    } else {
                        c.is_gt()
                    }
                } else {
                    true
                };
                if replace {
                    extremum = Some(value);
                }
            }
            _ => {}
        }
    }
    Ok(match call.kind {
        Count => V::I64(count),
        Sum => sum.unwrap_or(V::Null),
        Avg => match sum {
            None => V::Null,
            Some(V::I64(v)) => finite(v as f64 / count as f64)?,
            Some(V::F64(v)) => finite(v / count as f64)?,
            _ => return Err(failure("bound_type")),
        },
        Min | Max => extremum.unwrap_or(V::Null),
        Collect => V::List(collected),
        CountAll => unreachable!(),
    })
}

pub(crate) fn execute_v2(
    plan: &PhysicalPlanV2,
    budget: &mut ExecutionBudgetV2,
) -> Result<ExecutionOutputV2, QueryErrorV2> {
    budget.check()?;
    budget.reserve((plan.nodes().len() as u64).saturating_mul(1024))?;
    let mut outputs: Vec<Vec<Row>> = Vec::with_capacity(plan.nodes().len());
    let mut explain = plan.explain_nodes();
    for (index, node) in plan.nodes().iter().enumerate() {
        budget.check()?;
        let started = Instant::now();
        let mut out = Vec::new();
        let input = node
            .inputs
            .first()
            .map(|i| outputs[*i].as_slice())
            .unwrap_or(&[]);
        let input_rows = node.inputs.iter().map(|i| outputs[*i].len() as u64).sum();
        match &node.kernel {
            Kernel::Values(value) => {
                let V::List(values) = value.as_ref() else {
                    return Err(failure("bound_values"));
                };
                for v in values {
                    budget.reserve((size_of::<Row>() * 2) as u64)?;
                    let v = clone_value(v, budget)?;
                    out.push(vec![v]);
                }
            }
            Kernel::Filter(predicate) => {
                for row in input {
                    if eval(predicate, row, budget)? == V::Bool(true) {
                        push_clone(&mut out, row, budget)?;
                    }
                }
            }
            Kernel::Project(expressions) => {
                for row in input {
                    budget.reserve(
                        ((size_of::<Row>() * 2) + (size_of::<V>() * expressions.len())) as u64,
                    )?;
                    let mut projected = Vec::with_capacity(expressions.len());
                    for expr in expressions {
                        projected.push(eval(expr, row, budget)?);
                    }
                    out.push(projected);
                }
            }
            Kernel::Distinct => {
                for row in input {
                    budget.check()?;
                    let mut exists = false;
                    for old in &out {
                        budget.check()?;
                        if old == row {
                            exists = true;
                            break;
                        }
                    }
                    if !exists {
                        push_clone(&mut out, row, budget)?;
                    }
                }
            }
            Kernel::Take(count) => {
                for row in input
                    .iter()
                    .take(usize::try_from(*count).unwrap_or(usize::MAX))
                {
                    push_clone(&mut out, row, budget)?;
                }
            }
            Kernel::Offset(count) => {
                for row in input
                    .iter()
                    .skip(usize::try_from(*count).unwrap_or(usize::MAX))
                {
                    push_clone(&mut out, row, budget)?;
                }
            }
            Kernel::UnionAll => {
                for i in &node.inputs {
                    for row in &outputs[*i] {
                        push_clone(&mut out, row, budget)?;
                    }
                }
            }
            Kernel::Sort(keys) => {
                let mut decorated = Vec::new();
                for (i, row) in input.iter().enumerate() {
                    budget.reserve(
                        ((size_of::<usize>() + size_of::<Vec<V>>()) * 2
                            + keys.len() * size_of::<V>()) as u64,
                    )?;
                    let values = keys
                        .iter()
                        .map(|(e, _, _)| eval(e, row, budget))
                        .collect::<Result<Vec<_>, _>>()?;
                    decorated.push((i, values));
                }
                // Stable insertion-free sort; all throwing expressions were evaluated
                // before sorting. Track timeout in comparator, fail after sort.
                let mut error = None;
                decorated.sort_by(|a, b| {
                    // Keep a consistent comparator after deadline; sorting cannot
                    // publish success and the post-sort check returns this error.
                    if let Err(e) = budget.check() {
                        error = Some(e);
                    }
                    for (k, (_, direction, nulls)) in keys.iter().enumerate() {
                        let (x, y) = (&a.1[k], &b.1[k]);
                        let order = match (x == &V::Null, y == &V::Null) {
                            (true, true) => Ordering::Equal,
                            (true, false) => {
                                if matches!(nulls, NullOrder::First) {
                                    Ordering::Less
                                } else {
                                    Ordering::Greater
                                }
                            }
                            (false, true) => {
                                if matches!(nulls, NullOrder::First) {
                                    Ordering::Greater
                                } else {
                                    Ordering::Less
                                }
                            }
                            (false, false) => match compare(x, y) {
                                Ok(o) => {
                                    if matches!(direction, SortDirection::Desc) {
                                        o.reverse()
                                    } else {
                                        o
                                    }
                                }
                                Err(e) => {
                                    error = Some(e);
                                    return Ordering::Equal;
                                }
                            },
                        };
                        if order != Ordering::Equal {
                            return order;
                        }
                    }
                    a.0.cmp(&b.0)
                });
                if let Some(error) = error {
                    return Err(error);
                }
                for (i, _) in decorated {
                    push_clone(&mut out, &input[i], budget)?;
                }
            }
            Kernel::Join {
                kind,
                condition,
                right_width,
            } => {
                let right = &outputs[node.inputs[1]];
                for left in input {
                    let mut matched = false;
                    for r in right {
                        budget.reserve(row_bytes(left).saturating_add(row_bytes(r)))?;
                        let mut combined = left.clone();
                        combined.extend_from_slice(r);
                        if eval(condition, &combined, budget)? == V::Bool(true) {
                            matched = true;
                            match kind {
                                JoinKind::Inner | JoinKind::Left => {
                                    budget.reserve((size_of::<Row>() * 2) as u64)?;
                                    out.push(combined);
                                }
                                JoinKind::Semi | JoinKind::Anti => {}
                            }
                        }
                    }
                    match kind {
                        JoinKind::Semi if matched => push_clone(&mut out, left, budget)?,
                        JoinKind::Anti if !matched => push_clone(&mut out, left, budget)?,
                        JoinKind::Left if !matched => {
                            budget.reserve(row_bytes(left).saturating_add(
                                (right_width * size_of::<V>() + size_of::<Row>()) as u64,
                            ))?;
                            let mut row = left.clone();
                            row.resize(row.len() + right_width, V::Null);
                            out.push(row);
                        }
                        _ => {}
                    }
                }
            }
            Kernel::Aggregate { groups, aggregates } => {
                let mut buckets: Vec<(Row, Vec<&Row>)> = Vec::new();
                if groups.is_empty() {
                    budget.reserve(128)?;
                    buckets.push((Vec::new(), Vec::new()));
                }
                for row in input {
                    budget.reserve((groups.len() * size_of::<V>() + size_of::<Row>()) as u64)?;
                    let key = groups
                        .iter()
                        .map(|e| eval(e, row, budget))
                        .collect::<Result<Row, _>>()?;
                    let mut found = None;
                    for (i, (old, _)) in buckets.iter().enumerate() {
                        budget.check()?;
                        if *old == key {
                            found = Some(i);
                            break;
                        }
                    }
                    let i = if let Some(i) = found {
                        i
                    } else {
                        budget.reserve(128)?;
                        buckets.push((key, Vec::new()));
                        buckets.len() - 1
                    };
                    budget.reserve((size_of::<&Row>() * 2) as u64)?;
                    buckets[i].1.push(row);
                }
                for (mut key, rows) in buckets {
                    budget.reserve(
                        (aggregates.len() * size_of::<V>() + 2 * size_of::<Row>()) as u64,
                    )?;
                    for call in aggregates {
                        key.push(aggregate(call, &rows, budget)?);
                    }
                    out.push(key);
                }
            }
        }
        budget.check()?;
        explain[index].actual = Some(ActualCountersV2 {
            input_rows: Some(input_rows),
            output_rows: Some(out.len() as u64),
            elapsed_micros: Some(u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)),
            ..Default::default()
        });
        outputs.push(out);
    }
    let rows = std::mem::take(&mut outputs[plan.root_index()]);
    // Count boundary serialization without materializing another output buffer.
    struct Counter(u64);
    impl std::io::Write for Counter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(buf.len() as u64);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, &rows).map_err(|_| failure("result_encoding"))?;
    budget.check_output(rows.len() as u64, counter.0)?;
    Ok(ExecutionOutputV2 {
        rows,
        nodes: explain,
    })
}
