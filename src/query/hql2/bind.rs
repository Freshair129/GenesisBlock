//! Semantic scalar binding. Only this module constructs trusted bound nodes.
use super::{
    catalog::AuthorizedCatalogV2,
    error::QueryErrorV2,
    value::{QueryTypeV2 as Ty, QueryValueV2 as Val},
    wire::{
        self, BinaryOp, Config, Expr, JoinKind, LogicalRequestV2, NullOrder, SortDirection, UnaryOp,
    },
};
use crate::uee_v2::QueryOpV2;
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
    ) -> Result<Self, QueryErrorV2> {
        let mut values = BTreeMap::new();
        for (name, value) in params {
            let object = value
                .as_object()
                .ok_or_else(|| invalid("typed_parameter"))?;
            if object.len() != 2 || !object.contains_key("value") {
                return Err(invalid("typed_parameter"));
            }
            let ty = Ty::parse(
                object
                    .get("type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| invalid("typed_parameter"))?,
            )?;
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
    matches!(ty.base(), Ty::Bool | Ty::I64 | Ty::F64Finite | Ty::Utf8)
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
fn bind_expr(
    expr: &Expr,
    scope: &[Column],
    params: &BoundParametersV2,
    depth: usize,
) -> Result<BoundExpr, QueryErrorV2> {
    let mut stack = vec![(expr, depth, false)];
    let mut ready = BTreeMap::new();
    while let Some((current, depth, visited)) = stack.pop() {
        if depth >= 128 {
            return Err(invalid("expression_depth"));
        }
        if visited {
            let bound = bind_expr_node(current, scope, params, &mut ready)?;
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
            let v = ty.decode(literal)?;
            (ty, ExprKind::Literal(Arc::new(v)))
        }
        Expr::Param { param } => {
            let (ty, value) = params.get(param)?;
            (ty.clone(), ExprKind::Literal(value.clone()))
        }
        Expr::Field { field } => {
            let (index, column) = scope
                .iter()
                .enumerate()
                .find(|(_, c)| c.name == field.alias)
                .ok_or_else(|| invalid("unknown_field"))?;
            if !field.path.is_empty() {
                return Err(invalid("scalar_field_path"));
            }
            (column.ty.clone(), ExprKind::Field(index))
        }
        Expr::Binary {
            binary,
            left,
            right,
        } => {
            let left = next(left)?;
            let right = next(right)?;
            let null = left.ty.nullable() || right.ty.nullable();
            use BinaryOp::*;
            let ty = match binary {
                And | Or => {
                    boolean(&left.ty)?;
                    boolean(&right.ty)?;
                    nullable(Ty::Bool, null)
                }
                Add | Sub | Mul | Div => {
                    if !numeric(&left.ty) || left.ty.base() != right.ty.base() {
                        return Err(invalid("numeric_type"));
                    }
                    nullable(left.ty.base().clone(), null)
                }
                Eq | Ne => {
                    if left.ty.base() != right.ty.base() {
                        return Err(invalid("comparison_type"));
                    }
                    if !ordered(&left.ty) {
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
                ExprKind::Binary(binary.clone(), Box::new(left), Box::new(right)),
            )
        }
        Expr::Unary { unary, arg } => {
            let arg = next(arg)?;
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
            (ty, ExprKind::Unary(unary.clone(), Box::new(arg)))
        }
        Expr::Call { call, args } => {
            if matches!(call.as_str(), "prop" | "has_prop") {
                return Err(unsupported());
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
            let values = values.iter().map(next).collect::<Result<Vec<_>, _>>()?;
            if !ordered(&needle.ty) {
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
) -> Result<(Vec<Column>, Vec<BoundExpr>), QueryErrorV2> {
    let mut columns = Vec::new();
    let mut exprs = Vec::new();
    for f in fields {
        let e = bind_expr(&f.expression, scope, params, 0)?;
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

pub(crate) fn bind_v2(
    request: LogicalRequestV2,
    params: &BoundParametersV2,
    _catalog: &AuthorizedCatalogV2<'_>,
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
            Config::Values { .. } => 0,
            Config::Join { .. } | Config::UnionAll {} => 2,
            Config::Filter { .. }
            | Config::Project { .. }
            | Config::Distinct {}
            | Config::Sort { .. }
            | Config::Take { .. }
            | Config::Offset { .. }
            | Config::Aggregate { .. } => 1,
            _ => return Err(unsupported()),
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
            Config::Filter { predicate } => {
                let e = bind_expr(&predicate, &scope, params, 0)?;
                boolean(&e.ty)?;
                (scope, Kernel::Filter(e), QueryOpV2::Filter)
            }
            Config::Project { fields } => {
                if fields.is_empty() {
                    return Err(invalid("project_empty"));
                }
                let (c, e) = bind_named(&fields, &scope, params)?;
                (c, Kernel::Project(e), QueryOpV2::Project)
            }
            Config::Distinct {} => (scope, Kernel::Distinct, QueryOpV2::Distinct),
            Config::Sort { keys } => {
                if keys.is_empty() {
                    return Err(invalid("sort_empty"));
                }
                let mut bound = Vec::new();
                for key in keys {
                    let e = bind_expr(&key.expression, &scope, params, 0)?;
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
                let condition = bind_expr(&condition, &combined, params, 0)?;
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
                let (mut columns, groups) = bind_named(&group_by, &scope, params)?;
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
                        .map(|e| bind_expr(e, &scope, params, 0))
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
            _ => return Err(unsupported()),
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
