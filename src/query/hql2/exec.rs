//! Native, exact scalar kernels. No Storage access or JSON row protocol.
use super::{
    bind::{
        Aggregate, AggregateKind, BoundExpr, BoundMatchAnchor, BoundPatternProperty,
        BoundSequenceNode, BoundSequenceStep, ExprKind, Kernel, VectorRankKind,
    },
    error::QueryErrorV2,
    plan::PhysicalPlanV2,
    result::{
        ActualCountersV2, CounterClockSourceV2, CounterElapsedScopeV2, CounterReadingV2,
        CounterSamplingV2, CounterUnitV2, CounterUnknownReasonV2, ExplainNodeV2,
    },
    source::{
        ExecBatchV2, FieldIdV2, GraphSnapshotV2, HydratedValuesV2, RecordKeyV2, VectorBatchV2,
    },
    value::{
        ContextEvidenceV2, ContextFragmentV2, ContextPackageV2, QueryPathModeV2, QueryValueV2 as V,
        ScoreValueV2, VectorValueV2,
    },
    wire::{BinaryOp, Direction, JoinKind, NullOrder, PathMode, SortDirection, UnaryOp},
};
use crate::uee_v2::{QueryBudgetV2, RecordRefV2};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    mem::size_of,
    time::{Duration, Instant},
};

type AnnotationLookupFnV2<'a> =
    dyn FnMut(&RecordRefV2, &mut ExecutionBudgetV2) -> Result<Vec<RecordRefV2>, QueryErrorV2> + 'a;
type HydrateFnV2<'a> = dyn FnMut(&[RecordRefV2], &[FieldIdV2], &mut ExecutionBudgetV2) -> Result<ExecBatchV2, QueryErrorV2>
    + 'a;
type VectorLookupFnV2<'a> = dyn FnMut(&[RecordRefV2], &str, bool, &mut ExecutionBudgetV2) -> Result<VectorBatchV2, QueryErrorV2>
    + 'a;
type CompactPathV2 = (Vec<RecordRefV2>, Vec<RecordRefV2>);

fn failure(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("BIND_ERROR", "execute", reason)
}
fn quota() -> QueryErrorV2 {
    QueryErrorV2::new("QUERY_BUDGET_EXCEEDED", "execute", "query_limit")
}

fn record_kind_name(kind: &crate::uee_v2::RecordKindV2) -> &'static str {
    match kind {
        crate::uee_v2::RecordKindV2::Node => "node",
        crate::uee_v2::RecordKindV2::Edge => "edge",
        crate::uee_v2::RecordKindV2::Row => "row",
        crate::uee_v2::RecordKindV2::Vector => "vector",
        crate::uee_v2::RecordKindV2::Annotation => "annotation",
        crate::uee_v2::RecordKindV2::Artifact => "artifact",
    }
}

pub(crate) struct ExecutionBudgetV2 {
    started: Instant,
    elapsed: Duration,
    memory: u64,
    accounted: u64,
    rows: u64,
    bytes: u64,
    expanded_nodes: u64,
    expanded_nodes_used: u64,
    expanded_edges: u64,
    expanded_edges_used: u64,
    vector_candidates: u64,
    vector_candidates_used: u64,
    distance_evaluations: u64,
    distance_evaluations_used: u64,
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
            expanded_nodes: 100_000,
            expanded_nodes_used: 0,
            expanded_edges: 200_000,
            expanded_edges_used: 0,
            vector_candidates: 100_000,
            vector_candidates_used: 0,
            distance_evaluations: 1_000_000,
            distance_evaluations_used: 0,
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
            budget.expanded_nodes = limit(o.max_expanded_nodes, 100_000)?;
            budget.expanded_edges = limit(o.max_expanded_edges, 200_000)?;
            budget.vector_candidates = limit(o.max_vector_candidates, 100_000)?;
            budget.distance_evaluations = limit(o.max_distance_evaluations, 1_000_000)?;
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
    pub(crate) fn reserve_expanded_nodes(&mut self, count: u64) -> Result<(), QueryErrorV2> {
        self.check()?;
        let next = self
            .expanded_nodes_used
            .checked_add(count)
            .ok_or_else(quota)?;
        if next > self.expanded_nodes {
            return Err(quota());
        }
        self.expanded_nodes_used = next;
        Ok(())
    }
    pub(crate) fn reserve_expanded_edges(&mut self, count: u64) -> Result<(), QueryErrorV2> {
        self.check()?;
        let next = self
            .expanded_edges_used
            .checked_add(count)
            .ok_or_else(quota)?;
        if next > self.expanded_edges {
            return Err(quota());
        }
        self.expanded_edges_used = next;
        Ok(())
    }
    pub(crate) fn reserve_vector_candidates(&mut self, count: u64) -> Result<(), QueryErrorV2> {
        self.check()?;
        let next = self
            .vector_candidates_used
            .checked_add(count)
            .ok_or_else(quota)?;
        if next > self.vector_candidates {
            return Err(quota());
        }
        self.vector_candidates_used = next;
        Ok(())
    }
    pub(crate) fn reserve_distance_evaluations(&mut self, count: u64) -> Result<(), QueryErrorV2> {
        self.check()?;
        let next = self
            .distance_evaluations_used
            .checked_add(count)
            .ok_or_else(quota)?;
        if next > self.distance_evaluations {
            return Err(quota());
        }
        self.distance_evaluations_used = next;
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
        V::Json(value) => json_value_bytes(value),
        V::Entity(record) => {
            (record.database_id.len()
                + record.namespace.len()
                + record.id.len()
                + record.revision.len()) as u64
        }
        V::HistoryRevision(revision) => {
            (revision.subject.database_id.len()
                + revision.subject.namespace.len()
                + revision.subject.id.len()
                + revision.subject.revision.len()
                + revision.operation.len()
                + revision.valid_from.len()
                + revision.valid_to.as_ref().map_or(0, String::len)) as u64
        }
        V::ChangeEvent(event) => {
            (event.subject.database_id.len()
                + event.subject.namespace.len()
                + event.subject.id.len()
                + event.subject.revision.len()
                + event.operation.len()) as u64
        }
        V::Score(score) => {
            (score.owner.database_id.len()
                + score.owner.namespace.len()
                + score.owner.id.len()
                + score.owner.revision.len()
                + score.source.len()
                + score.scope.len()
                + score.metric.len()) as u64
        }
        V::Context(context) => {
            context.rendered_context.len() as u64
                + context.tokenizer_fingerprint.len() as u64
                + context
                    .fragments
                    .iter()
                    .map(|fragment| {
                        fragment.text.len() as u64
                            + fragment.citation.len() as u64
                            + fragment.evidence.source.id.len() as u64
                            + fragment.evidence.source.revision.len() as u64
                            + fragment.evidence.source_hash.len() as u64
                    })
                    .sum::<u64>()
                + context
                    .omitted_refs
                    .iter()
                    .map(|evidence| {
                        evidence.source.id.len() as u64
                            + evidence.source.revision.len() as u64
                            + evidence.source_hash.len() as u64
                    })
                    .sum::<u64>()
        }
        V::Path(path) => path
            .vertices
            .iter()
            .chain(&path.edges)
            .map(|record| {
                (record.database_id.len()
                    + record.namespace.len()
                    + record.id.len()
                    + record.revision.len()) as u64
                    + size_of::<RecordRefV2>() as u64
            })
            .sum(),
        V::List(v) => v.iter().map(bytes).fold(0u64, u64::saturating_add),
        _ => 0,
    };
    (size_of::<V>() as u64).saturating_add(heap)
}
fn entity_value_bytes(record: &RecordRefV2) -> u64 {
    (size_of::<V>() as u64).saturating_add(
        (record.database_id.len()
            + record.namespace.len()
            + record.id.len()
            + record.revision.len()) as u64,
    )
}
pub(crate) fn json_value_bytes(value: &Value) -> u64 {
    let mut total = 0u64;
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        total = total.saturating_add(32);
        match value {
            Value::String(value) => total = total.saturating_add(value.len() as u64),
            Value::Array(values) => pending.extend(values),
            Value::Object(values) => {
                for (key, value) in values {
                    total = total.saturating_add(key.len() as u64);
                    pending.push(value);
                }
            }
            _ => {}
        }
    }
    total
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
fn push_with_value(
    out: &mut Vec<Row>,
    row: &[V],
    value: V,
    budget: &mut ExecutionBudgetV2,
) -> Result<(), QueryErrorV2> {
    budget.reserve(
        row_bytes(row)
            .saturating_add(bytes(&value))
            .saturating_add(size_of::<Row>() as u64),
    )?;
    let mut extended = Vec::with_capacity(row.len().saturating_add(1));
    extended.extend(row.iter().cloned());
    extended.push(value);
    out.push(extended);
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
        (V::DecimalU64(a), V::DecimalU64(b)) => Ok(a.cmp(b)),
        (V::F64(a), V::F64(b)) => a.partial_cmp(b).ok_or_else(|| failure("nonfinite_result")),
        (V::Utf8(a), V::Utf8(b)) => Ok(a.cmp(b)),
        _ => Err(failure("comparator_unavailable")),
    }
}

