//! Envelope policies checked before catalog binding or generation publication.
use super::{
    ast::{Hql2Statement, StatementKind, UnsignedKind},
    error::QueryErrorV2,
};
use crate::uee_v2::{
    ExplainV2, HqlLanguageVersionV2, IndexPolicyV2, QueryFormatV2, QueryRequestV2,
};
use chrono::{DateTime, Utc};

pub(crate) struct ReadOptionsV2 {
    pub(crate) mode: ExplainV2,
    pub(crate) valid_at: DateTime<Utc>,
    pub(crate) tx: Option<u64>,
}

fn bind_error(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("BIND_ERROR", "bind", reason)
}
fn unsupported(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("CAPABILITY_UNSUPPORTED", "bind", reason)
}
/// Borrow-only envelope checks. IR shape is checked once by the closed decoder;
/// G0's general validator clones IDs and must not run ahead of reservations.
pub(crate) fn validate_envelope(request: &QueryRequestV2) -> Result<(), QueryErrorV2> {
    if request.contract_version != "genesis.api.v2"
        || request
            .ir
            .as_ref()
            .is_some_and(|ir| ir.contract_version != "query-ir.v2")
    {
        return Err(QueryErrorV2::new(
            "VERSION_UNSUPPORTED",
            "contract",
            "contract_version",
        ));
    }
    if !(1..=256).contains(&request.request_id.chars().count()) {
        return Err(bind_error("request_id"));
    }
    let namespace = request.namespace.as_bytes();
    if !(1..=63).contains(&namespace.len())
        || !namespace[0].is_ascii_lowercase()
        || !namespace
            .iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
    {
        return Err(bind_error("namespace"));
    }
    match (&request.hql, &request.ir, &request.language_version) {
        (Some(hql), None, Some(_)) if (1..=262_144).contains(&hql.chars().count()) => {}
        (None, Some(_), None) => {}
        _ => return Err(bind_error("source_shape")),
    }
    Ok(())
}
pub(crate) fn decimal(value: &str) -> Result<u64, QueryErrorV2> {
    let number: u64 = value.parse().map_err(|_| bind_error("frontier_range"))?;
    if number.to_string() != value {
        return Err(bind_error("frontier_encoding"));
    }
    Ok(number)
}

pub(crate) fn normalize(
    request: &QueryRequestV2,
    statement: Option<&Hql2Statement>,
    frontier: u64,
    horizon: u64,
    now: DateTime<Utc>,
) -> Result<ReadOptionsV2, QueryErrorV2> {
    if request.contract_version != "genesis.api.v2"
        || request
            .ir
            .as_ref()
            .is_some_and(|ir| ir.contract_version != "query-ir.v2")
    {
        return Err(QueryErrorV2::new(
            "VERSION_UNSUPPORTED",
            "contract",
            "contract_version",
        ));
    }
    validate_envelope(request)?;
    if request.language_version == Some(HqlLanguageVersionV2::HqlV1) {
        return Err(unsupported("legacy_lowering"));
    }
    if request.allow_partial == Some(true) {
        return Err(unsupported("partial_results"));
    }
    if request.format == Some(QueryFormatV2::Stream) {
        return Err(unsupported("stream_format"));
    }
    if matches!(
        request.index_policy,
        Some(IndexPolicyV2::Wait | IndexPolicyV2::Eventual)
    ) {
        return Err(unsupported("index_policy"));
    }
    if request.transaction_id.is_some() {
        return Err(bind_error("read_transaction_id"));
    }
    if let Some(required) = &request.required_frontier {
        if decimal(required)? > frontier {
            return Err(QueryErrorV2::new(
                "INDEX_COVERAGE_TIMEOUT",
                "bind",
                "required_frontier",
            ));
        }
    }
    let textual_mode = statement
        .map(|s| match s.kind {
            StatementKind::Read => Ok(None),
            StatementKind::Explain => Ok(Some(ExplainV2::Plan)),
            StatementKind::AnalyzeRead => Ok(Some(ExplainV2::Analyze)),
            _ => Err(unsupported("statement_execution")),
        })
        .transpose()?
        .flatten();
    if request
        .explain
        .as_ref()
        .zip(textual_mode.as_ref())
        .is_some_and(|(a, b)| a != b)
    {
        return Err(bind_error("explain_mode_conflict"));
    }
    let mode = request
        .explain
        .clone()
        .or(textual_mode)
        .unwrap_or(ExplainV2::None);
    let mut tx = request
        .temporal
        .as_ref()
        .and_then(|t| t.tx_as_of.as_deref())
        .map(decimal)
        .transpose()?;
    let mut valid_at = request.temporal.as_ref().and_then(|t| t.valid_at);
    if let Some(scope) = statement
        .and_then(|s| s.query.as_ref())
        .and_then(|q| q.scope.as_ref())
    {
        if scope.namespace.value != request.namespace {
            return Err(bind_error("namespace_conflict"));
        }
        if let Some(selected) = &scope.tx {
            let UnsignedKind::Literal(selected) = selected.value else {
                return Err(unsupported("temporal_parameter"));
            };
            if tx.is_some_and(|t| t != selected) {
                return Err(bind_error("temporal_conflict"));
            }
            tx = Some(selected);
        }
        if let Some(selected) = &scope.valid_at {
            let selected = DateTime::parse_from_rfc3339(&selected.value)
                .map_err(|_| bind_error("valid_time"))?
                .with_timezone(&Utc);
            if valid_at.is_some_and(|t| t != selected) {
                return Err(bind_error("temporal_conflict"));
            }
            valid_at = Some(selected);
        }
    }
    if let Some(tx) = tx {
        if tx < horizon {
            return Err(QueryErrorV2::new(
                "BEYOND_HORIZON",
                "bind",
                "retention_horizon",
            ));
        }
        if tx > frontier {
            return Err(bind_error("future_snapshot"));
        }
    }
    Ok(ReadOptionsV2 {
        mode,
        valid_at: valid_at.unwrap_or(now),
        tx,
    })
}

