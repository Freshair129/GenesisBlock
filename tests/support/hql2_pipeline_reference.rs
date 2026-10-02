//! Independent P7 composition harness. This is not a production query endpoint.
//! All 23 logical families dispatch through explicit fixture operators. Domains
//! share one snapshot and preserve typed references, bag rows and score provenance.
#[path = "hql2_reference.rs"]
pub mod relational;
pub use relational::{graph, rank, Aggregate, Expr, JoinKind, Row, SortKey, Value};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
type GraphBudget = Cell<usize>;

#[derive(Clone, Debug)]
pub struct Environment {
    pub catalog: graph::Catalog,
    pub view: graph::View,
    pub ranking: rank::Fixture,
    pub tokenizers: rank::TokenizerRegistry,
    pub limits: graph::Limits,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Scalar(&'static str),
    Graph(graph::Error),
    Ranking(rank::Error),
    Schema,
    OwnerDomain,
    Depth,
    RowLimit,
}
impl From<&'static str> for Error {
    fn from(e: &'static str) -> Self {
        Self::Scalar(e)
    }
}
impl From<graph::Error> for Error {
    fn from(e: graph::Error) -> Self {
        Self::Graph(e)
    }
}
impl From<rank::Error> for Error {
    fn from(e: rank::Error) -> Self {
        Self::Ranking(e)
    }
}
type Outcome<T> = Result<T, Error>;
#[derive(Clone, Debug, PartialEq)]
pub struct Metadata {
    pub snapshot: rank::Snapshot,
    pub approximate_sources: BTreeSet<String>,
    pub rerank_candidates: bool,
    /// Score names map to the source binding, not a later expanded endpoint.
    pub score_owners: BTreeMap<String, ScoreOwner>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreOwner {
    pub origin: String,
    pub binding: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ResultSet {
    pub rows: Vec<Row>,
    pub metadata: Metadata,
    pub graph_schema: graph::Schema,
    pub columns: BTreeSet<String>,
    types: BTreeMap<String, ValueType>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueType {
    Null,
    Bool,
    I64,
    F64,
    Text,
    Vector,
    List,
    Graph,
    Context,
    Unknown,
}
fn value_type(v: &Value) -> ValueType {
    match v {
        Value::Null => ValueType::Null,
        Value::Bool(_) => ValueType::Bool,
        Value::I64(_) => ValueType::I64,
        Value::F64(_) => ValueType::F64,
        Value::Text(_) => ValueType::Text,
        Value::Vector(_) => ValueType::Vector,
        Value::List(_) => ValueType::List,
        Value::Graph(_) => ValueType::Graph,
        Value::Context(_) => ValueType::Context,
    }
}
fn boolean_type(t: ValueType) -> Outcome<()> {
    if matches!(t, ValueType::Bool | ValueType::Null | ValueType::Unknown) {
        Ok(())
    } else {
        Err(Error::Scalar("TYPE_MISMATCH"))
    }
}
fn sortable_type(t: ValueType) -> Outcome<()> {
    if matches!(
        t,
        ValueType::Bool
            | ValueType::I64
            | ValueType::F64
            | ValueType::Text
            | ValueType::Null
            | ValueType::Unknown
    ) {
        Ok(())
    } else {
        Err(Error::Scalar("TYPE_MISMATCH"))
    }
}
fn numeric_type(t: ValueType) -> Outcome<()> {
    if matches!(
        t,
        ValueType::I64 | ValueType::F64 | ValueType::Null | ValueType::Unknown
    ) {
        Ok(())
    } else {
        Err(Error::Scalar("TYPE_MISMATCH"))
    }
}
fn expression_type(expr: &Expr, types: &BTreeMap<String, ValueType>) -> Outcome<ValueType> {
    Ok(match expr {
        Expr::Literal(v) => value_type(v),
        Expr::Field(n) => *types.get(n).ok_or(Error::Scalar("FIELD_UNKNOWN"))?,
        Expr::Property(_) => ValueType::Unknown,
        Expr::HasProperty(_) => ValueType::Bool,
        Expr::Not(a) => {
            boolean_type(expression_type(a, types)?)?;
            ValueType::Bool
        }
        Expr::And(a, b) | Expr::Or(a, b) => {
            boolean_type(expression_type(a, types)?)?;
            boolean_type(expression_type(b, types)?)?;
            ValueType::Bool
        }
        Expr::Eq(a, b) => {
            let a = expression_type(a, types)?;
            let b = expression_type(b, types)?;
            if a != b
                && !matches!(a, ValueType::Null | ValueType::Unknown)
                && !matches!(b, ValueType::Null | ValueType::Unknown)
            {
                return Err(Error::Scalar("TYPE_MISMATCH"));
            }
            ValueType::Bool
        }
        Expr::Add(a, b) | Expr::Div(a, b) | Expr::Rem(a, b) => {
            let a = expression_type(a, types)?;
            let b = expression_type(b, types)?;
            if !matches!(
                a,
                ValueType::I64 | ValueType::F64 | ValueType::Null | ValueType::Unknown
            ) || !matches!(
                b,
                ValueType::I64 | ValueType::F64 | ValueType::Null | ValueType::Unknown
            ) || (matches!(a, ValueType::I64 | ValueType::F64)
                && matches!(b, ValueType::I64 | ValueType::F64)
                && a != b)
            {
                return Err(Error::Scalar("TYPE_MISMATCH"));
            }
            if a == ValueType::Null {
                b
            } else {
                a
            }
        }
    })
}
#[derive(Clone, Debug)]
pub enum Plan {
    Values(Vec<Row>),
    NodeScan(String, graph::Predicate),
    EdgeScan(String, graph::Predicate),
    RowScan(String, graph::Predicate),
    AnnotationScan(String, graph::Predicate),
    HistoryScan {
        kind: graph::Kind,
        alias: String,
        predicate: graph::Predicate,
        transactions: graph::Interval<u64>,
        valid: graph::Interval<i64>,
    },
    ChangeScan {
        alias: String,
        predicate: graph::Predicate,
        after: u64,
        through: u64,
    },
    Match {
        start: String,
        predicate: graph::Predicate,
        expansion: graph::Expand,
    },
    Expand(Box<Plan>, graph::Expand),
    AnnotationLookup(Box<Plan>, graph::AnnotationLookup),
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
        owner: String,
        output: String,
        request: rank::VectorRequest,
    },
    Rerank {
        input: Box<Plan>,
        owner: String,
        output: String,
        request: rank::VectorRequest,
    },
    LexicalMatch {
        input: Box<Plan>,
        owner: String,
        output: String,
        request: rank::LexicalRequest,
    },
    ContextPack {
        input: Box<Plan>,
        owner: String,
        output: String,
        tokenizer: String,
        budget: usize,
    },
}
fn empty(env: &Environment) -> ResultSet {
    ResultSet {
        rows: vec![],
        metadata: Metadata {
            snapshot: rank::Snapshot {
                transaction: env.view.transaction,
                valid: env.view.valid_at,
            },
            approximate_sources: BTreeSet::new(),
            rerank_candidates: false,
            score_owners: BTreeMap::new(),
        },
        types: BTreeMap::new(),
        graph_schema: BTreeMap::new(),
        columns: BTreeSet::new(),
    }
}
fn graph_value(b: graph::Binding) -> Value {
    if b == graph::Binding::Null {
        Value::Null
    } else {
        Value::Graph(b)
    }
}
fn from_graph(env: &Environment, relation: graph::Relation) -> ResultSet {
    ResultSet {
        types: relation
            .schema
            .keys()
            .map(|n| (n.clone(), ValueType::Graph))
            .collect(),
        columns: relation.schema.keys().cloned().collect(),
        graph_schema: relation.schema,
        rows: relation
            .rows
            .into_iter()
            .map(|r| r.into_iter().map(|(k, v)| (k, graph_value(v))).collect())
            .collect(),
        ..empty(env)
    }
}
fn source(env: &Environment, budget: &GraphBudget, source: graph::Source) -> Outcome<ResultSet> {
    Ok(from_graph(
        env,
        graph_execute(
            env,
            budget,
            &graph::Plan {
                source,
                stages: vec![],
            },
            env.limits,
        )?,
    ))
}
fn input_relation(data: &ResultSet, rows: &[Row]) -> Outcome<graph::Relation> {
    let rows = rows
        .iter()
        .map(|row| {
            data.graph_schema
                .keys()
                .map(|key| {
                    let binding = match row.get(key) {
                        Some(Value::Graph(v)) => v.clone(),
                        Some(Value::Null) => graph::Binding::Null,
                        _ => return Err(Error::Schema),
                    };
                    Ok((key.clone(), binding))
                })
                .collect()
        })
        .collect::<Outcome<Vec<_>>>()?;
    Ok(graph::Relation {
        schema: data.graph_schema.clone(),
        rows,
    })
}
fn transform(
    env: &Environment,
    budget: &GraphBudget,
    mut data: ResultSet,
    stage: graph::Stage,
) -> Outcome<ResultSet> {
    let schema = graph_transform(
        env,
        budget,
        &input_relation(&data, &[])?,
        std::slice::from_ref(&stage),
        env.limits,
    )?
    .schema;
    if schema
        .keys()
        .any(|key| data.columns.contains(key) && !data.graph_schema.contains_key(key))
    {
        return Err(Error::Schema);
    }
    let mut result = Vec::new();
    for row in &data.rows {
        let expanded = graph_transform(
            env,
            budget,
            &input_relation(&data, std::slice::from_ref(row))?,
            std::slice::from_ref(&stage),
            env.limits,
        )?;
        for bindings in expanded.rows {
            let mut carried = row.clone();
            for (name, value) in bindings {
                carried.insert(name, graph_value(value));
            }
            result.push(carried);
            if result.len() > env.limits.max_rows {
                return Err(Error::RowLimit);
            }
        }
    }
    data.rows = result;
    data.columns.extend(schema.keys().cloned());
    data.types
        .extend(schema.keys().map(|n| (n.clone(), ValueType::Graph)));
    data.graph_schema = schema;
    Ok(data)
}
fn merge_metadata(a: &mut Metadata, b: &Metadata) -> Outcome<()> {
    if a.snapshot != b.snapshot {
        return Err(Error::Schema);
    }
    a.approximate_sources
        .extend(b.approximate_sources.iter().cloned());
    a.rerank_candidates |= b.rerank_candidates;
    for (name, owner) in &b.score_owners {
        if a.score_owners.get(name).is_some_and(|old| old != owner) {
            return Err(Error::Schema);
        }
        a.score_owners.insert(name.clone(), owner.clone());
    }
    Ok(())
}
fn project_schema(data: &mut ResultSet, fields: &[(String, Expr)]) -> Outcome<()> {
    let types = fields
        .iter()
        .map(|(n, e)| Ok((n.clone(), expression_type(e, &data.types)?)))
        .collect::<Outcome<BTreeMap<_, _>>>()?;
    data.graph_schema = fields
        .iter()
        .filter_map(|(name, expr)| match expr {
            Expr::Field(old) => data.graph_schema.get(old).map(|t| (name.clone(), *t)),
            Expr::Literal(Value::Graph(graph::Binding::Entity(e))) => {
                Some((name.clone(), graph::BindingType::Entity(e.kind)))
            }
            Expr::Literal(Value::Graph(graph::Binding::Edges(_))) => {
                Some((name.clone(), graph::BindingType::Edges))
            }
            Expr::Literal(Value::Graph(graph::Binding::Path(_))) => {
                Some((name.clone(), graph::BindingType::Path))
            }
            _ => None,
        })
        .collect();
    data.metadata.score_owners = fields
        .iter()
        .filter_map(|(name, expr)| match expr {
            Expr::Field(old) => data.metadata.score_owners.get(old).map(|v| {
                let binding = v.binding.as_ref().and_then(|old_owner| {
                    fields.iter().find_map(|(new_owner, e)| match e {
                        Expr::Field(old) if old == old_owner => Some(new_owner.clone()),
                        _ => None,
                    })
                });
                (
                    name.clone(),
                    ScoreOwner {
                        origin: v.origin.clone(),
                        binding,
                    },
                )
            }),
            _ => None,
        })
        .collect();
    data.columns = fields.iter().map(|(name, _)| name.clone()).collect();
    data.types = types;
    Ok(())
}
/// Keys encode the full ordered fixture row, including namespace/kind/revision/
/// path identities. Only identical rows receive an occurrence suffix; no bags
/// are deduplicated. Debug encoding here is fixture-only, never a wire contract.
fn candidates(
    env: &Environment,
    budget: &GraphBudget,
    data: &ResultSet,
    owner: &str,
) -> Outcome<(rank::Batch, BTreeMap<String, Row>)> {
    if data.graph_schema.get(owner) != Some(&graph::BindingType::Entity(graph::Kind::Node)) {
        return Err(Error::OwnerDomain);
    }
    let mut occurrences = BTreeMap::<String, usize>::new();
    let mut originals = BTreeMap::new();
    let mut rows = Vec::new();
    for row in &data.rows {
        let entity = match row.get(owner) {
            Some(Value::Graph(graph::Binding::Entity(e))) => e,
            Some(Value::Null) => continue,
            _ => return Err(Error::OwnerDomain),
        };
        if entity.namespace != env.view.namespace || entity.kind != graph::Kind::Node {
            return Err(Error::OwnerDomain);
        }
        // Revalidate injected Values/relational bindings at the common S,V and
        // current graph policy before consulting a vector/text fixture.
        let relation = graph::Relation {
            schema: BTreeMap::from([(owner.into(), graph::BindingType::Entity(graph::Kind::Node))]),
            rows: vec![BTreeMap::from([(
                owner.into(),
                graph::Binding::Entity(entity.clone()),
            )])],
        };
        if graph_transform(env, budget, &relation, &[], env.limits)?
            .rows
            .is_empty()
        {
            continue;
        }
        let encoding = format!("{row:?}");
        let occurrence = occurrences.entry(encoding.clone()).or_default();
        let key = format!("{encoding}#{occurrence:08}");
        *occurrence += 1;
        originals.insert(key.clone(), row.clone());
        rows.push(rank::Candidate {
            row_key: key,
            owner_id: entity.id.clone(),
            revision: entity.revision.clone(),
            language: String::new(),
            hit: None,
            source_ranks: BTreeMap::new(),
        });
    }
    Ok((
        rank::Batch {
            snapshot: data.metadata.snapshot,
            rows,
            lineage: rank::Lineage {
                approximate_sources: data.metadata.approximate_sources.iter().cloned().collect(),
                scope: if data.metadata.rerank_candidates {
                    rank::Scope::RerankCandidates
                } else {
                    rank::Scope::WholeInput
                },
            },
        },
        originals,
    ))
}
fn scored(
    mut data: ResultSet,
    ranked: rank::Batch,
    mut originals: BTreeMap<String, Row>,
    owner: &str,
    output: &str,
) -> Outcome<ResultSet> {
    if output.is_empty() || data.columns.contains(output) {
        return Err(Error::Schema);
    }
    data.rows = ranked
        .rows
        .into_iter()
        .map(|r| {
            let mut row = originals.remove(&r.row_key).ok_or(Error::Schema)?;
            row.insert(output.into(), Value::F64(r.hit.ok_or(Error::Schema)?.value));
            Ok(row)
        })
        .collect::<Outcome<_>>()?;
    data.metadata.approximate_sources = ranked.lineage.approximate_sources.into_iter().collect();
    data.metadata.rerank_candidates = ranked.lineage.scope == rank::Scope::RerankCandidates;
    data.metadata.score_owners.insert(
        output.into(),
        ScoreOwner {
            origin: owner.into(),
            binding: Some(owner.into()),
        },
    );
    data.columns.insert(output.into());
    data.types.insert(output.into(), ValueType::F64);
    Ok(data)
}
fn ranking_fixture(env: &Environment, budget: &GraphBudget) -> Outcome<rank::Fixture> {
    let visible = source(
        env,
        budget,
        graph::Source::Scan {
            kind: graph::Kind::Node,
            alias: "owner".into(),
            predicate: graph::Predicate::default(),
        },
    )?;
    let owners = visible
        .rows
        .iter()
        .filter_map(|r| match &r["owner"] {
            Value::Graph(graph::Binding::Entity(e)) => Some((e.id.clone(), e.revision.clone())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut fixture = env.ranking.clone();
    for doc in &mut fixture.documents {
        doc.visibility.authorized &= owners.contains(&(doc.id.clone(), doc.revision.clone()));
    }
    Ok(fixture)
}
pub fn execute(env: &Environment, plan: &Plan) -> Outcome<ResultSet> {
    let budget = &GraphBudget::new(env.limits.max_work);
    // Preflight before evaluation; bounded logical depth does not guarantee a
    // Rust thread has space for equally deep, large interpreter stack frames.
    let mut pending = vec![(plan, false, 0usize)];
    let mut order = Vec::new();
    while let Some((node, visited, depth)) = pending.pop() {
        if depth > 128 {
            return Err(Error::Depth);
        }
        if visited {
            order.push(node);
            if order.len() > 10_000 {
                return Err(Error::RowLimit);
            }
        } else {
            pending.push((node, true, depth));
            for child in children(node).into_iter().rev() {
                pending.push((child, false, depth + 1));
            }
        }
    }
    let mut results = Vec::new();
    for node in order {
        let count = children(node).len();
        let start = results.len().checked_sub(count).ok_or(Error::Schema)?;
        let inputs = results.drain(start..).collect();
        results.push(evaluate(env, budget, node, inputs)?);
    }
    results.pop().ok_or(Error::Schema)
}
fn children(plan: &Plan) -> Vec<&Plan> {
    match plan {
        Plan::UnionAll(a, b)
        | Plan::Join {
            left: a, right: b, ..
        } => vec![a, b],
        Plan::Expand(p, _)
        | Plan::AnnotationLookup(p, _)
        | Plan::Filter(p, _)
        | Plan::Project(p, _)
        | Plan::Distinct(p)
        | Plan::Take(p, _)
        | Plan::Offset(p, _)
        | Plan::Sort(p, _)
        | Plan::Aggregate { input: p, .. }
        | Plan::Knn { input: p, .. }
        | Plan::Rerank { input: p, .. }
        | Plan::LexicalMatch { input: p, .. }
        | Plan::ContextPack { input: p, .. } => vec![p],
        _ => vec![],
    }
}
fn bind_expr(expr: &Expr, columns: &BTreeSet<String>, depth: usize) -> Outcome<()> {
    if depth > 128 {
        return Err(Error::Depth);
    }
    match expr {
        Expr::Field(name) if !columns.contains(name) => Err(Error::Scalar("FIELD_UNKNOWN")),
        Expr::Not(a) => bind_expr(a, columns, depth + 1),
        Expr::Eq(a, b)
        | Expr::And(a, b)
        | Expr::Or(a, b)
        | Expr::Add(a, b)
        | Expr::Div(a, b)
        | Expr::Rem(a, b) => {
            bind_expr(a, columns, depth + 1)?;
            bind_expr(b, columns, depth + 1)
        }
        Expr::Literal(_) => {
            relational::eval(expr, &Row::new())?;
            Ok(())
        }
        _ => Ok(()),
    }
}
fn evaluate(
    env: &Environment,
    budget: &GraphBudget,
    plan: &Plan,
    mut inputs: VecDeque<ResultSet>,
) -> Outcome<ResultSet> {
    let mut run = |_p: &Plan| inputs.pop_front().ok_or(Error::Schema);
    let result = match plan {
        Plan::Values(rows) => {
            let mut data = empty(env);
            data.rows = relational::execute(&relational::Plan::Values(rows.clone()))?;
            if let Some(row) = rows.first() {
                data.columns = row.keys().cloned().collect();
            }
            for row in rows {
                if row.keys().cloned().collect::<BTreeSet<_>>() != data.columns {
                    return Err(Error::Schema);
                }
                for (name, v) in row {
                    let ty = value_type(v);
                    if let Some(old) = data.types.get(name) {
                        if *old != ty && *old != ValueType::Null && ty != ValueType::Null {
                            return Err(Error::Schema);
                        }
                    }
                    if ty != ValueType::Null || !data.types.contains_key(name) {
                        data.types.insert(name.clone(), ty);
                    }
                    if let Value::Graph(binding) = v {
                        let ty = match binding {
                            graph::Binding::Entity(e) => graph::BindingType::Entity(e.kind),
                            graph::Binding::Edges(_) => graph::BindingType::Edges,
                            graph::Binding::Path(_) => graph::BindingType::Path,
                            graph::Binding::Null => return Err(Error::Schema),
                        };
                        if data
                            .graph_schema
                            .insert(name.clone(), ty)
                            .is_some_and(|old| old != ty)
                        {
                            return Err(Error::Schema);
                        }
                    }
                }
            }
            data
        }
        Plan::NodeScan(alias, p)
        | Plan::EdgeScan(alias, p)
        | Plan::RowScan(alias, p)
        | Plan::AnnotationScan(alias, p) => {
            let kind = match plan {
                Plan::NodeScan(..) => graph::Kind::Node,
                Plan::EdgeScan(..) => graph::Kind::Edge,
                Plan::RowScan(..) => graph::Kind::Row,
                _ => graph::Kind::Annotation,
            };
            source(
                env,
                budget,
                graph::Source::Scan {
                    kind,
                    alias: alias.clone(),
                    predicate: p.clone(),
                },
            )?
        }
        Plan::HistoryScan {
            kind,
            alias,
            predicate,
            transactions,
            valid,
        } => source(
            env,
            budget,
            graph::Source::HistoryScan {
                kind: *kind,
                alias: alias.clone(),
                predicate: predicate.clone(),
                transactions: transactions.clone(),
                valid: valid.clone(),
            },
        )?,
        Plan::ChangeScan {
            alias,
            predicate,
            after,
            through,
        } => source(
            env,
            budget,
            graph::Source::ChangeScan {
                alias: alias.clone(),
                predicate: predicate.clone(),
                after: *after,
                through: *through,
            },
        )?,
        Plan::Match {
            start,
            predicate,
            expansion,
        } => {
            if start != &expansion.start_alias || expansion.optional {
                return Err(Error::Schema);
            }
            let mut data = source(
                env,
                budget,
                graph::Source::Scan {
                    kind: graph::Kind::Node,
                    alias: start.clone(),
                    predicate: predicate.clone(),
                },
            )?;
            if expansion.shortest {
                let endpoint = expansion
                    .segments
                    .last()
                    .ok_or(Error::Schema)?
                    .end_alias
                    .clone();
                if !data.columns.contains(&endpoint) {
                    let ends = source(
                        env,
                        budget,
                        graph::Source::Scan {
                            kind: graph::Kind::Node,
                            alias: endpoint.clone(),
                            predicate: graph::Predicate::default(),
                        },
                    )?;
                    let mut pairs = Vec::new();
                    for start in &data.rows {
                        for end in &ends.rows {
                            let mut pair = start.clone();
                            pair.extend(end.clone());
                            pairs.push(pair);
                            if pairs.len() > env.limits.max_rows {
                                return Err(Error::RowLimit);
                            }
                        }
                    }
                    data.rows = pairs;
                    data.columns.extend(ends.columns);
                    data.graph_schema.extend(ends.graph_schema);
                    data.types.extend(ends.types);
                }
            }
            transform(env, budget, data, graph::Stage::Expand(expansion.clone()))?
        }
        Plan::Expand(input, config) => transform(
            env,
            budget,
            run(input)?,
            graph::Stage::Expand(config.clone()),
        )?,
        Plan::AnnotationLookup(input, config) => transform(
            env,
            budget,
            run(input)?,
            graph::Stage::Annotations(config.clone()),
        )?,
        Plan::Filter(input, expr) => {
            let mut data = run(input)?;
            bind_expr(expr, &data.columns, 0)?;
            boolean_type(expression_type(expr, &data.types)?)?;
            data.rows = relational::execute(&relational::Plan::Filter(
                Box::new(relational::Plan::Values(data.rows)),
                expr.clone(),
            ))?;
            data
        }
        Plan::Project(input, fields) => {
            let mut data = run(input)?;
            for (_, expr) in fields {
                bind_expr(expr, &data.columns, 0)?;
            }
            data.rows = relational::execute(&relational::Plan::Project(
                Box::new(relational::Plan::Values(data.rows)),
                fields.clone(),
            ))?;
            project_schema(&mut data, fields)?;
            data
        }
        Plan::Distinct(input)
        | Plan::Take(input, _)
        | Plan::Offset(input, _)
        | Plan::Sort(input, _) => {
            let mut data = run(input)?;
            if let Plan::Sort(_, keys) = plan {
                for key in keys {
                    bind_expr(&key.expr, &data.columns, 0)?;
                    sortable_type(expression_type(&key.expr, &data.types)?)?;
                }
            }
            let input = Box::new(relational::Plan::Values(data.rows));
            let operation = match plan {
                Plan::Distinct(_) => relational::Plan::Distinct(input),
                Plan::Take(_, n) => relational::Plan::Take(input, *n),
                Plan::Offset(_, n) => relational::Plan::Offset(input, *n),
                Plan::Sort(_, keys) => relational::Plan::Sort(input, keys.clone()),
                _ => unreachable!(),
            };
            data.rows = relational::execute(&operation)?;
            data
        }
        Plan::UnionAll(a, b) => {
            let mut a = run(a)?;
            let b = run(b)?;
            if a.columns != b.columns
                || a.graph_schema != b.graph_schema
                || a.types != b.types
                || a.metadata.score_owners != b.metadata.score_owners
            {
                return Err(Error::Schema);
            }
            merge_metadata(&mut a.metadata, &b.metadata)?;
            a.rows.extend(b.rows);
            a
        }
        Plan::Join {
            left,
            right,
            on,
            kind,
            right_fields,
        } => {
            let mut a = run(left)?;
            let b = run(right)?;
            let mut joined_types = a.types.clone();
            joined_types.extend(b.types.clone());
            boolean_type(expression_type(on, &joined_types)?)?;
            bind_expr(on, &a.columns.union(&b.columns).cloned().collect(), 0)?;
            if a.columns.intersection(&b.columns).next().is_some()
                || right_fields.iter().cloned().collect::<BTreeSet<_>>() != b.columns
            {
                return Err(Error::Schema);
            }
            a.rows = relational::execute(&relational::Plan::Join {
                left: Box::new(relational::Plan::Values(a.rows)),
                right: Box::new(relational::Plan::Values(b.rows)),
                on: on.clone(),
                kind: *kind,
                right_fields: right_fields.clone(),
            })?;
            if !matches!(kind, JoinKind::Semi | JoinKind::Anti) {
                a.columns.extend(b.columns);
                a.graph_schema.extend(b.graph_schema);
                a.types = joined_types;
            }
            merge_metadata(&mut a.metadata, &b.metadata)?;
            a.metadata
                .score_owners
                .retain(|name, _| a.columns.contains(name));
            a
        }
        Plan::Aggregate {
            input,
            keys,
            aggregates,
        } => {
            let mut data = run(input)?;
            for (_, expr) in keys {
                bind_expr(expr, &data.columns, 0)?;
            }
            for (_, agg) in aggregates {
                match agg {
                    Aggregate::CountAll => {}
                    Aggregate::Count(e)
                    | Aggregate::Sum(e)
                    | Aggregate::Avg(e)
                    | Aggregate::Min(e)
                    | Aggregate::Max(e)
                    | Aggregate::Collect(e, _) => {
                        expression_type(e, &data.types)?;
                    }
                }
                match agg {
                    Aggregate::Sum(e) | Aggregate::Avg(e) => {
                        numeric_type(expression_type(e, &data.types)?)?
                    }
                    Aggregate::Min(e) | Aggregate::Max(e) => {
                        sortable_type(expression_type(e, &data.types)?)?
                    }
                    _ => {}
                }
                match agg {
                    Aggregate::CountAll => {}
                    Aggregate::Count(e)
                    | Aggregate::Sum(e)
                    | Aggregate::Avg(e)
                    | Aggregate::Min(e)
                    | Aggregate::Max(e)
                    | Aggregate::Collect(e, _) => bind_expr(e, &data.columns, 0)?,
                }
            }
            data.rows = relational::execute(&relational::Plan::Aggregate {
                input: Box::new(relational::Plan::Values(data.rows)),
                keys: keys.clone(),
                aggregates: aggregates.clone(),
            })?;
            let aggregate_types = aggregates
                .iter()
                .map(|(n, a)| {
                    Ok((
                        n.clone(),
                        match a {
                            Aggregate::CountAll | Aggregate::Count(_) => ValueType::I64,
                            Aggregate::Avg(_) => ValueType::F64,
                            Aggregate::Collect(_, _) => ValueType::List,
                            Aggregate::Sum(e) | Aggregate::Min(e) | Aggregate::Max(e) => {
                                expression_type(e, &data.types)?
                            }
                        },
                    ))
                })
                .collect::<Outcome<BTreeMap<_, _>>>()?;
            project_schema(&mut data, keys)?;
            data.types.extend(aggregate_types);
            data.columns
                .extend(aggregates.iter().map(|(n, _)| n.clone()));
            data
        }
        Plan::Knn {
            input,
            owner,
            output,
            request,
        }
        | Plan::Rerank {
            input,
            owner,
            output,
            request,
        } => {
            let data = run(input)?;
            let (mut batch, originals) = candidates(env, budget, &data, owner)?;
            let fixture = ranking_fixture(env, budget)?;
            if matches!(plan, Plan::Knn { .. }) {
                batch.rows.retain(|row| {
                    fixture
                        .documents
                        .iter()
                        .any(|d| d.id == row.owner_id && d.revision == row.revision)
                });
            }
            let result = if matches!(plan, Plan::Rerank { .. }) {
                rank::rerank(batch, &fixture, request).map_err(|e| {
                    if e == rank::Error::OwnerUnknown {
                        rank::Error::OriginalUnavailable
                    } else {
                        e
                    }
                })?
            } else {
                rank::knn(batch, &fixture, request)?
            };
            scored(data, result, originals, owner, output)?
        }
        Plan::LexicalMatch {
            input,
            owner,
            output,
            request,
        } => {
            let data = run(input)?;
            let (mut batch, originals) = candidates(env, budget, &data, owner)?;
            let fixture = ranking_fixture(env, budget)?;
            batch.rows.retain(|row| {
                fixture
                    .documents
                    .iter()
                    .any(|d| d.id == row.owner_id && d.revision == row.revision)
            });
            let result = rank::lexical_rank(batch, &fixture, request)?;
            scored(data, result, originals, owner, output)?
        }
        Plan::ContextPack {
            input,
            owner,
            output,
            tokenizer,
            budget: token_budget,
        } => {
            let mut data = run(input)?;
            if output.is_empty() {
                return Err(Error::Schema);
            }
            let (batch, _) = candidates(env, budget, &data, owner)?;
            let package = rank::pack_context(
                &batch,
                &ranking_fixture(env, budget)?,
                &env.tokenizers,
                tokenizer,
                *token_budget,
            )?;
            data.rows = vec![BTreeMap::from([(
                output.clone(),
                Value::Context(Box::new(package.clone())),
            )])];
            data.types = BTreeMap::from([(output.clone(), ValueType::Context)]);
            data.columns = BTreeSet::from([output.clone()]);
            data.graph_schema.clear();
            data.metadata.score_owners.clear();
            data
        }
    };
    if result.rows.len() > env.limits.max_rows {
        Err(Error::RowLimit)
    } else {
        Ok(result)
    }
}
fn graph_execute(
    env: &Environment,
    budget: &GraphBudget,
    plan: &graph::Plan,
    mut limits: graph::Limits,
) -> Outcome<graph::Relation> {
    limits.max_work = budget.get();
    let (out, used) = graph::execute_metered(&env.catalog, &env.view, plan, limits)?;
    budget.set(
        budget
            .get()
            .checked_sub(used)
            .ok_or(Error::Graph(graph::Error::BudgetExceeded))?,
    );
    Ok(out)
}
fn graph_transform(
    env: &Environment,
    budget: &GraphBudget,
    input: &graph::Relation,
    stages: &[graph::Stage],
    limits: graph::Limits,
) -> Outcome<graph::Relation> {
    graph_execute(
        env,
        budget,
        &graph::Plan {
            source: graph::Source::Input(input.clone()),
            stages: stages.to_vec(),
        },
        limits,
    )
}