fn exact_vector_distance(
    query: &VectorValueV2,
    candidate: &VectorValueV2,
    metric: &str,
) -> Result<f64, QueryErrorV2> {
    if query.space_id != candidate.space_id
        || query.values.len() != candidate.values.len()
        || query
            .values
            .iter()
            .chain(&candidate.values)
            .any(|value| !value.is_finite())
    {
        return Err(QueryErrorV2::new(
            "COLLECTION_SPACE_MISMATCH",
            "execute",
            "vector_space",
        ));
    }
    match metric {
        "L2" => {
            let distance = query
                .values
                .iter()
                .zip(&candidate.values)
                .map(|(left, right)| {
                    let delta = left - right;
                    delta * delta
                })
                .sum::<f64>();
            if distance.is_finite() {
                Ok(distance)
            } else {
                Err(QueryErrorV2::new(
                    "BIND_ERROR",
                    "execute",
                    "nonfinite_result",
                ))
            }
        }
        "Cosine" => {
            let query_scale = query
                .values
                .iter()
                .fold(0.0_f64, |scale, value| scale.max(value.abs()));
            let candidate_scale = candidate
                .values
                .iter()
                .fold(0.0_f64, |scale, value| scale.max(value.abs()));
            if query_scale == 0.0 || candidate_scale == 0.0 {
                return Err(QueryErrorV2::new(
                    "COLLECTION_SPACE_MISMATCH",
                    "execute",
                    "zero_cosine_norm",
                ));
            }
            let mut dot = 0.0;
            let mut query_norm = 0.0;
            let mut candidate_norm = 0.0;
            for (left, right) in query.values.iter().zip(&candidate.values) {
                let left = left / query_scale;
                let right = right / candidate_scale;
                dot += left * right;
                query_norm += left * left;
                candidate_norm += right * right;
            }
            let similarity = dot / (query_norm.sqrt() * candidate_norm.sqrt());
            if similarity.is_finite() {
                Ok(1.0 - similarity.clamp(-1.0, 1.0))
            } else {
                Err(QueryErrorV2::new(
                    "BIND_ERROR",
                    "execute",
                    "nonfinite_result",
                ))
            }
        }
        _ => Err(QueryErrorV2::new(
            "CAPABILITY_UNSUPPORTED",
            "execute",
            "vector_metric_unavailable",
        )),
    }
}
fn equal(a: &V, b: &V) -> bool {
    a == b
}
fn matches_match_anchors(row: &[V], anchors: &[BoundMatchAnchor]) -> Result<bool, QueryErrorV2> {
    for anchor in anchors {
        let value = row
            .get(anchor.column_index)
            .ok_or_else(|| failure("match_anchor_binding"))?;
        if !matches!(value, V::Entity(record) if record == &anchor.record) {
            return Ok(false);
        }
    }
    Ok(true)
}
fn matches_compact_match_anchors(
    start: &RecordRefV2,
    endpoint: &RecordRefV2,
    single_node_alias: bool,
    anchors: &[BoundMatchAnchor],
) -> Result<bool, QueryErrorV2> {
    for anchor in anchors {
        let record = match anchor.column_index {
            0 => start,
            1 if !single_node_alias => endpoint,
            _ => return Err(failure("match_anchor_binding")),
        };
        if record != &anchor.record {
            return Ok(false);
        }
    }
    Ok(true)
}
fn matches_pattern_properties(
    properties: &BTreeMap<String, BoundPatternProperty>,
    record: &RecordRefV2,
    input_row: &[V],
    budget: &mut ExecutionBudgetV2,
    hydrated: &HydratedValuesV2,
) -> Result<bool, QueryErrorV2> {
    for (name, property) in properties {
        budget.check()?;
        let present = hydrated
            .get(record, &property.has_property_field, name)
            .ok_or_else(|| failure("pattern_property_hydration"))?;
        if present != &V::Bool(true) {
            return Ok(false);
        }
        let actual = hydrated
            .get(record, &property.value_field, name)
            .ok_or_else(|| failure("pattern_property_hydration"))?;
        let expected = eval(&property.expected, input_row, budget, hydrated)?;
        if !matches!(actual, V::Json(_)) || actual != &expected {
            return Ok(false);
        }
    }
    Ok(true)
}

fn eval(
    e: &BoundExpr,
    row: &[V],
    budget: &mut ExecutionBudgetV2,
    hydrated: &HydratedValuesV2,
) -> Result<V, QueryErrorV2> {
    budget.check()?;
    budget.reserve(64)?;
    let mut stack = vec![(e, false)];
    let mut ready = std::collections::BTreeMap::new();
    while let Some((current, visited)) = stack.pop() {
        budget.check()?;
        if visited {
            let value = eval_node(current, row, budget, hydrated, &mut ready)?;
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
            ExprKind::Property { entity, name, .. } => {
                budget.reserve(64)?;
                stack.push((name, false));
                stack.push((entity, false));
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
    hydrated: &HydratedValuesV2,
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
        ExprKind::EntityRefField(index, field) => {
            let entity = row.get(*index).ok_or_else(|| failure("bound_field"))?;
            if entity == &V::Null {
                return Ok(V::Null);
            }
            let record = match entity {
                V::Entity(record) => record,
                _ => return Err(failure("bound_type")),
            };
            let value = match field.as_str() {
                "database_id" => record.database_id.clone(),
                "namespace" => record.namespace.clone(),
                "kind" => match record.kind {
                    crate::uee_v2::RecordKindV2::Node => "node",
                    crate::uee_v2::RecordKindV2::Edge => "edge",
                    crate::uee_v2::RecordKindV2::Row => "row",
                    crate::uee_v2::RecordKindV2::Vector => "vector",
                    crate::uee_v2::RecordKindV2::Annotation => "annotation",
                    crate::uee_v2::RecordKindV2::Artifact => "artifact",
                }
                .to_owned(),
                "id" => record.id.clone(),
                "revision" => record.revision.clone(),
                _ => return Err(failure("bound_field")),
            };
            clone_value(&V::Utf8(value), budget)
        }
        ExprKind::ScoreField(index, field) => {
            let score = row.get(*index).ok_or_else(|| failure("bound_field"))?;
            let V::Score(score) = score else {
                return Err(failure("bound_type"));
            };
            match field.as_str() {
                "value" | "distance" => clone_value(&V::F64(score.value), budget),
                "owner" => clone_value(&V::Entity(score.owner.clone()), budget),
                "source" => clone_value(&V::Utf8(score.source.clone()), budget),
                "scope" => clone_value(&V::Utf8(score.scope.clone()), budget),
                "metric" => clone_value(&V::Utf8(score.metric.clone()), budget),
                _ => Err(failure("bound_field")),
            }
        }
        ExprKind::HistoryField(index, field) => {
            let value = row.get(*index).ok_or_else(|| failure("bound_field"))?;
            let V::HistoryRevision(revision) = value else {
                return Err(failure("bound_type"));
            };
            match field.as_str() {
                "subject" => clone_value(&V::Entity(revision.subject.clone()), budget),
                "operation" => clone_value(&V::Utf8(revision.operation.clone()), budget),
                "tx_from" => clone_value(&V::DecimalU64(revision.tx_from), budget),
                "tx_to" => match revision.tx_to {
                    Some(value) => clone_value(&V::DecimalU64(value), budget),
                    None => Ok(V::Null),
                },
                "valid_from" => clone_value(&V::Utf8(revision.valid_from.clone()), budget),
                "valid_to" => match &revision.valid_to {
                    Some(value) => clone_value(&V::Utf8(value.clone()), budget),
                    None => Ok(V::Null),
                },
                "id" => clone_value(&V::Utf8(revision.subject.id.clone()), budget),
                "kind" => clone_value(
                    &V::Utf8(record_kind_name(&revision.subject.kind).into()),
                    budget,
                ),
                "revision_id" => clone_value(&V::Utf8(revision.subject.revision.clone()), budget),
                _ => Err(failure("bound_field")),
            }
        }
        ExprKind::ChangeField(index, field) => {
            let value = row.get(*index).ok_or_else(|| failure("bound_field"))?;
            let V::ChangeEvent(event) = value else {
                return Err(failure("bound_type"));
            };
            match field.as_str() {
                "sequence" => clone_value(&V::DecimalU64(event.sequence), budget),
                "operation" => clone_value(&V::Utf8(event.operation.clone()), budget),
                "subject" => clone_value(&V::Entity(event.subject.clone()), budget),
                _ => Err(failure("bound_field")),
            }
        }
        ExprKind::Property {
            entity,
            name,
            has_property,
            field,
        } => {
            let entity = take(entity)?;
            let name = take(name)?;
            let V::Utf8(name) = name else {
                return Err(failure("bound_type"));
            };
            match entity {
                V::Null => Ok(if *has_property {
                    V::Bool(false)
                } else {
                    V::Null
                }),
                V::Entity(record) => {
                    let identity_bytes = (record.database_id.len() as u64)
                        .saturating_add(record.namespace.len() as u64)
                        .saturating_add(record.id.len() as u64)
                        .saturating_add(record.revision.len() as u64);
                    budget.reserve(
                        identity_bytes
                            .saturating_add(name.len() as u64)
                            .saturating_add(128),
                    )?;
                    let value = hydrated
                        .get(&record, field, &name)
                        .ok_or_else(|| failure("source_hydration_missing"))?;
                    budget.reserve(bytes(value))?;
                    Ok(value.clone())
                }
                _ => Err(failure("bound_type")),
            }
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
        Add | Sub | Mul | Div | Rem => match (a, b) {
            (V::I64(a), V::I64(b)) => {
                if matches!(op, Div | Rem) && b == 0 {
                    return Err(failure("division_by_zero"));
                }
                let v = match op {
                    Add => a.checked_add(b),
                    Sub => a.checked_sub(b),
                    Mul => a.checked_mul(b),
                    Div => a.checked_div(b),
                    Rem => a.checked_rem(b),
                    _ => return Err(failure("bound_operator")),
                };
                Ok(V::I64(v.ok_or_else(|| failure("integer_overflow"))?))
            }
            (V::F64(a), V::F64(b)) => {
                if matches!(op, Div | Rem) && b == 0.0 {
                    return Err(failure("division_by_zero"));
                }
                finite(match op {
                    Add => a + b,
                    Sub => a - b,
                    Mul => a * b,
                    Div => a / b,
                    Rem => a % b,
                    _ => return Err(failure("bound_operator")),
                })
            }
            _ => Err(failure("bound_type")),
        },
        _ => Err(failure("bound_operator")),
    }
}

struct PropertyNeedV2<'a> {
    entity: &'a BoundExpr,
    name: &'a BoundExpr,
    field: &'a FieldIdV2,
}

fn kernel_expression_roots<'a>(
    kernel: &'a Kernel,
    budget: &mut ExecutionBudgetV2,
) -> Result<Vec<&'a BoundExpr>, QueryErrorV2> {
    let count = match kernel {
        Kernel::Filter(_) | Kernel::Join { .. } => 1,
        Kernel::Project(expressions) => expressions.len(),
        Kernel::Sort(keys) => keys.len(),
        Kernel::Aggregate { groups, aggregates } => {
            groups.len()
                + aggregates
                    .iter()
                    .filter(|aggregate| aggregate.arg.is_some())
                    .count()
        }
        Kernel::ContextPack { .. } => 2,
        _ => 0,
    };
    budget.reserve((count as u64).saturating_mul(size_of::<&BoundExpr>() as u64))?;
    let mut roots = Vec::with_capacity(count);
    match kernel {
        Kernel::Filter(expression) => roots.push(expression),
        Kernel::Project(expressions) => roots.extend(expressions),
        Kernel::Sort(keys) => roots.extend(keys.iter().map(|(expression, _, _)| expression)),
        Kernel::Aggregate { groups, aggregates } => {
            roots.extend(groups);
            roots.extend(
                aggregates
                    .iter()
                    .filter_map(|aggregate| aggregate.arg.as_ref()),
            );
        }
        Kernel::Join { condition, .. } => roots.push(condition),
        Kernel::ContextPack { text, evidence, .. } => {
            roots.push(text);
            roots.push(evidence);
        }
        _ => {}
    }
    Ok(roots)
}