/// Bound input size without allocating an encoded copy. The caller reserves this
/// conservative footprint before typed decoding and binding clone any values.
pub(crate) fn input_bytes(request: &QueryRequestV2) -> Result<u64, QueryErrorV2> {
    let mut bytes = 4096u64;
    let limit = request
        .budget
        .as_ref()
        .and_then(|b| b.max_memory_bytes)
        .unwrap_or(64 * 1024 * 1024)
        .min(64 * 1024 * 1024);
    let mut add = |n: usize| -> Result<(), QueryErrorV2> {
        bytes = bytes
            .checked_add(n as u64)
            .ok_or_else(|| bind_error("input_size"))?;
        if bytes > limit {
            return Err(QueryErrorV2::new(
                "QUERY_BUDGET_EXCEEDED",
                "execute",
                "input_memory",
            ));
        }
        Ok(())
    };
    add(request.params.len().saturating_mul(128))?;
    add(request.request_id.len())?;
    add(request.namespace.len())?;
    if let Some(frontier) = &request.required_frontier {
        add(frontier.len())?;
    }
    if let Some(tx) = request.temporal.as_ref().and_then(|t| t.tx_as_of.as_ref()) {
        add(tx.len())?;
    }
    if let Some(hql) = &request.hql {
        add(hql.len().saturating_mul(16))?;
    }
    let mut stack = Vec::new();
    for (name, value) in &request.params {
        add(name.len())?;
        stack.push((value, 0usize));
    }
    if let Some(ir) = &request.ir {
        if ir.nodes.len() > 10_000 {
            return Err(bind_error("plan_nodes"));
        }
        for (name, ty) in &ir.parameter_types {
            add(name.len() + ty.len() + 128)?;
        }
        for node in &ir.nodes {
            add(node.id.len().saturating_mul(16).saturating_add(256))?;
            for input in &node.inputs {
                add(input.len().saturating_mul(16).saturating_add(32))?;
            }
            for (key, value) in &node.config {
                add(key.len() + 32)?;
                stack.push((value, 0));
            }
        }
    }
    while let Some((value, depth)) = stack.pop() {
        if depth > 512 {
            return Err(bind_error("value_depth"));
        }
        add(128)?;
        match value {
            serde_json::Value::String(v) => add(v.len().saturating_mul(4))?,
            serde_json::Value::Array(v) => {
                add(v.len().saturating_mul(128))?;
                stack.extend(v.iter().map(|v| (v, depth + 1)));
            }
            serde_json::Value::Object(v) => {
                add(v.len().saturating_mul(128))?;
                for (k, v) in v {
                    add(k.len().saturating_mul(4))?;
                    stack.push((v, depth + 1));
                }
            }
            _ => {}
        }
    }
    Ok(bytes)
}

