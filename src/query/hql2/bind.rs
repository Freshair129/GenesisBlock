//! Semantic scalar binding. Only this module constructs trusted bound nodes.
use super::{
    catalog::{
        is_registered_analyzer, is_registered_tokenizer, text_profile_fingerprints,
        AuthorizedCatalogV2, CollectionSpaceV2,
    },
    error::QueryErrorV2,
    source::{BoundSourceV2, FieldIdV2},
    value::{QueryTypeV2 as Ty, QueryValueV2 as Val},
    wire::{
        self, BinaryOp, Config, Expr, JoinKind, LogicalRequestV2, NullOrder, SortDirection, UnaryOp,
    },
};
use crate::uee_v2::{QueryOpV2, RecordRefV2};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

fn invalid(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("BIND_ERROR", "bind", reason)
}
fn unsupported() -> QueryErrorV2 {
    QueryErrorV2::new("CAPABILITY_UNSUPPORTED", "bind", "operator_unavailable")
}

#[derive(Clone, Debug)]
pub(crate) struct BoundParametersV2 {
    values: BTreeMap<String, (Ty, Arc<Val>)>,
}
impl BoundParametersV2 {
    pub(crate) fn decode(
        params: &BTreeMap<String, serde_json::Value>,
        parameter_types: Option<&BTreeMap<String, String>>,
    ) -> Result<Self, QueryErrorV2> {
        let mut values = BTreeMap::new();
        for (name, value) in params {
            let object = value
                .as_object()
                .ok_or_else(|| invalid("typed_parameter"))?;
            if object.get("type").and_then(|value| value.as_str()) == Some("vector") {
                if object.len() != 3 {
                    return Err(invalid("typed_parameter"));
                }
                let ty = if let Some(declared) = parameter_types.and_then(|types| types.get(name)) {
                    Ty::parse(declared)?
                } else if parameter_types.is_none() {
                    let space_id = object
                        .get("space_id")
                        .and_then(serde_json::Value::as_str)
                        .filter(|space_id| !space_id.is_empty())
                        .ok_or_else(|| invalid("vector_type_unresolved"))?;
                    let dimension = object
                        .get("values")
                        .and_then(serde_json::Value::as_array)
                        .and_then(|values| u16::try_from(values.len()).ok())
                        .filter(|dimension| *dimension > 0)
                        .ok_or_else(|| invalid("vector_type_unresolved"))?;
                    Ty::Vector {
                        space_id: space_id.to_owned(),
                        dimension,
                        scalar: super::value::VectorScalarV2::F64,
                    }
                } else {
                    return Err(invalid("vector_type_unresolved"));
                };
                let Ty::Vector { space_id, .. } = &ty else {
                    return Err(invalid("parameter_type_mismatch"));
                };
                if object.get("space_id").and_then(|value| value.as_str())
                    != Some(space_id.as_str())
                {
                    return Err(QueryErrorV2::new(
                        "COLLECTION_SPACE_MISMATCH",
                        "bind",
                        "vector_space",
                    ));
                }
                let vector = object
                    .get("values")
                    .ok_or_else(|| invalid("typed_parameter"))?;
                let value = ty.decode(vector)?;
                values.insert(name.clone(), (ty, Arc::new(value)));
                continue;
            }
            if object.len() != 2 || !object.contains_key("value") {
                return Err(invalid("typed_parameter"));
            }
            let ty = Ty::parse(
                object
                    .get("type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| invalid("typed_parameter"))?,
            )?;
            if ty == Ty::Null || contains_json(&ty) {
                return Err(QueryErrorV2::new(
                    "CAPABILITY_UNSUPPORTED",
                    "bind",
                    "json_parameter_unavailable",
                ));
            }
            if ty != Ty::Entity && contains_entity_type(&ty) {
                return Err(QueryErrorV2::new(
                    "CAPABILITY_UNSUPPORTED",
                    "bind",
                    "entity_parameter_scope",
                ));
            }
            let value = ty.decode(&object["value"])?;
            values.insert(name.clone(), (ty, Arc::new(value)));
        }
        Ok(Self { values })
    }
    fn get(&self, name: &str) -> Result<&(Ty, Arc<Val>), QueryErrorV2> {
        self.values
            .get(name)
            .ok_or_else(|| invalid("undeclared_parameter"))
    }

    pub(crate) fn decimal_u64(&self, name: &str) -> Result<u64, QueryErrorV2> {
        let (ty, value) = self.get(name)?;
        if ty != &Ty::DecimalU64 {
            return Err(invalid("parameter_type_mismatch"));
        }
        match value.as_ref() {
            Val::DecimalU64(value) => Ok(*value),
            _ => Err(invalid("parameter_type_mismatch")),
        }
    }
}

fn contains_entity_type(ty: &Ty) -> bool {
    match ty {
        Ty::Entity => true,
        Ty::Nullable(inner) | Ty::List(inner) => contains_entity_type(inner),
        _ => false,
    }
}

fn contains_json(ty: &Ty) -> bool {
    match ty {
        Ty::Json => true,
        Ty::Nullable(inner) | Ty::List(inner) => contains_json(inner),
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Column {
    pub name: String,
    pub ty: Ty,
}
#[derive(Clone, Debug)]
pub(super) struct BoundExpr {
    pub ty: Ty,
    pub kind: ExprKind,
}
#[derive(Clone, Debug)]
pub(super) enum ExprKind {
    Literal(Arc<Val>),
    Field(usize),
    EntityRefField(usize, String),
    ScoreField(usize, String),
    HistoryField(usize, String),
    ChangeField(usize, String),
    Property {
        entity: Box<BoundExpr>,
        name: Box<BoundExpr>,
        has_property: bool,
        field: FieldIdV2,
    },
    Binary(BinaryOp, Box<BoundExpr>, Box<BoundExpr>),
    Unary(UnaryOp, Box<BoundExpr>),
    Lower(Box<BoundExpr>),
    Length(Box<BoundExpr>),
    In(Box<BoundExpr>, Vec<BoundExpr>, bool),
}
#[derive(Clone, Debug)]
pub(super) enum AggregateKind {
    CountAll,
    Count,
    Sum,
    Avg,
    Min,
    Max,
    Collect,
}
#[derive(Clone, Debug)]
pub(super) struct Aggregate {
    pub kind: AggregateKind,
    pub arg: Option<BoundExpr>,
}
#[derive(Clone, Debug)]
pub(super) enum Kernel {
    Values(Arc<Val>),
    SourceScan(BoundSourceV2),
    MatchCompact {
        pattern: wire::CompactPattern,
        shortest: bool,
        anchors: Vec<BoundMatchAnchor>,
    },
    MatchSequence {
        start: BoundSequenceNode,
        steps: Vec<BoundSequenceStep>,
        mode: wire::PathMode,
        path_index: Option<usize>,
        shortest: bool,
        anchors: Vec<BoundMatchAnchor>,
    },
    AnnotationLookup {
        target_index: usize,
        optional: bool,
    },
    VectorRank {
        entity_index: usize,
        query: BoundExpr,
        collection: CollectionSpaceV2,
        k: u32,
        kind: VectorRankKind,
    },
    LexicalMatch {
        entity_index: usize,
        field: FieldIdV2,
        has_property: FieldIdV2,
        query: String,
        k: u32,
        alias_index: usize,
    },
    ContextPack {
        text: BoundExpr,
        evidence: BoundExpr,
        text_entity: BoundExpr,
        tokens: u32,
        tokenizer_fingerprint: String,
    },
    Expand {
        start_index: usize,
        end_index: Option<usize>,
        pattern: wire::CompactPattern,
        optional: bool,
    },
    ExpandSequence {
        start_index: usize,
        start: BoundSequenceNode,
        steps: Vec<BoundSequenceStep>,
        mode: wire::PathMode,
        path_index: Option<usize>,
        optional: bool,
    },
    Filter(BoundExpr),
    Project(Vec<BoundExpr>),
    Distinct,
    Sort(Vec<(BoundExpr, SortDirection, NullOrder)>),
    Take(u64),
    Offset(u64),
    UnionAll,
    Aggregate {
        groups: Vec<BoundExpr>,
        aggregates: Vec<Aggregate>,
    },
    Join {
        kind: JoinKind,
        condition: BoundExpr,
        right_width: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VectorRankKind {
    Knn,
    Rerank,
}
#[derive(Clone, Debug)]
pub(super) struct BoundSequenceStep {
    pub end_index: usize,
    pub edge_index: Option<usize>,
    pub pattern: wire::CompactPattern,
    pub node: BoundSequenceNode,
    pub edge_properties: BTreeMap<String, BoundPatternProperty>,
}
#[derive(Clone, Debug)]
pub(super) struct BoundSequenceNode {
    pub id: Option<BoundExpr>,
    pub labels: Vec<String>,
    pub properties: BTreeMap<String, BoundPatternProperty>,
}
#[derive(Clone, Debug)]
pub(super) struct BoundPatternProperty {
    pub expected: BoundExpr,
    pub value_field: FieldIdV2,
    pub has_property_field: FieldIdV2,
}
#[derive(Clone, Debug)]
pub(super) struct BoundMatchAnchor {
    pub column_index: usize,
    pub record: RecordRefV2,
}
#[derive(Clone, Debug)]
pub(super) struct BoundNode {
    pub id: String,
    pub op: QueryOpV2,
    pub inputs: Vec<usize>,
    pub columns: Vec<Column>,
    pub kernel: Kernel,
}
#[derive(Debug)]
pub(crate) struct BoundQueryV2 {
    nodes: Vec<BoundNode>,
    root: usize,
}
impl BoundQueryV2 {
    pub(super) fn into_parts(self) -> (Vec<BoundNode>, usize) {
        (self.nodes, self.root)
    }
}

fn ordered(ty: &Ty) -> bool {
    matches!(
        ty.base(),
        Ty::Bool | Ty::I64 | Ty::DecimalU64 | Ty::F64Finite | Ty::Utf8
    )
}
fn equality_comparable(ty: &Ty) -> bool {
    ordered(ty) || ty.base() == &Ty::Json
}
fn numeric(ty: &Ty) -> bool {
    matches!(ty.base(), Ty::I64 | Ty::F64Finite)
}
fn boolean(ty: &Ty) -> Result<(), QueryErrorV2> {
    if ty.base() == &Ty::Bool {
        Ok(())
    } else {
        Err(invalid("boolean_type"))
    }
}
fn nullable(ty: Ty, yes: bool) -> Ty {
    if yes {
        ty.as_nullable()
    } else {
        ty
    }
}

fn contextual_null(mut null: BoundExpr, context: &Ty) -> Result<BoundExpr, QueryErrorV2> {
    if null.ty != Ty::Null || context.base() == &Ty::Null {
        return Err(invalid("ambiguous_null_literal"));
    }
    null.ty = context.as_nullable();
    Ok(null)
}

#[derive(Default)]
struct FieldIdRegistryV2 {
    next: u32,
}

impl FieldIdRegistryV2 {
    fn issue(
        &mut self,
        has_property: bool,
        bound_name: Option<&str>,
    ) -> Result<FieldIdV2, QueryErrorV2> {
        let index = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| invalid("field_count"))?;
        let field = FieldIdV2::new(index, has_property);
        Ok(match bound_name {
            Some(name) => field.with_name(name),
            None => field,
        })
    }
}

fn bind_expr(
    expr: &Expr,
    scope: &[Column],
    params: &BoundParametersV2,
    field_ids: &mut FieldIdRegistryV2,
    depth: usize,
) -> Result<BoundExpr, QueryErrorV2> {
    let mut stack = vec![(expr, depth, false)];
    let mut ready = BTreeMap::new();
    while let Some((current, depth, visited)) = stack.pop() {
        if depth >= 128 {
            return Err(invalid("expression_depth"));
        }
        if visited {
            let bound = bind_expr_node(current, scope, params, field_ids, &mut ready)?;
            ready.insert(current as *const Expr as usize, bound);
            continue;
        }
        stack.push((current, depth, true));
        match current {
            Expr::Binary { left, right, .. } => {
                stack.push((right, depth + 1, false));
                stack.push((left, depth + 1, false));
            }
            Expr::Unary { arg, .. } => stack.push((arg, depth + 1, false)),
            Expr::Call { args, .. } => {
                stack.extend(args.iter().rev().map(|e| (e, depth + 1, false)))
            }
            Expr::In {
                expression, values, ..
            } => {
                stack.extend(values.iter().rev().map(|e| (e, depth + 1, false)));
                stack.push((expression, depth + 1, false));
            }
            _ => {}
        }
    }
    ready
        .remove(&(expr as *const Expr as usize))
        .ok_or_else(|| invalid("bound_expression"))
}

fn bind_expr_node(
    expr: &Expr,
    scope: &[Column],
    params: &BoundParametersV2,
    field_ids: &mut FieldIdRegistryV2,
    ready: &mut BTreeMap<usize, BoundExpr>,
) -> Result<BoundExpr, QueryErrorV2> {
    // Keys identify borrowed syntax nodes only during this call; never persisted
    // or dereferenced. Taking children avoids deep cloning of bound trees.
    let mut next = |e: &Expr| {
        ready
            .remove(&(e as *const Expr as usize))
            .ok_or_else(|| invalid("bound_expression"))
    };
    let (ty, kind) = match expr {
        Expr::Literal { literal, ty } => {
            let ty = Ty::parse(ty)?;
            if ty == Ty::Null {
                if !literal.is_null() {
                    return Err(invalid("null_literal"));
                }
                return Ok(BoundExpr {
                    ty,
                    kind: ExprKind::Literal(Arc::new(Val::Null)),
                });
            }
            if ty == Ty::Entity {
                return Err(QueryErrorV2::new(
                    "CAPABILITY_UNSUPPORTED",
                    "bind",
                    "entity_literal_scope",
                ));
            }
            let v = ty.decode(literal)?;
            (ty, ExprKind::Literal(Arc::new(v)))
        }
        Expr::Param { param } => {
            let (ty, value) = params.get(param)?;
            if *ty == Ty::Entity {
                return Err(QueryErrorV2::new(
                    "CAPABILITY_UNSUPPORTED",
                    "bind",
                    "entity_parameter_scope",
                ));
            }
            (ty.clone(), ExprKind::Literal(value.clone()))
        }
        Expr::Field { field } => {
            let (index, column) = scope
                .iter()
                .enumerate()
                .find(|(_, c)| c.name == field.alias)
                .ok_or_else(|| invalid("unknown_field"))?;
            if field.path.is_empty() {
                (column.ty.clone(), ExprKind::Field(index))
            } else if column.ty.base() == &Ty::Entity
                && field.path.len() == 1
                && matches!(
                    field.path[0].as_str(),
                    "database_id" | "namespace" | "kind" | "id" | "revision"
                )
            {
                (
                    nullable(Ty::Utf8, column.ty.nullable()),
                    ExprKind::EntityRefField(index, field.path[0].clone()),
                )
            } else if column.ty.base() == &Ty::Score && field.path.len() == 1 {
                let ty = match field.path[0].as_str() {
                    "value" | "distance" => Ty::F64Finite,
                    "owner" => Ty::Entity,
                    "source" | "scope" | "metric" => Ty::Utf8,
                    _ => return Err(invalid("score_field")),
                };
                (ty, ExprKind::ScoreField(index, field.path[0].clone()))
            } else if column.ty.base() == &Ty::HistoryRevision && field.path.len() == 1 {
                let ty = match field.path[0].as_str() {
                    "subject" => Ty::Entity,
                    "operation" | "valid_from" => Ty::Utf8,
                    "tx_from" => Ty::DecimalU64,
                    "tx_to" => Ty::DecimalU64.as_nullable(),
                    "valid_to" => Ty::Utf8.as_nullable(),
                    "id" | "kind" | "revision_id" => Ty::Utf8,
                    _ => return Err(invalid("history_field")),
                };
                (ty, ExprKind::HistoryField(index, field.path[0].clone()))
            } else if column.ty.base() == &Ty::ChangeEvent && field.path.len() == 1 {
                let ty = match field.path[0].as_str() {
                    "sequence" => Ty::DecimalU64,
                    "operation" => Ty::Utf8,
                    "subject" => Ty::Entity,
                    _ => return Err(invalid("change_field")),
                };
                (ty, ExprKind::ChangeField(index, field.path[0].clone()))
            } else {
                return Err(invalid("scalar_field_path"));
            }
        }
        Expr::Binary {
            binary,
            left,
            right,
        } => {
            let mut left = next(left)?;
            let mut right = next(right)?;
            if left.ty == Ty::Null && right.ty != Ty::Null {
                left = contextual_null(left, &right.ty)?;
            } else if right.ty == Ty::Null && left.ty != Ty::Null {
                right = contextual_null(right, &left.ty)?;
            }
            let null = left.ty.nullable() || right.ty.nullable();
            use BinaryOp::*;
            let ty = match binary {
                And | Or => {
                    boolean(&left.ty)?;
                    boolean(&right.ty)?;
                    nullable(Ty::Bool, null)
                }
                Add | Sub | Mul | Div | Rem => {
                    if !numeric(&left.ty) || left.ty.base() != right.ty.base() {
                        return Err(invalid("numeric_type"));
                    }
                    nullable(left.ty.base().clone(), null)
                }
                Eq | Ne => {
                    if left.ty == Ty::Null || right.ty == Ty::Null {
                        return Err(invalid("ambiguous_null_literal"));
                    }
                    if left.ty.base() != right.ty.base() {
                        return Err(invalid("comparison_type"));
                    }
                    if !equality_comparable(&left.ty) {
                        return Err(QueryErrorV2::new(
                            "CAPABILITY_UNSUPPORTED",
                            "bind",
                            "equality_unavailable",
                        ));
                    }
                    nullable(Ty::Bool, null)
                }
                Lt | Le | Gt | Ge => {
                    if !ordered(&left.ty) || left.ty.base() != right.ty.base() {
                        return Err(invalid("comparator_unavailable"));
                    }
                    nullable(Ty::Bool, null)
                }
                Contains | Startswith => {
                    if left.ty.base() != &Ty::Utf8 || right.ty.base() != &Ty::Utf8 {
                        return Err(invalid("string_type"));
                    }
                    nullable(Ty::Bool, null)
                }
            };
            (
                ty,
                ExprKind::Binary(*binary, Box::new(left), Box::new(right)),
            )
        }
        Expr::Unary { unary, arg } => {
            let arg = next(arg)?;
            if matches!(unary, UnaryOp::IsNull | UnaryOp::IsNotNull) && arg.ty == Ty::Null {
                return Err(invalid("ambiguous_null_literal"));
            }
            let ty = match unary {
                UnaryOp::Not => {
                    boolean(&arg.ty)?;
                    arg.ty.clone()
                }
                UnaryOp::Neg => {
                    if !numeric(&arg.ty) {
                        return Err(invalid("numeric_type"));
                    }
                    arg.ty.clone()
                }
                UnaryOp::IsNull | UnaryOp::IsNotNull => Ty::Bool,
            };
            (ty, ExprKind::Unary(*unary, Box::new(arg)))
        }
        Expr::Call { call, args } => {
            if matches!(call.as_str(), "prop" | "has_prop") {
                if args.len() != 2 {
                    return Err(invalid("function_arity"));
                }
                let entity = next(&args[0])?;
                let name = next(&args[1])?;
                if entity.ty.base() != &Ty::Entity || name.ty.base() != &Ty::Utf8 {
                    return Err(invalid("property_type"));
                }
                let has_property = call == "has_prop";
                let bound_name = match &name.kind {
                    ExprKind::Literal(value) => match value.as_ref() {
                        Val::Utf8(value) => Some(value.as_str()),
                        _ => None,
                    },
                    _ => None,
                };
                let field = field_ids.issue(has_property, bound_name)?;
                return Ok(BoundExpr {
                    ty: if has_property {
                        Ty::Bool
                    } else {
                        Ty::Nullable(Box::new(Ty::Json))
                    },
                    kind: ExprKind::Property {
                        entity: Box::new(entity),
                        name: Box::new(name),
                        has_property,
                        field,
                    },
                });
            }
            if !matches!(call.as_str(), "lower" | "length") {
                return Err(invalid("unknown_function"));
            }
            if args.len() != 1 {
                return Err(invalid("function_arity"));
            }
            let arg = next(&args[0])?;
            if arg.ty.base() != &Ty::Utf8 {
                return Err(invalid("function_type"));
            }
            let ty = nullable(
                if call == "lower" { Ty::Utf8 } else { Ty::I64 },
                arg.ty.nullable(),
            );
            (
                ty,
                if call == "lower" {
                    ExprKind::Lower(Box::new(arg))
                } else {
                    ExprKind::Length(Box::new(arg))
                },
            )
        }
        Expr::In {
            expression,
            values,
            negated,
        } => {
            let needle = next(expression)?;
            let mut values = values.iter().map(next).collect::<Result<Vec<_>, _>>()?;
            if needle.ty == Ty::Null {
                return Err(invalid("ambiguous_null_literal"));
            }
            for value in &mut values {
                if value.ty == Ty::Null {
                    *value = contextual_null(value.clone(), &needle.ty)?;
                }
            }
            if !equality_comparable(&needle.ty) {
                return Err(QueryErrorV2::new(
                    "CAPABILITY_UNSUPPORTED",
                    "bind",
                    "equality_unavailable",
                ));
            }
            if values.iter().any(|v| v.ty.base() != needle.ty.base()) {
                return Err(invalid("comparison_type"));
            }
            let ty = nullable(
                Ty::Bool,
                needle.ty.nullable() || values.iter().any(|v| v.ty.nullable()),
            );
            (ty, ExprKind::In(Box::new(needle), values, *negated))
        }
    };
    Ok(BoundExpr { ty, kind })
}

fn bind_named(
    fields: &[wire::NamedExpr],
    scope: &[Column],
    params: &BoundParametersV2,
    field_ids: &mut FieldIdRegistryV2,
) -> Result<(Vec<Column>, Vec<BoundExpr>), QueryErrorV2> {
    let mut columns = Vec::new();
    let mut exprs = Vec::new();
    for f in fields {
        let e = bind_expr(&f.expression, scope, params, field_ids, 0)?;
        if e.ty == Ty::Null {
            return Err(invalid("ambiguous_null_literal"));
        }
        columns.push(Column {
            name: f.alias.clone(),
            ty: e.ty.clone(),
        });
        exprs.push(e);
    }
    unique(&columns)?;
    Ok((columns, exprs))
}
fn unique(columns: &[Column]) -> Result<(), QueryErrorV2> {
    let mut seen = BTreeSet::new();
    if columns.iter().any(|c| !seen.insert(&c.name)) {
        Err(invalid("duplicate_alias"))
    } else {
        Ok(())
    }
}

fn bind_sequence_node(
    node: &wire::PatternNode,
    scope: &[Column],
    params: &BoundParametersV2,
    field_ids: &mut FieldIdRegistryV2,
    allow_input_field: bool,
) -> Result<BoundSequenceNode, QueryErrorV2> {
    let id = node
        .id
        .as_ref()
        .map(|expression| {
            match expression {
                wire::Expr::Literal { .. } | wire::Expr::Param { .. } => {}
                wire::Expr::Field { .. } if allow_input_field => {}
                _ => return Err(invalid("pattern_id_expression")),
            }
            let bound = bind_expr(expression, scope, params, field_ids, 0)?;
            if bound.ty != Ty::Utf8 {
                return Err(invalid("pattern_id_type"));
            }
            Ok(bound)
        })
        .transpose()?;
    Ok(BoundSequenceNode {
        id,
        labels: node
            .labels
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        properties: bind_pattern_properties(&node.properties, scope, params, field_ids)?,
    })
}

fn bind_pattern_properties(
    properties: &BTreeMap<String, wire::Expr>,
    scope: &[Column],
    params: &BoundParametersV2,
    field_ids: &mut FieldIdRegistryV2,
) -> Result<BTreeMap<String, BoundPatternProperty>, QueryErrorV2> {
    properties
        .iter()
        .map(|(name, expression)| {
            let expected = bind_expr(expression, scope, params, field_ids, 0)?;
            if expected.ty != Ty::Json {
                return Err(invalid("pattern_property_type"));
            }
            let value_field = field_ids.issue(false, Some(name))?;
            let has_property_field = field_ids.issue(true, Some(name))?;
            Ok((
                name.clone(),
                BoundPatternProperty {
                    expected,
                    value_field,
                    has_property_field,
                },
            ))
        })
        .collect()
}

fn bind_sequence(
    pattern: wire::SequencePattern,
    mut columns: Vec<Column>,
    optional: bool,
    params: &BoundParametersV2,
    field_ids: &mut FieldIdRegistryV2,
    constraint_scope: &[Column],
) -> Result<(Vec<Column>, Kernel), QueryErrorV2> {
    let start_constraint =
        bind_sequence_node(&pattern.start, constraint_scope, params, field_ids, true)?;
    let start_index = columns
        .iter()
        .position(|column| column.name == pattern.start.alias)
        .ok_or_else(|| invalid("expand_start"))?;
    if columns[start_index].ty.base() != &Ty::Entity {
        return Err(invalid("expand_start_type"));
    }
    let mut aliases = columns
        .iter()
        .enumerate()
        .map(|(index, column)| (column.name.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut steps = Vec::with_capacity(pattern.steps.len());
    let mut previous_alias = pattern.start.alias;
    for step in pattern.steps {
        let node_constraint =
            bind_sequence_node(&step.node, constraint_scope, params, field_ids, true)?;
        let end_index = if let Some(index) = aliases.get(&step.node.alias).copied() {
            if columns[index].ty.base() != &Ty::Entity {
                return Err(invalid("expand_end_type"));
            }
            index
        } else {
            let index = columns.len();
            aliases.insert(step.node.alias.clone(), index);
            columns.push(Column {
                name: step.node.alias.clone(),
                ty: nullable(Ty::Entity, optional),
            });
            index
        };
        let edge_index = if let Some(alias) = &step.edge.alias {
            if aliases.contains_key(alias) {
                return Err(invalid("duplicate_alias"));
            }
            let index = columns.len();
            aliases.insert(alias.clone(), index);
            let ty = if step.edge.min_hops == 1 && step.edge.max_hops == 1 {
                Ty::Entity
            } else {
                Ty::List(Box::new(Ty::Entity))
            };
            columns.push(Column {
                name: alias.clone(),
                ty: nullable(ty, optional),
            });
            Some(index)
        } else {
            None
        };
        steps.push(BoundSequenceStep {
            end_index,
            edge_index,
            pattern: wire::CompactPattern {
                start_alias: previous_alias,
                end_alias: step.node.alias.clone(),
                edge_alias: step.edge.alias,
                relations: step.edge.relations,
                direction: step.edge.direction,
                min_hops: step.edge.min_hops,
                max_hops: step.edge.max_hops,
                mode: wire::PathMode::Walk,
                path_alias: None,
            },
            node: node_constraint,
            edge_properties: bind_pattern_properties(
                &step.edge.properties,
                constraint_scope,
                params,
                field_ids,
            )?,
        });
        previous_alias = step.node.alias;
    }
    let path_index = if let Some(alias) = pattern.path_alias {
        if aliases.contains_key(&alias) {
            return Err(invalid("duplicate_alias"));
        }
        let index = columns.len();
        columns.push(Column {
            name: alias,
            ty: nullable(Ty::Path, optional),
        });
        Some(index)
    } else {
        None
    };
    unique(&columns)?;
    Ok((
        columns,
        Kernel::ExpandSequence {
            start_index,
            start: start_constraint,
            steps,
            mode: pattern.mode,
            path_index,
            optional,
        },
    ))
}

fn bind_match(
    pattern: wire::Pattern,
    anchors: BTreeMap<String, wire::Expr>,
    shortest: bool,
    params: &BoundParametersV2,
    field_ids: &mut FieldIdRegistryV2,
) -> Result<(Vec<Column>, Kernel), QueryErrorV2> {
    let (columns, kernel, node_aliases) = match pattern {
        wire::Pattern::Compact(pattern) => {
            let node_aliases =
                BTreeSet::from([pattern.start_alias.clone(), pattern.end_alias.clone()]);
            let mut columns = vec![Column {
                name: pattern.start_alias.clone(),
                ty: Ty::Entity,
            }];
            if pattern.end_alias != pattern.start_alias {
                columns.push(Column {
                    name: pattern.end_alias.clone(),
                    ty: Ty::Entity,
                });
            }
            if let Some(alias) = &pattern.edge_alias {
                let ty = if pattern.min_hops == 1 && pattern.max_hops == 1 {
                    Ty::Entity
                } else {
                    Ty::List(Box::new(Ty::Entity))
                };
                columns.push(Column {
                    name: alias.clone(),
                    ty,
                });
            }
            if let Some(alias) = &pattern.path_alias {
                columns.push(Column {
                    name: alias.clone(),
                    ty: Ty::Path,
                });
            }
            unique(&columns)?;
            (
                columns,
                Kernel::MatchCompact {
                    pattern,
                    shortest,
                    anchors: Vec::new(),
                },
                node_aliases,
            )
        }
        wire::Pattern::Sequence(pattern) => {
            let start_alias = pattern.start.alias.clone();
            let node_aliases = std::iter::once(start_alias.clone())
                .chain(pattern.steps.iter().map(|step| step.node.alias.clone()))
                .collect::<BTreeSet<_>>();
            let (columns, kernel) = bind_sequence(
                pattern,
                vec![Column {
                    name: start_alias,
                    ty: Ty::Entity,
                }],
                false,
                params,
                field_ids,
                &[],
            )?;
            let Kernel::ExpandSequence {
                start,
                steps,
                mode,
                path_index,
                ..
            } = kernel
            else {
                return Err(invalid("match_sequence_binding"));
            };
            (
                columns,
                Kernel::MatchSequence {
                    start,
                    steps,
                    mode,
                    path_index,
                    shortest,
                    anchors: Vec::new(),
                },
                node_aliases,
            )
        }
    };
    let mut bound_anchors = Vec::with_capacity(anchors.len());
    for (alias, expression) in anchors {
        if !node_aliases.contains(&alias) {
            return Err(invalid("match_anchor_alias"));
        }
        let record = match expression {
            wire::Expr::Literal { literal, ty } => {
                if Ty::parse(&ty)? != Ty::Entity {
                    return Err(invalid("match_anchor_type"));
                }
                let value = Ty::Entity.decode(&literal)?;
                let Val::Entity(record) = value else {
                    return Err(invalid("match_anchor_type"));
                };
                record
            }
            wire::Expr::Param { param } => {
                let (ty, value) = params.get(&param)?;
                if *ty != Ty::Entity {
                    return Err(invalid("match_anchor_type"));
                }
                let Val::Entity(record) = value.as_ref() else {
                    return Err(invalid("match_anchor_type"));
                };
                record.clone()
            }
            _ => {
                return Err(invalid("match_anchor_expression"));
            }
        };
        let column_index = columns
            .iter()
            .position(|column| column.name == alias)
            .ok_or_else(|| invalid("match_anchor_alias"))?;
        bound_anchors.push(BoundMatchAnchor {
            column_index,
            record,
        });
    }
    let mut kernel = kernel;
    match &mut kernel {
        Kernel::MatchCompact { anchors, .. } | Kernel::MatchSequence { anchors, .. } => {
            *anchors = bound_anchors;
        }
        _ => return Err(invalid("match_binding")),
    }
    Ok((columns, kernel))
}

// These are independent validated plan inputs, catalog state, and binder state.
#[allow(clippy::too_many_arguments)]
fn bind_vector_rank(
    mut scope: Vec<Column>,
    entity: String,
    collection_name: String,
    query_expr: Expr,
    k: u32,
    alias: String,
    mode: Option<wire::KnnMode>,
    kind: VectorRankKind,
    catalog: &AuthorizedCatalogV2<'_>,
    params: &BoundParametersV2,
    field_ids: &mut FieldIdRegistryV2,
) -> Result<(Vec<Column>, Kernel, QueryOpV2), QueryErrorV2> {
    if mode == Some(wire::KnnMode::Approx) {
        return Err(QueryErrorV2::new(
            "CAPABILITY_UNSUPPORTED",
            "bind",
            "knn_approx_unavailable",
        ));
    }
    let collection = catalog
        .collection(&collection_name)
        .cloned()
        .ok_or_else(|| invalid("unknown_collection"))?;
    let query = bind_expr(&query_expr, &scope, params, field_ids, 0)?;
    let Ty::Vector {
        space_id,
        dimension,
        ..
    } = query.ty.base()
    else {
        return Err(invalid("vector_query_type"));
    };
    if space_id != &collection.space_id || *dimension != collection.dimension {
        return Err(QueryErrorV2::new(
            "COLLECTION_SPACE_MISMATCH",
            "bind",
            "vector_space",
        ));
    }
    if collection.metric == "Cosine"
        && matches!(&query.kind, ExprKind::Literal(value)
            if matches!(value.as_ref(), Val::Vector(vector)
                if vector.values.iter().all(|value| *value == 0.0)))
    {
        return Err(QueryErrorV2::new(
            "COLLECTION_SPACE_MISMATCH",
            "bind",
            "zero_cosine_norm",
        ));
    }
    let (entity_index, entity_column) = scope
        .iter()
        .enumerate()
        .find(|(_, column)| column.name == entity)
        .ok_or_else(|| invalid("vector_entity"))?;
    if entity_column.ty.base() != &Ty::Entity {
        return Err(invalid("vector_entity_type"));
    }
    scope.push(Column {
        name: alias,
        ty: Ty::Score,
    });
    unique(&scope)?;
    Ok((
        scope,
        Kernel::VectorRank {
            entity_index,
            query,
            collection,
            k,
            kind,
        },
        match kind {
            VectorRankKind::Knn => QueryOpV2::Knn,
            VectorRankKind::Rerank => QueryOpV2::Rerank,
        },
    ))
}

pub(crate) fn bind_v2(
    request: LogicalRequestV2,
    params: &BoundParametersV2,
    catalog: &AuthorizedCatalogV2<'_>,
) -> Result<BoundQueryV2, QueryErrorV2> {
    // The wire decoder checks shape and reachability; defend the trusted boundary
    // too because HQL lowering constructs this same logical representation.
    if request.nodes.is_empty() || request.nodes.len() > 10_000 {
        return Err(invalid("dag_size"));
    }
    if !request.from_hql && request.parameter_types.len() != params.values.len() {
        return Err(invalid("parameter_declarations"));
    }
    for (name, declared) in &request.parameter_types {
        if Ty::parse(declared)? != params.get(name)?.0 {
            return Err(invalid("parameter_type_mismatch"));
        }
    }
    let mut nodes: Vec<BoundNode> = Vec::new();
    let mut field_ids = FieldIdRegistryV2::default();
    let mut index = BTreeMap::new();
    let mut depths = Vec::new();
    for node in request.nodes {
        if index.contains_key(&node.id) {
            return Err(invalid("duplicate_node"));
        }
        let inputs = node
            .inputs
            .iter()
            .map(|s| index.get(s).copied().ok_or_else(|| invalid("dag_order")))
            .collect::<Result<Vec<usize>, _>>()?;
        let depth = inputs.iter().map(|i| depths[*i]).max().unwrap_or(0) + 1;
        if depth > 128 {
            return Err(invalid("dag_depth"));
        }
        let expected = match &node.config {
            Config::Values { .. }
            | Config::NodeScan { .. }
            | Config::EdgeScan { .. }
            | Config::RowScan { .. }
            | Config::AnnotationScan { .. }
            | Config::HistoryScan { .. }
            | Config::ChangeScan { .. }
            | Config::Match { .. } => 0,
            Config::Join { .. } | Config::UnionAll {} => 2,
            Config::AnnotationLookup { .. }
            | Config::Expand { .. }
            | Config::Knn { .. }
            | Config::Rerank { .. }
            | Config::LexicalMatch { .. }
            | Config::ContextPack { .. } => 1,
            Config::Filter { .. }
            | Config::Project { .. }
            | Config::Distinct {}
            | Config::Sort { .. }
            | Config::Take { .. }
            | Config::Offset { .. }
            | Config::Aggregate { .. } => 1,
        };
        if inputs.len() != expected {
            return Err(invalid("input_arity"));
        }
        let scope = inputs
            .first()
            .map(|i| nodes[*i].columns.clone())
            .unwrap_or_default();
        let (columns, kernel, op) = match node.config {
            Config::Values { param, alias } => {
                let (ty, value) = params.get(&param)?;
                let (Ty::List(inner), Val::List(_)) = (ty, value.as_ref()) else {
                    return Err(invalid("values_list_parameter"));
                };
                (
                    vec![Column {
                        name: alias,
                        ty: *inner.clone(),
                    }],
                    Kernel::Values(Arc::clone(value)),
                    QueryOpV2::Values,
                )
            }
            Config::AnnotationScan { alias } => (
                vec![Column {
                    name: alias,
                    ty: Ty::Entity,
                }],
                Kernel::SourceScan(BoundSourceV2::Annotation),
                QueryOpV2::AnnotationScan,
            ),
            Config::HistoryScan { kind, id, alias } => {
                let kind = match kind {
                    wire::EntityKind::Node => crate::uee_v2::RecordKindV2::Node,
                    wire::EntityKind::Edge => crate::uee_v2::RecordKindV2::Edge,
                    wire::EntityKind::Row => crate::uee_v2::RecordKindV2::Row,
                    wire::EntityKind::Vector => crate::uee_v2::RecordKindV2::Vector,
                    wire::EntityKind::Annotation => crate::uee_v2::RecordKindV2::Annotation,
                    wire::EntityKind::Artifact => {
                        return Err(QueryErrorV2::new(
                            "CAPABILITY_UNSUPPORTED",
                            "bind",
                            "history_source_unavailable",
                        ))
                    }
                };
                let id = bind_expr(&id, &scope, params, &mut field_ids, 0)?;
                let ExprKind::Literal(value) = id.kind else {
                    return Err(invalid("history_id_constant"));
                };
                let Val::Utf8(id) = value.as_ref() else {
                    return Err(invalid("history_id_type"));
                };
                if id.is_empty() || id.len() > 1024 || id.contains('\0') {
                    return Err(invalid("history_id"));
                }
                (
                    vec![Column {
                        name: alias,
                        ty: Ty::HistoryRevision,
                    }],
                    Kernel::SourceScan(BoundSourceV2::History {
                        kind,
                        id: id.clone(),
                    }),
                    QueryOpV2::HistoryScan,
                )
            }
            Config::ChangeScan { after_seq, alias } => (
                vec![Column {
                    name: alias,
                    ty: Ty::ChangeEvent,
                }],
                Kernel::SourceScan(BoundSourceV2::Changes { after_seq }),
                QueryOpV2::ChangeScan,
            ),
            Config::AnnotationLookup {
                target,
                alias,
                optional,
            } => {
                let (target_index, column) = scope
                    .iter()
                    .enumerate()
                    .find(|(_, column)| column.name == target)
                    .ok_or_else(|| invalid("annotation_target"))?;
                if column.ty.base() != &Ty::Entity {
                    return Err(invalid("annotation_target_type"));
                }
                let mut columns = scope;
                columns.push(Column {
                    name: alias,
                    ty: if optional {
                        Ty::Entity.as_nullable()
                    } else {
                        Ty::Entity
                    },
                });
                unique(&columns)?;
                (
                    columns,
                    Kernel::AnnotationLookup {
                        target_index,
                        optional,
                    },
                    QueryOpV2::AnnotationLookup,
                )
            }
            Config::Knn {
                entity,
                collection,
                query,
                k,
                mode,
                alias,
            } => bind_vector_rank(
                scope,
                entity,
                collection,
                query,
                k,
                alias,
                Some(mode),
                VectorRankKind::Knn,
                catalog,
                params,
                &mut field_ids,
            )?,
            Config::Rerank {
                entity,
                collection,
                query,
                k,
                fidelity: _,
                alias,
            } => bind_vector_rank(
                scope,
                entity,
                collection,
                query,
                k,
                alias,
                None,
                VectorRankKind::Rerank,
                catalog,
                params,
                &mut field_ids,
            )?,
            Config::LexicalMatch {
                entity,
                field,
                index,
                query,
                k,
                alias,
            } => {
                if !is_registered_analyzer(&index) {
                    return Err(QueryErrorV2::new(
                        "CAPABILITY_UNSUPPORTED",
                        "bind",
                        "unknown_analyzer",
                    ));
                }
                let entity_index = scope
                    .iter()
                    .position(|column| column.name == entity)
                    .ok_or_else(|| invalid("lexical_entity"))?;
                if scope[entity_index].ty.base() != &Ty::Entity {
                    return Err(invalid("lexical_entity_type"));
                }
                let query = bind_expr(&query, &scope, params, &mut field_ids, 0)?;
                let ExprKind::Literal(value) = &query.kind else {
                    return Err(invalid("lexical_query_constant"));
                };
                let Val::Utf8(query) = value.as_ref() else {
                    return Err(invalid("lexical_query_type"));
                };
                let field = field_ids.issue(false, Some(&field))?;
                let has_property = field_ids.issue(true, Some(field.name().unwrap_or_default()))?;
                let alias_index = scope.len();
                let mut columns = scope;
                columns.push(Column {
                    name: alias,
                    ty: Ty::Score,
                });
                unique(&columns)?;
                (
                    columns,
                    Kernel::LexicalMatch {
                        entity_index,
                        field,
                        has_property,
                        query: query.clone(),
                        k,
                        alias_index,
                    },
                    QueryOpV2::LexicalMatch,
                )
            }
            Config::ContextPack {
                text,
                evidence,
                tokens,
                tokenizer,
                alias,
            } => {
                if !is_registered_tokenizer(&tokenizer) {
                    return Err(QueryErrorV2::new(
                        "CAPABILITY_UNSUPPORTED",
                        "bind",
                        "unknown_tokenizer",
                    ));
                }
                let text = bind_expr(&text, &scope, params, &mut field_ids, 0)?;
                let evidence = bind_expr(&evidence, &scope, params, &mut field_ids, 0)?;
                if text.ty.base() != &Ty::Json || evidence.ty != Ty::Entity {
                    return Err(invalid("context_source_type"));
                }
                let text_entity = match &text.kind {
                    ExprKind::Property {
                        entity,
                        name,
                        field,
                        ..
                    } if field.name().is_some()
                        && matches!(name.kind, ExprKind::Literal(ref value) if matches!(value.as_ref(), Val::Utf8(_))) =>
                    {
                        entity.as_ref().clone()
                    }
                    _ => return Err(invalid("context_text_source")),
                };
                let (ExprKind::Field(text_index), ExprKind::Field(evidence_index)) =
                    (&text_entity.kind, &evidence.kind)
                else {
                    return Err(invalid("context_evidence_source"));
                };
                if text_index != evidence_index {
                    return Err(invalid("context_evidence_source"));
                }
                let fingerprint = text_profile_fingerprints()
                    .into_iter()
                    .find(|(name, _)| *name == "tokenizer:unicode-scalar-v1")
                    .map(|(_, fingerprint)| fingerprint)
                    .ok_or_else(unsupported)?;
                (
                    vec![Column {
                        name: alias,
                        ty: Ty::Context,
                    }],
                    Kernel::ContextPack {
                        text,
                        evidence,
                        text_entity,
                        tokens,
                        tokenizer_fingerprint: fingerprint,
                    },
                    QueryOpV2::ContextPack,
                )
            }
            Config::Match {
                pattern,
                anchors,
                shortest,
            } => {
                let (columns, kernel) =
                    bind_match(pattern, anchors, shortest, params, &mut field_ids)?;
                (columns, kernel, QueryOpV2::Match)
            }
            Config::Expand { pattern, optional } => {
                if let wire::Pattern::Sequence(pattern) = pattern {
                    let constraint_scope = scope.clone();
                    let (columns, kernel) = bind_sequence(
                        pattern,
                        scope,
                        optional,
                        params,
                        &mut field_ids,
                        &constraint_scope,
                    )?;
                    (columns, kernel, QueryOpV2::Expand)
                } else {
                    let wire::Pattern::Compact(pattern) = pattern else {
                        unreachable!()
                    };
                    let (start_index, start) = scope
                        .iter()
                        .enumerate()
                        .find(|(_, column)| column.name == pattern.start_alias)
                        .ok_or_else(|| invalid("expand_start"))?;
                    if start.ty.base() != &Ty::Entity {
                        return Err(invalid("expand_start_type"));
                    }
                    let end_index = scope
                        .iter()
                        .position(|column| column.name == pattern.end_alias);
                    if end_index.is_some_and(|index| scope[index].ty.base() != &Ty::Entity) {
                        return Err(invalid("expand_end_type"));
                    }
                    let mut columns = scope;
                    if end_index.is_none() {
                        columns.push(Column {
                            name: pattern.end_alias.clone(),
                            ty: nullable(Ty::Entity, optional),
                        });
                    }
                    if let Some(alias) = &pattern.edge_alias {
                        let ty = if pattern.min_hops == 1 && pattern.max_hops == 1 {
                            Ty::Entity
                        } else {
                            Ty::List(Box::new(Ty::Entity))
                        };
                        columns.push(Column {
                            name: alias.clone(),
                            ty: nullable(ty, optional),
                        });
                    }
                    if let Some(alias) = &pattern.path_alias {
                        columns.push(Column {
                            name: alias.clone(),
                            ty: nullable(Ty::Path, optional),
                        });
                    }
                    unique(&columns)?;
                    (
                        columns,
                        Kernel::Expand {
                            start_index,
                            end_index,
                            pattern,
                            optional,
                        },
                        QueryOpV2::Expand,
                    )
                }
            }
            Config::NodeScan { alias, label } => (
                vec![Column {
                    name: alias,
                    ty: Ty::Entity,
                }],
                Kernel::SourceScan(BoundSourceV2::Node(label)),
                QueryOpV2::NodeScan,
            ),
            Config::EdgeScan { alias, relation } => (
                vec![Column {
                    name: alias,
                    ty: Ty::Entity,
                }],
                Kernel::SourceScan(BoundSourceV2::Edge(relation)),
                QueryOpV2::EdgeScan,
            ),
            Config::RowScan { table, alias } => {
                if !catalog.has_table(&table) {
                    return Err(invalid("unknown_table"));
                }
                (
                    vec![Column {
                        name: alias,
                        ty: Ty::Entity,
                    }],
                    Kernel::SourceScan(BoundSourceV2::Row(table)),
                    QueryOpV2::RowScan,
                )
            }
            Config::Filter { predicate } => {
                let e = bind_expr(&predicate, &scope, params, &mut field_ids, 0)?;
                boolean(&e.ty)?;
                (scope, Kernel::Filter(e), QueryOpV2::Filter)
            }
            Config::Project { fields } => {
                if fields.is_empty() {
                    return Err(invalid("project_empty"));
                }
                let (c, e) = bind_named(&fields, &scope, params, &mut field_ids)?;
                (c, Kernel::Project(e), QueryOpV2::Project)
            }
            Config::Distinct {} => (scope, Kernel::Distinct, QueryOpV2::Distinct),
            Config::Sort { keys } => {
                if keys.is_empty() {
                    return Err(invalid("sort_empty"));
                }
                let mut bound = Vec::new();
                for key in keys {
                    let e = bind_expr(&key.expression, &scope, params, &mut field_ids, 0)?;
                    if !ordered(&e.ty) {
                        return Err(invalid("comparator_unavailable"));
                    }
                    bound.push((e, key.direction, key.nulls));
                }
                (scope, Kernel::Sort(bound), QueryOpV2::Sort)
            }
            Config::Take { count } => (scope, Kernel::Take(count), QueryOpV2::Take),
            Config::Offset { count } => (scope, Kernel::Offset(count), QueryOpV2::Offset),
            Config::UnionAll {} => {
                if scope != nodes[inputs[1]].columns {
                    return Err(invalid("union_schema"));
                }
                (scope, Kernel::UnionAll, QueryOpV2::UnionAll)
            }
            Config::Join { kind, condition } => {
                let right = &nodes[inputs[1]].columns;
                let mut combined = scope.clone();
                combined.extend_from_slice(right);
                unique(&combined)?;
                let condition = bind_expr(&condition, &combined, params, &mut field_ids, 0)?;
                boolean(&condition.ty)?;
                let columns = match kind {
                    JoinKind::Semi | JoinKind::Anti => scope,
                    JoinKind::Inner => combined,
                    JoinKind::Left => {
                        let mut c = scope;
                        c.extend(right.iter().map(|c| Column {
                            name: c.name.clone(),
                            ty: c.ty.as_nullable(),
                        }));
                        c
                    }
                };
                (
                    columns,
                    Kernel::Join {
                        kind,
                        condition,
                        right_width: right.len(),
                    },
                    QueryOpV2::Join,
                )
            }
            Config::Aggregate {
                group_by,
                aggregates,
            } => {
                let (mut columns, groups) = bind_named(&group_by, &scope, params, &mut field_ids)?;
                let mut bound = Vec::new();
                for named in aggregates {
                    let Expr::Call { call, args } = named.expression else {
                        return Err(invalid("aggregate_call"));
                    };
                    let kind = match call.as_str() {
                        "count_all" => AggregateKind::CountAll,
                        "count" => AggregateKind::Count,
                        "sum" => AggregateKind::Sum,
                        "avg" => AggregateKind::Avg,
                        "min" => AggregateKind::Min,
                        "max" => AggregateKind::Max,
                        "collect" => AggregateKind::Collect,
                        _ => return Err(invalid("aggregate_function")),
                    };
                    let arity = if matches!(kind, AggregateKind::CountAll) {
                        0
                    } else {
                        1
                    };
                    if args.len() != arity {
                        return Err(invalid("aggregate_arity"));
                    }
                    let arg = args
                        .first()
                        .map(|e| bind_expr(e, &scope, params, &mut field_ids, 0))
                        .transpose()?;
                    let ty = match kind {
                        AggregateKind::CountAll | AggregateKind::Count => Ty::I64,
                        AggregateKind::Sum | AggregateKind::Avg => {
                            let ty = &arg.as_ref().unwrap().ty;
                            if !numeric(ty) {
                                return Err(invalid("aggregate_numeric"));
                            }
                            if matches!(kind, AggregateKind::Avg) {
                                Ty::F64Finite.as_nullable()
                            } else {
                                ty.as_nullable()
                            }
                        }
                        AggregateKind::Min | AggregateKind::Max => {
                            let ty = &arg.as_ref().unwrap().ty;
                            if !ordered(ty) {
                                return Err(invalid("comparator_unavailable"));
                            }
                            ty.as_nullable()
                        }
                        AggregateKind::Collect => {
                            Ty::List(Box::new(arg.as_ref().unwrap().ty.clone()))
                        }
                    };
                    columns.push(Column {
                        name: named.alias,
                        ty,
                    });
                    bound.push(Aggregate { kind, arg });
                }
                if columns.is_empty() {
                    return Err(invalid("aggregate_empty"));
                }
                unique(&columns)?;
                (
                    columns,
                    Kernel::Aggregate {
                        groups,
                        aggregates: bound,
                    },
                    QueryOpV2::Aggregate,
                )
            }
        };
        if op != node.op {
            return Err(invalid("operator_config_mismatch"));
        }
        index.insert(node.id.clone(), nodes.len());
        depths.push(depth);
        nodes.push(BoundNode {
            id: node.id,
            op,
            inputs,
            columns,
            kernel,
        });
    }
    let root = *index
        .get(&request.root)
        .ok_or_else(|| invalid("root_missing"))?;
    let mut reachable = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(i) = stack.pop() {
        if reachable.insert(i) {
            stack.extend_from_slice(&nodes[i].inputs);
        }
    }
    if reachable.len() != nodes.len() {
        return Err(invalid("orphan_node"));
    }
    Ok(BoundQueryV2 { nodes, root })
}