fn property_needs<'a>(
    roots: Vec<&'a BoundExpr>,
    budget: &mut ExecutionBudgetV2,
) -> Result<Vec<PropertyNeedV2<'a>>, QueryErrorV2> {
    let mut stack = roots;
    let mut seen = BTreeSet::new();
    let mut needs = Vec::new();
    while let Some(expression) = stack.pop() {
        budget.check()?;
        match &expression.kind {
            ExprKind::Property {
                entity,
                name,
                field,
                ..
            } => {
                if seen.insert(field.index()) {
                    budget.reserve(64)?;
                    needs.push(PropertyNeedV2 {
                        entity,
                        name,
                        field,
                    });
                }
                budget.reserve((2 * size_of::<&BoundExpr>()) as u64)?;
                stack.push(name);
                stack.push(entity);
            }
            ExprKind::Binary(_, left, right) => {
                budget.reserve((2 * size_of::<&BoundExpr>()) as u64)?;
                stack.push(right);
                stack.push(left);
            }
            ExprKind::Unary(_, argument)
            | ExprKind::Lower(argument)
            | ExprKind::Length(argument) => {
                budget.reserve(size_of::<&BoundExpr>() as u64)?;
                stack.push(argument)
            }
            ExprKind::In(expression, values, _) => {
                budget.reserve(
                    ((values.len() + 1) as u64).saturating_mul(size_of::<&BoundExpr>() as u64),
                )?;
                stack.extend(values.iter());
                stack.push(expression);
            }
            ExprKind::Literal(_)
            | ExprKind::Field(_)
            | ExprKind::EntityRefField(_, _)
            | ExprKind::ScoreField(_, _)
            | ExprKind::HistoryField(_, _)
            | ExprKind::ChangeField(_, _) => {}
        }
    }
    Ok(needs)
}

#[derive(Default)]
struct HydrationGroupV2 {
    fields: BTreeMap<u32, FieldIdV2>,
    records: BTreeMap<RecordKeyV2, RecordRefV2>,
}

fn collect_row_hydration(
    row: &[V],
    needs: &[PropertyNeedV2<'_>],
    static_group: &mut HydrationGroupV2,
    groups: &mut BTreeMap<String, HydrationGroupV2>,
    budget: &mut ExecutionBudgetV2,
) -> Result<(), QueryErrorV2> {
    let empty = HydratedValuesV2::default();
    for need in needs {
        let entity = eval(need.entity, row, budget, &empty)?;
        let record = match entity {
            V::Null => continue,
            V::Entity(record) => record,
            _ => return Err(failure("bound_type")),
        };
        let identity_bytes = (record.database_id.len() as u64)
            .saturating_add(record.namespace.len() as u64)
            .saturating_add(record.id.len() as u64)
            .saturating_add(record.revision.len() as u64);
        if let Some(name) = need.field.name() {
            budget.reserve(
                (name.len() as u64)
                    .saturating_add(identity_bytes)
                    .saturating_add(128),
            )?;
            let record_key = RecordKeyV2::from(&record);
            static_group
                .fields
                .entry(need.field.index())
                .or_insert_with(|| need.field.clone());
            static_group.records.entry(record_key).or_insert(record);
        } else {
            let value = eval(need.name, row, budget, &empty)?;
            let V::Utf8(name) = value else {
                return Err(failure("bound_type"));
            };
            budget.reserve(
                (name.len() as u64)
                    .saturating_add(identity_bytes)
                    .saturating_add(128),
            )?;
            let record_key = RecordKeyV2::from(&record);
            let group = groups.entry(name.clone()).or_default();
            group
                .fields
                .entry(need.field.index())
                .or_insert_with(|| need.field.with_name(&name));
            group.records.entry(record_key).or_insert(record);
        }
    }
    Ok(())
}

fn add_static_property_records<'a>(
    properties: impl Iterator<Item = &'a BoundPatternProperty>,
    records: impl Iterator<Item = &'a RecordRefV2>,
    group: &mut HydrationGroupV2,
    budget: &mut ExecutionBudgetV2,
) -> Result<(), QueryErrorV2> {
    let properties = properties.collect::<Vec<_>>();
    if properties.is_empty() {
        return Ok(());
    }
    for property in &properties {
        budget.reserve(2 * size_of::<FieldIdV2>() as u64 + 64)?;
        group
            .fields
            .insert(property.value_field.index(), property.value_field.clone());
        group.fields.insert(
            property.has_property_field.index(),
            property.has_property_field.clone(),
        );
    }
    for record in records {
        budget.check()?;
        let identity_bytes = (record.database_id.len() as u64)
            .saturating_add(record.namespace.len() as u64)
            .saturating_add(record.id.len() as u64)
            .saturating_add(record.revision.len() as u64);
        budget.reserve(identity_bytes.saturating_add(size_of::<RecordRefV2>() as u64 + 128))?;
        group
            .records
            .entry(RecordKeyV2::from(record))
            .or_insert_with(|| record.clone());
    }
    Ok(())
}