/// QueryRequestV2 is a public Rust input and can contain JSON deeper than a
/// JSON parser accepts. Refusing it must not recurse during destruction.
pub(crate) fn discard_request(mut request: QueryRequestV2) {
    fn discard(value: serde_json::Value) {
        let mut pending = vec![value];
        while let Some(value) = pending.pop() {
            match value {
                serde_json::Value::Array(values) => pending.extend(values),
                serde_json::Value::Object(values) => pending.extend(values.into_values()),
                _ => {}
            }
        }
    }
    for value in std::mem::take(&mut request.params).into_values() {
        discard(value);
    }
    if let Some(ir) = request.ir.take() {
        for node in ir.nodes {
            for value in node.config.into_values() {
                discard(value);
            }
        }
    }
}

/// Reserve all possible propagated schema copies before the binder allocates
/// them. A node can expose only aliases declared somewhere in this closed DAG;
/// six copies cover bound scope, temporary scope, plan, explain and result headers.
pub(crate) fn planning_bytes(
    logical: &super::wire::LogicalRequestV2,
    request: &QueryRequestV2,
) -> u64 {
    use super::wire::{Config, Expr};
    let mut alias_bytes = 0u64;
    let mut aliases = 0u64;
    let mut wrappers = 0u64;
    let mut type_len = logical
        .parameter_types
        .values()
        .map(|s| s.len() as u64)
        .max()
        .unwrap_or(16);
    type_len = type_len.max(
        request
            .params
            .values()
            .filter_map(|v| v.get("type")?.as_str())
            .map(|s| s.len() as u64)
            .max()
            .unwrap_or(16),
    );
    let mut expressions = Vec::new();
    let mut expression_count = 0u64;
    for node in &logical.nodes {
        let mut alias = |name: &str| {
            aliases = aliases.saturating_add(1);
            alias_bytes = alias_bytes.saturating_add(name.len() as u64);
        };
        match &node.config {
            Config::Values { alias: name, .. } => alias(name),
            Config::Project { fields } => {
                for field in fields {
                    alias(&field.alias);
                    expressions.push(&field.expression);
                }
            }
            Config::Aggregate {
                group_by,
                aggregates,
            } => {
                for field in group_by.iter().chain(aggregates) {
                    alias(&field.alias);
                    expressions.push(&field.expression);
                }
                wrappers = wrappers.saturating_add(16);
            }
            Config::Filter { predicate } => expressions.push(predicate),
            Config::Sort { keys } => expressions.extend(keys.iter().map(|k| &k.expression)),
            Config::Join { condition, .. } => {
                expressions.push(condition);
                wrappers = wrappers.saturating_add(10);
            }
            _ => {}
        }
    }
    while let Some(expression) = expressions.pop() {
        expression_count = expression_count.saturating_add(1);
        match expression {
            Expr::Literal { ty, .. } => type_len = type_len.max(ty.len() as u64),
            Expr::Binary { left, right, .. } => expressions.extend([left.as_ref(), right.as_ref()]),
            Expr::Unary { arg, .. } => expressions.push(arg),
            Expr::Call { args, .. } => expressions.extend(args),
            Expr::In {
                expression, values, ..
            } => {
                expressions.push(expression);
                expressions.extend(values);
            }
            _ => {}
        }
    }
    let type_storage = type_len
        .saturating_add(wrappers)
        .saturating_mul(8)
        .saturating_add(512);
    alias_bytes
        .saturating_add(aliases.saturating_mul(type_storage))
        .saturating_mul(logical.nodes.len() as u64)
        .saturating_mul(6)
        .saturating_add((logical.nodes.len() as u64).saturating_mul(2048))
        .saturating_add(
            expression_count
                .saturating_mul(type_storage)
                .saturating_mul(3),
        )
}
