//! Narrow, actor-scoped lowering for legacy HQL accepted by the v2 boundary.

use super::{error::QueryErrorV2, exec::ExecutionBudgetV2};
use crate::{
    query::ast::{HqlCommand, HqlField, HqlOp, HqlValue, PatternDirection, PatternReturn},
    uee_v2::{IndexPolicyV2, QueryFormatV2, QueryRequestV2},
};

fn unsupported() -> QueryErrorV2 {
    QueryErrorV2::new("CAPABILITY_UNSUPPORTED", "bind", "legacy_lowering")
}

fn is_plain_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Lower the differential-tested HQL1 subsets into the shared HQL2 source pipeline.
/// The caller must authorize the request before invoking the legacy parser.
pub(crate) fn lower_hql_v1(
    request: &QueryRequestV2,
    actor_namespace: &str,
    budget: &mut ExecutionBudgetV2,
) -> Result<(String, String), QueryErrorV2> {
    if actor_namespace != "default"
        || request.namespace != "default"
        || !request.params.is_empty()
        || request.temporal.is_some()
        || request.transaction_id.is_some()
        || request.budget.is_some()
        || request.required_frontier.is_some()
        || request.explain.is_some()
        || request
            .index_policy
            .as_ref()
            .is_some_and(|policy| !matches!(policy, IndexPolicyV2::MergeDelta))
        || request.allow_partial == Some(true)
        || request
            .format
            .as_ref()
            .is_some_and(|format| !matches!(format, QueryFormatV2::Json))
    {
        return Err(unsupported());
    }

    let source = request
        .hql
        .as_deref()
        .ok_or_else(|| QueryErrorV2::new("BIND_ERROR", "bind", "source_shape"))?;
    // Bound Pest's legacy pair queue before parsing; query_v2 separately
    // reserves the canonical HQL2 parser after this lowering step.
    budget.reserve(super::required_heap_bytes(source)?)?;
    let command = HqlCommand::try_from(source)
        .map_err(|_| QueryErrorV2::new("HQL_PARSE_ERROR", "parse", "legacy_syntax"))?;
    let HqlCommand::MatchPattern {
        pattern,
        as_of,
        clauses,
    } = command
    else {
        return Err(unsupported());
    };
    let Some(alias) = pattern.start.var.as_deref() else {
        return Err(unsupported());
    };
    if as_of.is_some()
        || !pattern.start.props.is_empty()
        || clauses.where_preds.len() > 1
        || clauses.limit.is_some()
    {
        return Err(unsupported());
    }
    let fields = match clauses.ret {
        Some(PatternReturn::Fields(fields)) => fields,
        _ => return Err(unsupported()),
    };
    if fields.len() != 1 {
        return Err(unsupported());
    }
    let projection = &fields[0];
    if projection.field.as_ref() != Some(&HqlField::Id) {
        return Err(unsupported());
    }

    let source = match pattern.hops.as_slice() {
        [] if projection.var == alias
            && clauses.where_preds.is_empty()
            && clauses.order_by.is_none() =>
        {
            let label = match pattern.start.label.as_deref() {
                Some(label) if is_plain_identifier(label) => format!(" {label}"),
                Some(_) => return Err(unsupported()),
                None => String::new(),
            };
            format!("USE default FROM NODES{label} AS __hql1_node |> RETURN __hql1_node.id AS id")
        }
        [(edge, end)] => {
            if pattern.start.label.is_some()
                || !end.props.is_empty()
                || end.label.is_some()
                || edge.var.is_some()
            {
                return Err(unsupported());
            }
            let Some(end_alias) = end.var.as_deref() else {
                return Err(unsupported());
            };
            let edge_source = match (edge.direction, edge.rel_type.as_deref()) {
                (PatternDirection::Out, Some(relation)) if is_plain_identifier(relation) => {
                    format!("-[:{relation}]->")
                }
                (PatternDirection::In, Some(relation)) if is_plain_identifier(relation) => {
                    format!("<-[:{relation}]-")
                }
                (PatternDirection::Both, Some(relation)) if is_plain_identifier(relation) => {
                    format!("-[:{relation}]-")
                }
                (PatternDirection::Out, None) => "-->".into(),
                (PatternDirection::In, None) => "<--".into(),
                (PatternDirection::Both, None) => "--".into(),
                _ => return Err(unsupported()),
            };
            if end_alias == alias || (projection.var != alias && projection.var != end_alias) {
                return Err(unsupported());
            }
            let filter = match clauses.where_preds.as_slice() {
                [] => String::new(),
                [predicate]
                    if predicate.op == HqlOp::Eq
                        && predicate.field.field.as_ref() == Some(&HqlField::Id) =>
                {
                    let filter_alias = if predicate.field.var == alias {
                        "__hql1_source"
                    } else if predicate.field.var == end_alias {
                        "__hql1_target"
                    } else {
                        return Err(unsupported());
                    };
                    let HqlValue::Str(value) = &predicate.value else {
                        return Err(unsupported());
                    };
                    let value = serde_json::to_string(value).map_err(|_| unsupported())?;
                    format!(" |> FILTER {filter_alias}.id = {value}")
                }
                _ => return Err(unsupported()),
            };
            let projected_alias = if projection.var == alias {
                "__hql1_source"
            } else {
                "__hql1_target"
            };
            let order = match clauses.order_by.as_ref() {
                Some((field, descending))
                    if field.var == projection.var
                        && field.field.as_ref() == Some(&HqlField::Id) =>
                {
                    format!(
                        " |> ORDER BY {projected_alias}.id {}",
                        if *descending { "DESC" } else { "ASC" }
                    )
                }
                Some(_) => return Err(unsupported()),
                None => String::new(),
            };
            format!(
                "USE default MATCH (__hql1_source){edge_source}(__hql1_target) AS __hql1_path WALK{filter}{order} |> RETURN {projected_alias}.id AS id"
            )
        }
        _ => return Err(unsupported()),
    };

    Ok((source, projection.output_key()))
}