fn add_kernel_static_hydration(
    kernel: &Kernel,
    graph: Option<&GraphSnapshotV2>,
    group: &mut HydrationGroupV2,
    budget: &mut ExecutionBudgetV2,
) -> Result<(), QueryErrorV2> {
    match kernel {
        Kernel::LexicalMatch {
            field,
            has_property,
            ..
        } => {
            let graph = graph.ok_or_else(|| failure("lexical_source_unavailable"))?;
            budget.reserve(2 * size_of::<FieldIdV2>() as u64 + 64)?;
            group.fields.insert(field.index(), field.clone());
            group
                .fields
                .insert(has_property.index(), has_property.clone());
            for record in graph.nodes.values() {
                budget.check()?;
                let identity_bytes = (record.database_id.len() as u64)
                    .saturating_add(record.namespace.len() as u64)
                    .saturating_add(record.id.len() as u64)
                    .saturating_add(record.revision.len() as u64);
                budget.reserve(
                    identity_bytes.saturating_add(size_of::<RecordRefV2>() as u64 + 128),
                )?;
                group
                    .records
                    .entry(RecordKeyV2::from(record))
                    .or_insert_with(|| record.clone());
            }
        }
        Kernel::MatchSequence { start, steps, .. }
        | Kernel::ExpandSequence { start, steps, .. } => {
            let node_properties = std::iter::once(&start.properties)
                .chain(steps.iter().map(|step| &step.node.properties))
                .flat_map(|properties| properties.values());
            let edge_properties = steps.iter().flat_map(|step| step.edge_properties.values());
            if node_properties.clone().next().is_some() || edge_properties.clone().next().is_some()
            {
                let graph = graph.ok_or_else(|| failure("graph_source_unavailable"))?;
                add_static_property_records(node_properties, graph.nodes.values(), group, budget)?;
                add_static_property_records(
                    edge_properties,
                    graph.edges.iter().map(|edge| &edge.edge),
                    group,
                    budget,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn hydrate_for_kernel(
    kernel: &Kernel,
    input: &[Row],
    right: Option<&[Row]>,
    graph: Option<&GraphSnapshotV2>,
    budget: &mut ExecutionBudgetV2,
    hydrate: &mut HydrateFnV2<'_>,
) -> Result<HydratedValuesV2, QueryErrorV2> {
    let roots = kernel_expression_roots(kernel, budget)?;
    let needs = property_needs(roots, budget)?;
    let mut groups = BTreeMap::new();
    let mut static_group = HydrationGroupV2::default();
    add_kernel_static_hydration(kernel, graph, &mut static_group, budget)?;
    if needs.is_empty() && static_group.fields.is_empty() {
        return Ok(HydratedValuesV2::default());
    }

    if matches!(kernel, Kernel::Join { .. }) {
        let right = right.ok_or_else(|| failure("join_input"))?;
        let width = input
            .first()
            .map_or(0, Vec::len)
            .saturating_add(right.first().map_or(0, Vec::len));
        budget.reserve(
            (width as u64)
                .saturating_mul(size_of::<V>() as u64)
                .saturating_add(size_of::<Row>() as u64),
        )?;
        let mut combined = Vec::with_capacity(width);
        for left in input {
            for right in right {
                budget.check()?;
                combined.clear();
                combined.extend_from_slice(left);
                combined.extend_from_slice(right);
                collect_row_hydration(&combined, &needs, &mut static_group, &mut groups, budget)?;
            }
        }
    } else {
        for row in input {
            collect_row_hydration(row, &needs, &mut static_group, &mut groups, budget)?;
        }
    }

    let mut hydrated = HydratedValuesV2::default();
    let batch_count = groups
        .len()
        .saturating_add(usize::from(!static_group.fields.is_empty()));
    budget.reserve((batch_count as u64).saturating_mul(size_of::<HydrationGroupV2>() as u64))?;
    let mut batches = Vec::with_capacity(batch_count);
    if !static_group.fields.is_empty() {
        batches.push(static_group);
    }
    batches.extend(groups.into_values());
    for group in batches {
        budget.check()?;
        let field_count = group.fields.len() as u64;
        let record_count = group.records.len() as u64;
        let identity_bytes = group
            .records
            .values()
            .map(|record| {
                (record.database_id.len() as u64)
                    .saturating_add(record.namespace.len() as u64)
                    .saturating_add(record.id.len() as u64)
                    .saturating_add(record.revision.len() as u64)
            })
            .fold(0u64, u64::saturating_add);
        let field_name_bytes = group
            .fields
            .values()
            .filter_map(FieldIdV2::name)
            .map(|name| name.len() as u64)
            .fold(0u64, u64::saturating_add);
        let cells = record_count.saturating_mul(field_count);
        budget.reserve(
            field_count
                .saturating_mul(size_of::<FieldIdV2>() as u64)
                .saturating_add(record_count.saturating_mul(size_of::<RecordRefV2>() as u64 + 128))
                .saturating_add(cells.saturating_mul(64))
                .saturating_add(identity_bytes.saturating_mul(field_count))
                .saturating_add(field_name_bytes.saturating_mul(record_count)),
        )?;
        let fields: Vec<_> = group.fields.into_values().collect();
        let records: Vec<_> = group.records.into_values().collect();
        let batch = hydrate(&records, &fields, budget)?;
        hydrated.add_batch(&records, &fields, batch)?;
    }
    Ok(hydrated)
}

fn aggregate(
    call: &Aggregate,
    rows: &[&Row],
    b: &mut ExecutionBudgetV2,
    hydrated: &HydratedValuesV2,
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
        let value = eval(arg, row, b, hydrated)?;
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
    let mut no_source = |_: &[RecordRefV2], _: &[FieldIdV2], _: &mut ExecutionBudgetV2| {
        Err(failure("source_hydration_missing"))
    };
    let mut no_annotation_lookup = |_: &RecordRefV2, _: &mut ExecutionBudgetV2| {
        Err(QueryErrorV2::new(
            "CAPABILITY_UNSUPPORTED",
            "execute",
            "annotation_lookup_unavailable",
        ))
    };
    execute_v2_with_source_adapters(
        plan,
        budget,
        &BTreeMap::new(),
        &mut no_annotation_lookup,
        &mut no_source,
    )
}

fn compact_expand_paths(
    row: &[V],
    start_index: usize,
    end_index: Option<usize>,
    pattern: &super::wire::CompactPattern,
    graph: &GraphSnapshotV2,
    budget: &mut ExecutionBudgetV2,
) -> Result<Vec<CompactPathV2>, QueryErrorV2> {
    let start = match row.get(start_index) {
        Some(V::Entity(record)) if graph.nodes.get(&record.id) == Some(record) => record.clone(),
        Some(V::Null) => return Ok(Vec::new()),
        Some(V::Entity(_)) => return Ok(Vec::new()),
        _ => return Err(failure("bound_type")),
    };
    let mut pending = vec![(vec![start.clone()], Vec::<RecordRefV2>::new())];
    let mut matches = Vec::new();
    while let Some((vertices, edges)) = pending.pop() {
        budget.reserve_expanded_nodes(1)?;
        let current = vertices.last().ok_or_else(|| failure("expand_path"))?;
        let hops = edges.len();
        if hops >= pattern.min_hops as usize {
            let endpoint_matches = match end_index {
                Some(index) => row.get(index) == Some(&V::Entity(current.clone())),
                None => true,
            };
            if endpoint_matches {
                budget.reserve(
                    (vertices.len() as u64)
                        .saturating_mul(size_of::<RecordRefV2>() as u64)
                        .saturating_add(
                            (edges.len() as u64).saturating_mul(size_of::<RecordRefV2>() as u64),
                        ),
                )?;
                matches.push((vertices.clone(), edges.clone()));
            }
        }
        if hops >= pattern.max_hops as usize {
            continue;
        }
        for edge in &graph.edges {
            budget.reserve_expanded_edges(1)?;
            if !pattern.relations.is_empty()
                && !pattern
                    .relations
                    .iter()
                    .any(|relation| relation == &edge.relation)
            {
                continue;
            }
            let next = match pattern.direction {
                Direction::Out if edge.source == *current => Some(&edge.target),
                Direction::In if edge.target == *current => Some(&edge.source),
                Direction::Both if edge.source == *current => Some(&edge.target),
                Direction::Both if edge.target == *current => Some(&edge.source),
                _ => None,
            };
            let Some(next) = next else {
                continue;
            };
            if pattern.mode == PathMode::Trail && edges.iter().any(|used| used.id == edge.edge.id) {
                continue;
            }
            if pattern.mode == PathMode::Simple && vertices.iter().any(|used| used.id == next.id) {
                continue;
            }
            budget.reserve(
                (vertices.len() as u64 + 1)
                    .saturating_mul(size_of::<RecordRefV2>() as u64)
                    .saturating_add(
                        (edges.len() as u64 + 1).saturating_mul(size_of::<RecordRefV2>() as u64),
                    ),
            )?;
            let mut next_vertices = vertices.clone();
            next_vertices.push(next.clone());
            let mut next_edges = edges.clone();
            next_edges.push(edge.edge.clone());
            pending.push((next_vertices, next_edges));
        }
    }
    matches.sort_by(|left, right| {
        left.1
            .len()
            .cmp(&right.1.len())
            .then_with(|| compare_record_refs(&left.1, &right.1))
            .then_with(|| compare_record_refs(&left.0, &right.0))
    });
    Ok(matches)
}

fn compare_record_refs(left: &[RecordRefV2], right: &[RecordRefV2]) -> Ordering {
    for (left, right) in left.iter().zip(right) {
        let order = left
            .database_id
            .cmp(&right.database_id)
            .then_with(|| left.namespace.cmp(&right.namespace))
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.revision.cmp(&right.revision));
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}

fn matches_sequence_node(
    constraint: &BoundSequenceNode,
    record: &RecordRefV2,
    input_row: &[V],
    graph: &GraphSnapshotV2,
    budget: &mut ExecutionBudgetV2,
    hydrated: &HydratedValuesV2,
) -> Result<bool, QueryErrorV2> {
    if constraint.id.is_none() && constraint.labels.is_empty() && constraint.properties.is_empty() {
        return Ok(true);
    }
    budget.reserve_expanded_nodes(1)?;
    if let Some(expression) = &constraint.id {
        let value = eval(expression, input_row, budget, hydrated)?;
        match value {
            V::Utf8(id) => {
                budget.reserve((record.id.len() as u64).saturating_add(id.len() as u64))?;
                if id != record.id {
                    return Ok(false);
                }
            }
            V::Null => return Ok(false),
            _ => return Err(failure("pattern_id_type")),
        }
    }
    if !constraint.labels.is_empty() {
        let labels = graph
            .node_labels
            .get(&record.id)
            .ok_or_else(|| failure("graph_node_labels"))?;
        for label in &constraint.labels {
            budget.reserve((label.len() as u64).saturating_add(16))?;
            if !labels.contains(label) {
                return Ok(false);
            }
        }
    }
    if !matches_pattern_properties(&constraint.properties, record, input_row, budget, hydrated)? {
        return Ok(false);
    }
    Ok(true)
}

struct SequencePathV2 {
    row: Vec<V>,
    vertices: Vec<RecordRefV2>,
    edges: Vec<RecordRefV2>,
    step_lengths: Vec<usize>,
}

// The path matcher consumes separate bound constraints and execution context.
#[allow(clippy::too_many_arguments)]
fn sequence_expand_paths(
    row: &[V],
    start_index: usize,
    start_constraint: &BoundSequenceNode,
    steps: &[BoundSequenceStep],
    mode: PathMode,
    graph: &GraphSnapshotV2,
    budget: &mut ExecutionBudgetV2,
    hydrated: &HydratedValuesV2,
) -> Result<Vec<SequencePathV2>, QueryErrorV2> {
    let start = match row.get(start_index) {
        Some(V::Entity(record)) if graph.nodes.get(&record.id) == Some(record) => record.clone(),
        Some(V::Null) | Some(V::Entity(_)) => return Ok(Vec::new()),
        _ => return Err(failure("bound_type")),
    };
    if !matches_sequence_node(start_constraint, &start, row, graph, budget, hydrated)? {
        return Ok(Vec::new());
    }
    budget.reserve(
        row_bytes(row)
            .saturating_add(size_of::<SequencePathV2>() as u64)
            .saturating_add(size_of::<RecordRefV2>() as u64)
            .saturating_add((steps.len() as u64).saturating_mul(size_of::<usize>() as u64)),
    )?;
    let mut paths = vec![SequencePathV2 {
        row: row.to_vec(),
        vertices: vec![start],
        edges: Vec::new(),
        step_lengths: Vec::with_capacity(steps.len()),
    }];
    for step in steps {
        let mut next_paths = Vec::new();
        for path in paths {
            budget.check()?;
            let current = path.vertices.last().ok_or_else(|| failure("expand_path"))?;
            budget
                .reserve((size_of::<Row>() as u64).saturating_add(entity_value_bytes(current)))?;
            let segment_row = vec![V::Entity(current.clone())];
            let segments =
                compact_expand_paths(&segment_row, 0, None, &step.pattern, graph, budget)?;
            for (segment_vertices, segment_edges) in segments {
                let mut edge_matches = true;
                for edge in &segment_edges {
                    if !matches_pattern_properties(
                        &step.edge_properties,
                        edge,
                        row,
                        budget,
                        hydrated,
                    )? {
                        edge_matches = false;
                        break;
                    }
                }
                if !edge_matches {
                    continue;
                }
                let endpoint = segment_vertices
                    .last()
                    .ok_or_else(|| failure("expand_path"))?;
                if mode == PathMode::Trail
                    && segment_edges.iter().enumerate().any(|(index, edge)| {
                        path.edges.iter().any(|used| used.id == edge.id)
                            || segment_edges[..index].iter().any(|used| used.id == edge.id)
                    })
                {
                    continue;
                }
                if mode == PathMode::Simple
                    && segment_vertices
                        .iter()
                        .skip(1)
                        .enumerate()
                        .any(|(index, vertex)| {
                            path.vertices.iter().any(|used| used.id == vertex.id)
                                || segment_vertices[1..index + 1]
                                    .iter()
                                    .any(|used| used.id == vertex.id)
                        })
                {
                    continue;
                }
                if step.end_index < path.row.len()
                    && !matches!(path.row.get(step.end_index), Some(V::Entity(bound)) if bound == endpoint)
                {
                    continue;
                }
                if step.end_index > path.row.len() {
                    return Err(failure("expand_sequence_binding"));
                }
                if !matches_sequence_node(&step.node, endpoint, row, graph, budget, hydrated)? {
                    continue;
                }
                let row_bytes_needed = row_bytes(&path.row)
                    .saturating_add(if step.end_index == path.row.len() {
                        entity_value_bytes(endpoint)
                    } else {
                        0
                    })
                    .saturating_add(step.edge_index.map_or(0, |_| {
                        if step.pattern.min_hops == 1 && step.pattern.max_hops == 1 {
                            segment_edges.first().map_or(0, entity_value_bytes)
                        } else {
                            size_of::<V>() as u64
                                + size_of::<Vec<V>>() as u64
                                + segment_edges
                                    .iter()
                                    .map(entity_value_bytes)
                                    .fold(0, u64::saturating_add)
                        }
                    }));
                budget.reserve(
                    row_bytes_needed
                        .saturating_add(
                            ((path.vertices.len() + segment_vertices.len() - 1) as u64)
                                .saturating_mul(size_of::<RecordRefV2>() as u64),
                        )
                        .saturating_add(
                            ((path.edges.len() + segment_edges.len()) as u64)
                                .saturating_mul(size_of::<RecordRefV2>() as u64),
                        )
                        .saturating_add(
                            ((path.step_lengths.len() + 1) as u64)
                                .saturating_mul(size_of::<usize>() as u64),
                        )
                        .saturating_add(size_of::<SequencePathV2>() as u64),
                )?;
                let mut expanded = path.row.clone();
                if step.end_index == expanded.len() {
                    expanded.push(V::Entity(endpoint.clone()));
                }
                if let Some(edge_index) = step.edge_index {
                    if edge_index != expanded.len() {
                        return Err(failure("expand_sequence_binding"));
                    }
                    let edge_value = if step.pattern.min_hops == 1 && step.pattern.max_hops == 1 {
                        V::Entity(
                            segment_edges
                                .first()
                                .ok_or_else(|| failure("expand_edge"))?
                                .clone(),
                        )
                    } else {
                        V::List(segment_edges.iter().cloned().map(V::Entity).collect())
                    };
                    expanded.push(edge_value);
                }
                let mut vertices = path.vertices.clone();
                vertices.extend(segment_vertices.into_iter().skip(1));
                let mut edges = path.edges.clone();
                edges.extend(segment_edges.iter().cloned());
                let mut step_lengths = path.step_lengths.clone();
                step_lengths.push(segment_edges.len());
                next_paths.push(SequencePathV2 {
                    row: expanded,
                    vertices,
                    edges,
                    step_lengths,
                });
            }
        }
        paths = next_paths;
        if paths.is_empty() {
            break;
        }
    }
    paths.sort_by(|left, right| {
        left.edges
            .len()
            .cmp(&right.edges.len())
            .then_with(|| compare_record_refs(&left.edges, &right.edges))
            .then_with(|| compare_record_refs(&left.vertices, &right.vertices))
            .then_with(|| left.step_lengths.cmp(&right.step_lengths))
    });
    Ok(paths)
}

fn unicode_whitespace_tokens(
    text: &str,
    budget: &mut ExecutionBudgetV2,
) -> Result<Vec<String>, QueryErrorV2> {
    let count = text
        .split(char::is_whitespace)
        .filter(|token| !token.is_empty())
        .count();
    budget.reserve(
        (text.len() as u64)
            .saturating_add((count as u64).saturating_mul(size_of::<String>() as u64 + 24)),
    )?;
    Ok(text
        .split(char::is_whitespace)
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect())
}

// Corpus inputs and query budget are separate validated plan values.
#[allow(clippy::too_many_arguments)]
fn lexical_match_rows(
    input: &[Row],
    graph: &GraphSnapshotV2,
    hydrated: &HydratedValuesV2,
    entity_index: usize,
    field: &FieldIdV2,
    has_property: &FieldIdV2,
    query: &str,
    k: u32,
    budget: &mut ExecutionBudgetV2,
) -> Result<Vec<Row>, QueryErrorV2> {
    let query_terms = unicode_whitespace_tokens(query, budget)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut corpus = BTreeMap::<RecordKeyV2, (RecordRefV2, Vec<String>)>::new();
    let mut total_length = 0u64;
    for record in graph.nodes.values() {
        budget.check()?;
        let present = hydrated
            .get(record, has_property, field.name().unwrap_or_default())
            .ok_or_else(|| failure("lexical_hydration"))?;
        if present != &V::Bool(true) {
            continue;
        }
        let value = hydrated
            .get(record, field, field.name().unwrap_or_default())
            .ok_or_else(|| failure("lexical_hydration"))?;
        let V::Json(Value::String(text)) = value else {
            continue;
        };
        let tokens = unicode_whitespace_tokens(text, budget)?;
        total_length = total_length.saturating_add(tokens.len() as u64);
        budget.reserve(
            (size_of::<(RecordRefV2, Vec<String>)>() + size_of::<RecordKeyV2>() + 64) as u64,
        )?;
        corpus.insert(RecordKeyV2::from(record), (record.clone(), tokens));
    }

    let corpus_size = corpus.len() as f64;
    let average_length = if corpus.is_empty() {
        0.0
    } else {
        total_length as f64 / corpus_size
    };
    let mut inverse_document_frequency = BTreeMap::new();
    for term in &query_terms {
        budget.reserve((term.len() as u64).saturating_add(64))?;
        let document_frequency = corpus
            .values()
            .filter(|(_, tokens)| tokens.iter().any(|token| token == term))
            .count() as f64;
        let idf =
            (1.0 + (corpus_size - document_frequency + 0.5) / (document_frequency + 0.5)).ln();
        inverse_document_frequency.insert(term, idf);
    }

    let mut ranked = Vec::new();
    budget.reserve((input.len() as u64).saturating_mul(64))?;
    for (row_index, row) in input.iter().enumerate() {
        budget.check()?;
        let record = match row.get(entity_index) {
            Some(V::Entity(record)) if record.kind == crate::uee_v2::RecordKindV2::Node => record,
            Some(V::Null) => continue,
            Some(V::Entity(_)) => {
                return Err(QueryErrorV2::new(
                    "CAPABILITY_UNSUPPORTED",
                    "execute",
                    "lexical_owner_kind",
                ))
            }
            _ => return Err(failure("lexical_owner")),
        };
        let Some((owner, tokens)) = corpus.get(&RecordKeyV2::from(record)) else {
            continue;
        };
        let mut score = 0.0;
        for (term, idf) in &inverse_document_frequency {
            let frequency = tokens.iter().filter(|token| *token == *term).count() as f64;
            if frequency == 0.0 {
                continue;
            }
            let length = 1.0 - 0.75 + 0.75 * (tokens.len() as f64 / average_length);
            let weight = frequency * (1.0 + 1.0 / 1.2) / (frequency / 1.2 + length);
            score += idf * weight;
        }
        if !score.is_finite() {
            return Err(failure("lexical_nonfinite_score"));
        }
        if score > 0.0 {
            budget.reserve((size_of::<ScoreValueV2>() + size_of::<RecordKeyV2>() + 24) as u64)?;
            ranked.push((
                row_index,
                RecordKeyV2::from(owner),
                ScoreValueV2 {
                    value: score,
                    owner: owner.clone(),
                    source: "lexical".into(),
                    scope: "whole_input".into(),
                    metric: super::catalog::LEXICAL_PROFILE_ID.into(),
                },
            ));
        }
    }
    ranked.sort_by(|left, right| {
        right
            .2
            .value
            .partial_cmp(&left.2.value)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.0.cmp(&right.0))
    });
    let mut out = Vec::new();
    for (row_index, _, score) in ranked.into_iter().take(k as usize) {
        push_with_value(&mut out, &input[row_index], V::Score(score), budget)?;
    }
    Ok(out)
}

fn context_evidence(
    source: &RecordRefV2,
    text: &str,
    start_scalar: u64,
    end_scalar: u64,
) -> ContextEvidenceV2 {
    ContextEvidenceV2 {
        source: source.clone(),
        source_hash: hex::encode(Sha256::digest(text.as_bytes())),
        start_scalar,
        end_scalar,
    }
}

// Context construction needs each bound expression and its output budget.
#[allow(clippy::too_many_arguments)]
fn context_pack_row(
    input: &[Row],
    text: &BoundExpr,
    evidence: &BoundExpr,
    text_entity: &BoundExpr,
    token_budget: u32,
    tokenizer_fingerprint: &str,
    budget: &mut ExecutionBudgetV2,
    hydrated: &HydratedValuesV2,
) -> Result<Vec<Row>, QueryErrorV2> {
    let mut package = ContextPackageV2 {
        rendered_context: String::new(),
        token_count: 0,
        token_budget: u64::from(token_budget),
        tokenizer_fingerprint: tokenizer_fingerprint.to_owned(),
        fragments: Vec::new(),
        omitted_refs: Vec::new(),
        truncated: false,
        truncation_reason: None,
    };
    let mut stopped = false;
    for row in input {
        budget.check()?;
        let source = match (
            eval(evidence, row, budget, hydrated)?,
            eval(text_entity, row, budget, hydrated)?,
        ) {
            (V::Entity(source), V::Entity(text_source)) if source == text_source => source,
            (V::Null, _) | (_, V::Null) => continue,
            (V::Entity(_), V::Entity(_)) => {
                return Err(QueryErrorV2::new(
                    "CAPABILITY_UNSUPPORTED",
                    "execute",
                    "context_evidence_mismatch",
                ))
            }
            _ => return Err(failure("context_evidence_type")),
        };
        let source_text = match eval(text, row, budget, hydrated)? {
            V::Null => continue,
            V::Json(Value::String(text)) => text,
            V::Json(_) => {
                return Err(QueryErrorV2::new(
                    "CAPABILITY_UNSUPPORTED",
                    "execute",
                    "context_text_type",
                ))
            }
            _ => return Err(failure("context_text_type")),
        };
        let total = source_text.chars().count() as u64;
        let citation = format!("[{}]", package.fragments.len() + 1);
        let separator = if package.fragments.is_empty() {
            ""
        } else {
            "\n"
        };
        let overhead = separator.chars().count() as u64 + citation.chars().count() as u64;
        let remaining = package.token_budget.saturating_sub(package.token_count);
        if stopped || remaining < overhead || (total > 0 && remaining == overhead) {
            budget.reserve(source_text.len() as u64 + 128)?;
            package
                .omitted_refs
                .push(context_evidence(&source, &source_text, 0, total));
            stopped = true;
            continue;
        }
        let kept = total.min(remaining - overhead);
        let text: String = source_text.chars().take(kept as usize).collect();
        let addition_bytes = separator.len() as u64
            + text.len() as u64
            + citation.len() as u64
            + source_text.len() as u64
            + source.id.len() as u64
            + source.revision.len() as u64
            + 256;
        budget.reserve(addition_bytes)?;
        package.rendered_context.push_str(separator);
        package.rendered_context.push_str(&text);
        package.rendered_context.push_str(&citation);
        package.token_count = package.rendered_context.chars().count() as u64;
        package.fragments.push(ContextFragmentV2 {
            text,
            citation,
            evidence: context_evidence(&source, &source_text, 0, kept),
        });
        if kept < total {
            package
                .omitted_refs
                .push(context_evidence(&source, &source_text, kept, total));
            stopped = true;
        }
    }
    package.truncated = !package.omitted_refs.is_empty();
    if package.truncated {
        package.truncation_reason = Some("TOKEN_BUDGET".into());
    }
    budget.reserve(
        package.rendered_context.len() as u64
            + package.tokenizer_fingerprint.len() as u64
            + (package.fragments.len() as u64 + package.omitted_refs.len() as u64)
                .saturating_mul(size_of::<ContextEvidenceV2>() as u64 + 128),
    )?;
    Ok(vec![vec![V::Context(package)]])
}

pub(crate) fn execute_v2_with_source_adapters(
    plan: &PhysicalPlanV2,
    budget: &mut ExecutionBudgetV2,
    sources: &BTreeMap<String, Vec<Vec<V>>>,
    annotation_lookup: &mut AnnotationLookupFnV2<'_>,
    hydrate: &mut HydrateFnV2<'_>,
) -> Result<ExecutionOutputV2, QueryErrorV2> {
    execute_v2_with_graph_adapter(plan, budget, sources, annotation_lookup, hydrate, None)
}

pub(crate) fn execute_v2_with_graph_adapter(
    plan: &PhysicalPlanV2,
    budget: &mut ExecutionBudgetV2,
    sources: &BTreeMap<String, Vec<Vec<V>>>,
    annotation_lookup: &mut AnnotationLookupFnV2<'_>,
    hydrate: &mut HydrateFnV2<'_>,
    graph: Option<&GraphSnapshotV2>,
) -> Result<ExecutionOutputV2, QueryErrorV2> {
    let mut no_vector_reader = |_: &[RecordRefV2],
                                _: &str,
                                _: bool,
                                _: &mut ExecutionBudgetV2|
     -> Result<VectorBatchV2, QueryErrorV2> {
        Err(QueryErrorV2::new(
            "CAPABILITY_UNSUPPORTED",
            "execute",
            "original_vector_unavailable",
        ))
    };
    execute_v2_with_vector_adapters(
        plan,
        budget,
        sources,
        annotation_lookup,
        hydrate,
        &mut no_vector_reader,
        graph,
    )
}

pub(crate) fn execute_v2_with_vector_adapters(
    plan: &PhysicalPlanV2,
    budget: &mut ExecutionBudgetV2,
    sources: &BTreeMap<String, Vec<Vec<V>>>,
    annotation_lookup: &mut AnnotationLookupFnV2<'_>,
    hydrate: &mut HydrateFnV2<'_>,
    vector_lookup: &mut VectorLookupFnV2<'_>,
    graph: Option<&GraphSnapshotV2>,
) -> Result<ExecutionOutputV2, QueryErrorV2> {
    budget.check()?;
    budget.reserve((plan.nodes().len() as u64).saturating_mul(1024))?;
    let mut outputs: Vec<Vec<Row>> = Vec::with_capacity(plan.nodes().len());
    let mut explain = plan.explain_nodes();
    for (index, node) in plan.nodes().iter().enumerate() {
        budget.check()?;
        let started = Instant::now();
        let expanded_nodes_before = budget.expanded_nodes_used;
        let expanded_edges_before = budget.expanded_edges_used;
        let distance_evaluations_before = budget.distance_evaluations_used;
        let mut out = Vec::new();
        let input = node
            .inputs
            .first()
            .map(|i| outputs[*i].as_slice())
            .unwrap_or(&[]);
        let right = node.inputs.get(1).map(|i| outputs[*i].as_slice());
        let hydrated = hydrate_for_kernel(&node.kernel, input, right, graph, budget, hydrate)?;
        let input_rows = node.inputs.iter().try_fold(0u64, |total, input_index| {
            u64::try_from(outputs[*input_index].len())
                .ok()
                .and_then(|count| total.checked_add(count))
        });
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
            Kernel::SourceScan(_) => {
                let rows = sources
                    .get(&node.id)
                    .ok_or_else(|| failure("source_batch_missing"))?;
                for row in rows {
                    push_clone(&mut out, row, budget)?;
                }
            }
            Kernel::MatchCompact {
                pattern,
                shortest,
                anchors,
            } => {
                let graph = graph.ok_or_else(|| {
                    QueryErrorV2::new(
                        "CAPABILITY_UNSUPPORTED",
                        "execute",
                        "graph_source_unavailable",
                    )
                })?;
                let mut endpoints = BTreeSet::new();
                for start in graph.nodes.values() {
                    budget.reserve(entity_value_bytes(start))?;
                    let row = [V::Entity(start.clone())];
                    let end_index = (pattern.start_alias == pattern.end_alias).then_some(0);
                    let paths = compact_expand_paths(&row, 0, end_index, pattern, graph, budget)?;
                    for (vertices, edges) in paths {
                        let endpoint = vertices.last().ok_or_else(|| failure("match_path"))?;
                        let endpoint_key = RecordKeyV2::from(endpoint);
                        if !matches_compact_match_anchors(
                            start,
                            endpoint,
                            pattern.start_alias == pattern.end_alias,
                            anchors,
                        )? {
                            continue;
                        }
                        let mut matched = vec![V::Entity(start.clone())];
                        if pattern.end_alias != pattern.start_alias {
                            matched.push(V::Entity(endpoint.clone()));
                        }
                        if pattern.edge_alias.is_some() {
                            if pattern.min_hops == 1 && pattern.max_hops == 1 {
                                matched.push(V::Entity(
                                    edges.first().ok_or_else(|| failure("match_edge"))?.clone(),
                                ));
                            } else {
                                matched
                                    .push(V::List(edges.iter().cloned().map(V::Entity).collect()));
                            }
                        }
                        if pattern.path_alias.is_some() {
                            let mode = match pattern.mode {
                                PathMode::Trail => QueryPathModeV2::Trail,
                                PathMode::Simple => QueryPathModeV2::Simple,
                                PathMode::Walk => QueryPathModeV2::Walk,
                            };
                            matched.push(V::Path(super::value::PathValueV2 {
                                vertices,
                                edges,
                                mode,
                            }));
                        }
                        if matched.len() != node.columns.len() {
                            return Err(failure("match_binding"));
                        }
                        if *shortest && !endpoints.insert((RecordKeyV2::from(start), endpoint_key))
                        {
                            continue;
                        }
                        budget.reserve(row_bytes(&matched))?;
                        out.push(matched);
                    }
                }
            }
            Kernel::MatchSequence {
                start: start_constraint,
                steps,
                mode,
                path_index,
                shortest,
                anchors,
            } => {
                let graph = graph.ok_or_else(|| {
                    QueryErrorV2::new(
                        "CAPABILITY_UNSUPPORTED",
                        "execute",
                        "graph_source_unavailable",
                    )
                })?;
                let mut endpoints = BTreeSet::new();
                for start_record in graph.nodes.values() {
                    budget.reserve(entity_value_bytes(start_record))?;
                    let row = [V::Entity(start_record.clone())];
                    for path in sequence_expand_paths(
                        &row,
                        0,
                        start_constraint,
                        steps,
                        *mode,
                        graph,
                        budget,
                        &hydrated,
                    )? {
                        if !matches_match_anchors(&path.row, anchors)? {
                            continue;
                        }
                        let first = path.vertices.first().ok_or_else(|| failure("match_path"))?;
                        let last = path.vertices.last().ok_or_else(|| failure("match_path"))?;
                        let endpoint_key = (RecordKeyV2::from(first), RecordKeyV2::from(last));
                        let mut matched = path.row;
                        if path_index.is_some() {
                            if *path_index != Some(matched.len()) {
                                return Err(failure("match_binding"));
                            }
                            let path_mode = match mode {
                                PathMode::Trail => QueryPathModeV2::Trail,
                                PathMode::Simple => QueryPathModeV2::Simple,
                                PathMode::Walk => QueryPathModeV2::Walk,
                            };
                            matched.push(V::Path(super::value::PathValueV2 {
                                vertices: path.vertices,
                                edges: path.edges,
                                mode: path_mode,
                            }));
                        }
                        if matched.len() != node.columns.len() {
                            return Err(failure("match_binding"));
                        }
                        if *shortest && !endpoints.insert(endpoint_key) {
                            continue;
                        }
                        budget.reserve(row_bytes(&matched))?;
                        out.push(matched);
                    }
                }
            }
            Kernel::AnnotationLookup {
                target_index,
                optional,
            } => {
                let mut matches_by_target = BTreeMap::new();
                for row in input {
                    budget.check()?;
                    let target = row
                        .get(*target_index)
                        .ok_or_else(|| failure("bound_field"))?;
                    let matches = match target {
                        V::Null => &[][..],
                        V::Entity(target) => {
                            let key_bytes = (target.database_id.len()
                                + target.namespace.len()
                                + target.id.len()
                                + target.revision.len())
                                as u64;
                            budget.reserve(
                                key_bytes
                                    .saturating_add(size_of::<RecordKeyV2>() as u64)
                                    .saturating_add(64),
                            )?;
                            let key = RecordKeyV2::from(target);
                            if !matches_by_target.contains_key(&key) {
                                let matches = annotation_lookup(target, budget)?;
                                budget.reserve(
                                    (matches.len() as u64)
                                        .saturating_mul(size_of::<RecordRefV2>() as u64),
                                )?;
                                for annotation in &matches {
                                    annotation.validate().map_err(|_| {
                                        QueryErrorV2::new(
                                            "DATA_CORRUPTION",
                                            "execute",
                                            "annotation_lookup_reference",
                                        )
                                    })?;
                                    if annotation.kind != crate::uee_v2::RecordKindV2::Annotation
                                        || annotation.database_id != target.database_id
                                        || annotation.namespace != target.namespace
                                    {
                                        return Err(QueryErrorV2::new(
                                            "DATA_CORRUPTION",
                                            "execute",
                                            "annotation_lookup_alignment",
                                        ));
                                    }
                                }
                                matches_by_target.insert(key.clone(), matches);
                            }
                            matches_by_target
                                .get(&key)
                                .ok_or_else(|| failure("annotation_lookup_batch"))?
                                .as_slice()
                        }
                        _ => return Err(failure("bound_type")),
                    };
                    if matches.is_empty() {
                        if *optional {
                            push_with_value(&mut out, row, V::Null, budget)?;
                        }
                    } else {
                        for annotation in matches {
                            push_with_value(&mut out, row, V::Entity(annotation.clone()), budget)?;
                        }
                    }
                }
            }
            Kernel::VectorRank {
                entity_index,
                query,
                collection,
                k,
                kind,
            } => {
                let candidate_count = u64::try_from(input.len()).map_err(|_| quota())?;
                budget.reserve_vector_candidates(candidate_count)?;
                budget
                    .reserve(candidate_count.saturating_mul(
                        (size_of::<RecordRefV2>() + size_of::<usize>() * 2) as u64,
                    ))?;
                let rerank = *kind == VectorRankKind::Rerank;
                let mut owners = Vec::with_capacity(input.len());
                let mut queries = Vec::with_capacity(input.len());
                for (row_index, row) in input.iter().enumerate() {
                    budget.check()?;
                    let owner = match row.get(*entity_index) {
                        Some(V::Entity(owner)) => owner,
                        Some(V::Null) if !rerank => continue,
                        Some(V::Null) => {
                            return Err(QueryErrorV2::new(
                                "EXACT_ORIGINAL_UNAVAILABLE",
                                "execute",
                                "candidate_owner_unavailable",
                            ));
                        }
                        _ => return Err(failure("vector_entity")),
                    };
                    if owner.kind != crate::uee_v2::RecordKindV2::Node {
                        return Err(QueryErrorV2::new(
                            "CAPABILITY_UNSUPPORTED",
                            "execute",
                            "vector_owner_kind_unavailable",
                        ));
                    }
                    let query_value = eval(query, row, budget, &hydrated)?;
                    let V::Vector(query_value) = query_value else {
                        return Err(failure("vector_query"));
                    };
                    if query_value.space_id != collection.space_id
                        || query_value.values.len() != usize::from(collection.dimension)
                    {
                        return Err(QueryErrorV2::new(
                            "COLLECTION_SPACE_MISMATCH",
                            "execute",
                            "vector_space",
                        ));
                    }
                    owners.push((row_index, owner.clone()));
                    queries.push(query_value);
                }
                let records = owners
                    .iter()
                    .map(|(_, owner)| owner.clone())
                    .collect::<Vec<_>>();
                let vectors = vector_lookup(&records, &collection.name, rerank, budget)?;
                if vectors.entries.len() != records.len() {
                    return Err(QueryErrorV2::new(
                        "DATA_CORRUPTION",
                        "execute",
                        "vector_batch_alignment",
                    ));
                }
                let mut ranked = Vec::new();
                for (((row_index, owner), query_value), candidate) in
                    owners.into_iter().zip(queries).zip(vectors.entries)
                {
                    budget.check()?;
                    let Some(candidate) = candidate else {
                        if rerank {
                            return Err(QueryErrorV2::new(
                                "EXACT_ORIGINAL_UNAVAILABLE",
                                "execute",
                                "candidate_vector_unavailable",
                            ));
                        }
                        continue;
                    };
                    budget.reserve_distance_evaluations(1)?;
                    let distance =
                        exact_vector_distance(&query_value, &candidate, &collection.metric)?;
                    budget.reserve(
                        (size_of::<usize>() + size_of::<ScoreValueV2>() + size_of::<RecordKeyV2>())
                            as u64,
                    )?;
                    ranked.push((
                        row_index,
                        RecordKeyV2::from(&owner),
                        ScoreValueV2 {
                            value: distance,
                            owner,
                            source: if rerank {
                                "original_rerank".into()
                            } else {
                                "exact_vector_scan".into()
                            },
                            scope: if rerank {
                                "rerank_candidates".into()
                            } else {
                                "whole_input".into()
                            },
                            metric: collection.metric.clone(),
                        },
                    ));
                }
                ranked.sort_by(|left, right| {
                    left.2
                        .value
                        .partial_cmp(&right.2.value)
                        .unwrap_or(Ordering::Equal)
                        .then_with(|| left.1.cmp(&right.1))
                        .then_with(|| left.0.cmp(&right.0))
                });
                for (row_index, _, score) in ranked.into_iter().take(*k as usize) {
                    push_with_value(&mut out, &input[row_index], V::Score(score), budget)?;
                }
            }
            Kernel::LexicalMatch {
                entity_index,
                field,
                has_property,
                query,
                k,
                alias_index,
            } => {
                if input.iter().any(|row| row.len() != *alias_index) {
                    return Err(failure("lexical_input_schema"));
                }
                let graph = graph.ok_or_else(|| {
                    QueryErrorV2::new(
                        "CAPABILITY_UNSUPPORTED",
                        "execute",
                        "lexical_source_unavailable",
                    )
                })?;
                out = lexical_match_rows(
                    input,
                    graph,
                    &hydrated,
                    *entity_index,
                    field,
                    has_property,
                    query,
                    *k,
                    budget,
                )?;
                if out.iter().any(|row| row.len() != node.columns.len()) {
                    return Err(failure("lexical_output_schema"));
                }
            }
            Kernel::ContextPack {
                text,
                evidence,
                text_entity,
                tokens,
                tokenizer_fingerprint,
            } => {
                if node.columns.len() != 1 {
                    return Err(failure("context_output_schema"));
                }
                out = context_pack_row(
                    input,
                    text,
                    evidence,
                    text_entity,
                    *tokens,
                    tokenizer_fingerprint,
                    budget,
                    &hydrated,
                )?;
            }
            Kernel::Expand {
                start_index,
                end_index,
                pattern,
                optional,
            } => {
                let graph = graph.ok_or_else(|| {
                    QueryErrorV2::new(
                        "CAPABILITY_UNSUPPORTED",
                        "execute",
                        "graph_source_unavailable",
                    )
                })?;
                for row in input {
                    budget.check()?;
                    let paths = compact_expand_paths(
                        row,
                        *start_index,
                        *end_index,
                        pattern,
                        graph,
                        budget,
                    )?;
                    if paths.is_empty() {
                        if *optional {
                            let mut expanded = row.clone();
                            if end_index.is_none() {
                                expanded.push(V::Null);
                            }
                            if pattern.edge_alias.is_some() {
                                expanded.push(V::Null);
                            }
                            if pattern.path_alias.is_some() {
                                expanded.push(V::Null);
                            }
                            budget.reserve(row_bytes(&expanded))?;
                            out.push(expanded);
                        }
                        continue;
                    }
                    for (vertices, edges) in paths {
                        let endpoint = vertices.last().ok_or_else(|| failure("expand_path"))?;
                        let mut expanded = row.clone();
                        if end_index.is_none() {
                            expanded.push(V::Entity(endpoint.clone()));
                        }
                        if pattern.edge_alias.is_some() {
                            if pattern.min_hops == 1 && pattern.max_hops == 1 {
                                let edge = edges.first().ok_or_else(|| failure("expand_edge"))?;
                                expanded.push(V::Entity(edge.clone()));
                            } else {
                                expanded
                                    .push(V::List(edges.iter().cloned().map(V::Entity).collect()));
                            }
                        }
                        if pattern.path_alias.is_some() {
                            let mode = match pattern.mode {
                                PathMode::Trail => QueryPathModeV2::Trail,
                                PathMode::Simple => QueryPathModeV2::Simple,
                                PathMode::Walk => QueryPathModeV2::Walk,
                            };
                            expanded.push(V::Path(super::value::PathValueV2 {
                                vertices,
                                edges,
                                mode,
                            }));
                        }
                        budget.reserve(row_bytes(&expanded))?;
                        out.push(expanded);
                    }
                }
            }
            Kernel::ExpandSequence {
                start_index,
                start,
                steps,
                mode,
                path_index,
                optional,
            } => {
                let graph = graph.ok_or_else(|| {
                    QueryErrorV2::new(
                        "CAPABILITY_UNSUPPORTED",
                        "execute",
                        "graph_source_unavailable",
                    )
                })?;
                for row in input {
                    budget.check()?;
                    let paths = sequence_expand_paths(
                        row,
                        *start_index,
                        start,
                        steps,
                        *mode,
                        graph,
                        budget,
                        &hydrated,
                    )?;
                    if paths.is_empty() {
                        if *optional {
                            let mut expanded = row.clone();
                            expanded.resize(node.columns.len(), V::Null);
                            budget.reserve(row_bytes(&expanded))?;
                            out.push(expanded);
                        }
                        continue;
                    }
                    for path in paths {
                        let mut expanded = path.row;
                        if let Some(path_index) = path_index {
                            if *path_index != expanded.len() {
                                return Err(failure("expand_sequence_binding"));
                            }
                            let mode = match mode {
                                PathMode::Trail => QueryPathModeV2::Trail,
                                PathMode::Simple => QueryPathModeV2::Simple,
                                PathMode::Walk => QueryPathModeV2::Walk,
                            };
                            expanded.push(V::Path(super::value::PathValueV2 {
                                vertices: path.vertices,
                                edges: path.edges,
                                mode,
                            }));
                        }
                        if expanded.len() != node.columns.len() {
                            return Err(failure("expand_sequence_binding"));
                        }
                        budget.reserve(row_bytes(&expanded))?;
                        out.push(expanded);
                    }
                }
            }
            Kernel::Filter(predicate) => {
                for row in input {
                    if eval(predicate, row, budget, &hydrated)? == V::Bool(true) {
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
                        projected.push(eval(expr, row, budget, &hydrated)?);
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
                        .map(|(e, _, _)| eval(e, row, budget, &hydrated))
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
                let right = right.ok_or_else(|| failure("join_input"))?;
                for left in input {
                    let mut matched = false;
                    for r in right {
                        budget.reserve(row_bytes(left).saturating_add(row_bytes(r)))?;
                        let mut combined = left.clone();
                        combined.extend_from_slice(r);
                        if eval(condition, &combined, budget, &hydrated)? == V::Bool(true) {
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
                        .map(|e| eval(e, row, budget, &hydrated))
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
                        key.push(aggregate(call, &rows, budget, &hydrated)?);
                    }
                    out.push(key);
                }
            }
        }
        budget.check()?;
        let graph_operator = matches!(
            &node.kernel,
            Kernel::Expand { .. }
                | Kernel::ExpandSequence { .. }
                | Kernel::MatchCompact { .. }
                | Kernel::MatchSequence { .. }
        );
        let distance_evaluations = if matches!(&node.kernel, Kernel::VectorRank { .. }) {
            CounterReadingV2::measured_or_overflow(
                CounterUnitV2::DistanceEvaluations,
                budget
                    .distance_evaluations_used
                    .checked_sub(distance_evaluations_before),
            )
        } else {
            CounterReadingV2::unknown(
                CounterUnitV2::DistanceEvaluations,
                CounterUnknownReasonV2::NotApplicable,
            )
        };
        let graph_expansions = if graph_operator {
            let expanded_nodes = budget
                .expanded_nodes_used
                .checked_sub(expanded_nodes_before);
            let expanded_edges = budget
                .expanded_edges_used
                .checked_sub(expanded_edges_before);
            CounterReadingV2::measured_or_overflow(
                CounterUnitV2::GraphExpansions,
                expanded_nodes
                    .and_then(|nodes| expanded_edges.and_then(|edges| nodes.checked_add(edges))),
            )
        } else {
            CounterReadingV2::unknown(
                CounterUnitV2::GraphExpansions,
                CounterUnknownReasonV2::NotApplicable,
            )
        };
        let elapsed_ns = if matches!(&node.kernel, Kernel::SourceScan(_)) {
            CounterReadingV2::unknown(
                CounterUnitV2::Nanoseconds,
                CounterUnknownReasonV2::NotInstrumented,
            )
        } else {
            CounterReadingV2::measured_or_overflow(
                CounterUnitV2::Nanoseconds,
                u64::try_from(started.elapsed().as_nanos()).ok(),
            )
        };
        explain[index].actual = Some(ActualCountersV2 {
            rows_in: CounterReadingV2::measured_or_overflow(CounterUnitV2::Rows, input_rows),
            rows_out: CounterReadingV2::measured_or_overflow(
                CounterUnitV2::Rows,
                u64::try_from(out.len()).ok(),
            ),
            bytes_read: CounterReadingV2::unknown(
                CounterUnitV2::Bytes,
                CounterUnknownReasonV2::NotInstrumented,
            ),
            work_units: CounterReadingV2::unknown(
                CounterUnitV2::WorkUnits,
                CounterUnknownReasonV2::NotInstrumented,
            ),
            index_probes: CounterReadingV2::unknown(
                CounterUnitV2::IndexProbes,
                CounterUnknownReasonV2::NotApplicable,
            ),
            distance_evaluations,
            graph_expansions,
            memory_peak_bytes: CounterReadingV2::unknown(
                CounterUnitV2::Bytes,
                CounterUnknownReasonV2::NotInstrumented,
            ),
            spill_bytes: CounterReadingV2::unknown(
                CounterUnitV2::Bytes,
                CounterUnknownReasonV2::NotApplicable,
            ),
            elapsed_ns,
            sampling: CounterSamplingV2::Complete,
            clock_source: CounterClockSourceV2::MonotonicInstant,
            elapsed_scope: CounterElapsedScopeV2::OperatorExecution,
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

#[cfg(test)]
mod vector_distance_tests {
    use super::{exact_vector_distance, VectorValueV2};

    fn vector(values: &[f64]) -> VectorValueV2 {
        VectorValueV2 {
            space_id: "test-space".into(),
            scalar: super::super::value::VectorScalarV2::F64,
            values: values.to_vec(),
        }
    }

    #[test]
    fn cosine_distance_scales_finite_extreme_and_subnormal_values() {
        assert_eq!(
            exact_vector_distance(
                &vector(&[1e308, 1e308]),
                &vector(&[1e308, -1e308]),
                "Cosine",
            )
            .unwrap(),
            1.0
        );
        assert_eq!(
            exact_vector_distance(&vector(&[5e-324, 0.0]), &vector(&[-5e-324, 0.0]), "Cosine",)
                .unwrap(),
            2.0
        );
    }
}
