//! Closed, untrusted logical wire input. No catalog or execution authority.
use super::error::QueryErrorV2;
use crate::uee_v2::{QueryIrV2, QueryOpV2};
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const MAX_NODES: usize = 10_000;
const MAX_DEPTH: usize = 128;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LogicalRequestV2 {
    pub nodes: Vec<LogicalNodeV2>,
    pub root: String,
    pub parameter_types: BTreeMap<String, String>,
    // Set only by the in-process HQL lowering path, never by wire JSON.
    pub(crate) from_hql: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LogicalNodeV2 {
    pub id: String,
    pub op: QueryOpV2,
    pub inputs: Vec<String>,
    pub config: Config,
}

fn invalid(reason: &str) -> QueryErrorV2 {
    QueryErrorV2::new("BIND_ERROR", "bind", reason)
}

fn is_symbol(s: &str) -> bool {
    let mut chars = s.bytes();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn symbol<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    if is_symbol(&s) {
        Ok(s)
    } else {
        Err(serde::de::Error::custom("invalid_symbol"))
    }
}

fn name<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    if (1..=256).contains(&s.chars().count()) {
        Ok(s)
    } else {
        Err(serde::de::Error::custom("invalid_name"))
    }
}

fn profile_name<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let value = String::deserialize(d)?;
    let mut chars = value.bytes();
    if (1..=128).contains(&value.len())
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("invalid_profile_name"))
    }
}

// Optional means absent, never an explicit JSON null (schema string/expr).
fn optional_symbol<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    symbol(d).map(Some)
}
fn optional_name<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    name(d).map(Some)
}
fn optional_expr<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Expr>, D::Error> {
    Expr::deserialize(d).map(Some)
}
fn names<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let items = Vec::<String>::deserialize(d)?;
    if items.iter().all(|s| (1..=256).contains(&s.chars().count())) {
        Ok(items)
    } else {
        Err(serde::de::Error::custom("invalid_name"))
    }
}
fn symbols<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let items = Vec::<String>::deserialize(d)?;
    if items.iter().all(|s| is_symbol(s)) {
        Ok(items)
    } else {
        Err(serde::de::Error::custom("invalid_symbol"))
    }
}
fn decimal_u64<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let s = String::deserialize(d)?;
    let n: u64 = s
        .parse()
        .map_err(|_| serde::de::Error::custom("invalid_frontier"))?;
    if n.to_string() == s {
        Ok(n)
    } else {
        Err(serde::de::Error::custom("invalid_frontier"))
    }
}

macro_rules! wire_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name { $($variant),+ }
    };
}
wire_enum!(EntityKind {
    Node,
    Edge,
    Row,
    Vector,
    Annotation,
    Artifact
});
wire_enum!(Direction { Out, In, Both });
wire_enum!(PathMode {
    Trail,
    Simple,
    Walk
});
wire_enum!(KnnMode { Exact, Approx });
wire_enum!(Fidelity { Original });
wire_enum!(JoinKind {
    Inner,
    Left,
    Semi,
    Anti
});
wire_enum!(SortDirection { Asc, Desc });
wire_enum!(NullOrder { First, Last });
wire_enum!(SequenceForm { Sequence });
wire_enum!(BinaryOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Contains,
    Startswith
});
wire_enum!(UnaryOp {
    Not,
    Neg,
    IsNull,
    IsNotNull
});

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expr {
    Literal {
        literal: Value,
        ty: String,
    },
    Param {
        param: String,
    },
    Field {
        field: FieldRef,
    },
    Binary {
        binary: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        unary: UnaryOp,
        arg: Box<Expr>,
    },
    Call {
        call: String,
        args: Vec<Expr>,
    },
    In {
        expression: Box<Expr>,
        values: Vec<Expr>,
        negated: bool,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldRef {
    #[serde(deserialize_with = "symbol")]
    pub alias: String,
    #[serde(deserialize_with = "symbols")]
    pub path: Vec<String>,
}

// Dispatch expressions without serde's untagged backtracking, and build the
// recursive native tree iteratively. Literal JSON is a boundary payload only;
// its declared type and registered function semantics are resolved by bind_v2.
impl<'de> Deserialize<'de> for Expr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        decode_expression(Value::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

fn decode<T: DeserializeOwned>(value: Value) -> Result<T, &'static str> {
    serde_json::from_value(value).map_err(|_| "invalid_nested_shape")
}
fn take(map: &mut Map<String, Value>, key: &str) -> Result<Value, &'static str> {
    map.remove(key).ok_or("missing_expression_field")
}
fn take_symbol(map: &mut Map<String, Value>, key: &str) -> Result<String, &'static str> {
    let s: String = decode(take(map, key)?)?;
    if is_symbol(&s) {
        Ok(s)
    } else {
        Err("invalid_symbol")
    }
}

fn decode_expression(value: Value) -> Result<Expr, &'static str> {
    enum Task {
        Visit(Value),
        Binary(BinaryOp),
        Unary(UnaryOp),
        Call(String, usize),
        In(bool, usize),
    }
    let mut tasks = vec![Task::Visit(value)];
    let mut values = Vec::<Expr>::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Visit(Value::Object(mut map)) => {
                if map.contains_key("literal") {
                    let literal = take(&mut map, "literal")?;
                    let ty: String = decode(take(&mut map, "type")?)?;
                    if ty.is_empty() {
                        return Err("empty_literal_type");
                    }
                    values.push(Expr::Literal { literal, ty });
                } else if map.contains_key("param") {
                    values.push(Expr::Param {
                        param: take_symbol(&mut map, "param")?,
                    });
                } else if map.contains_key("field") {
                    values.push(Expr::Field {
                        field: decode(take(&mut map, "field")?)?,
                    });
                } else if map.contains_key("binary") {
                    let op = decode(take(&mut map, "binary")?)?;
                    let left = take(&mut map, "left")?;
                    let right = take(&mut map, "right")?;
                    tasks.extend([Task::Binary(op), Task::Visit(right), Task::Visit(left)]);
                } else if map.contains_key("unary") {
                    let op = decode(take(&mut map, "unary")?)?;
                    tasks.extend([Task::Unary(op), Task::Visit(take(&mut map, "arg")?)]);
                } else if map.contains_key("call") {
                    let call = take_symbol(&mut map, "call")?;
                    let args: Vec<Value> = decode(take(&mut map, "args")?)?;
                    tasks.push(Task::Call(call, args.len()));
                    tasks.extend(args.into_iter().rev().map(Task::Visit));
                } else if map.contains_key("in") {
                    let expression = take(&mut map, "in")?;
                    let args: Vec<Value> = decode(take(&mut map, "values")?)?;
                    let negated = decode(take(&mut map, "negated")?)?;
                    tasks.push(Task::In(negated, args.len()));
                    tasks.extend(args.into_iter().rev().map(Task::Visit));
                    tasks.push(Task::Visit(expression));
                } else {
                    return Err("unknown_expression");
                }
                if !map.is_empty() {
                    return Err("unknown_expression_field");
                }
            }
            Task::Visit(_) => return Err("expression_not_object"),
            Task::Binary(binary) => {
                let right = Box::new(values.pop().ok_or("expression_shape")?);
                let left = Box::new(values.pop().ok_or("expression_shape")?);
                values.push(Expr::Binary {
                    binary,
                    left,
                    right,
                });
            }
            Task::Unary(unary) => {
                let arg = Box::new(values.pop().ok_or("expression_shape")?);
                values.push(Expr::Unary { unary, arg });
            }
            Task::Call(call, count) => {
                let start = values.len().checked_sub(count).ok_or("expression_shape")?;
                let args = values.split_off(start);
                values.push(Expr::Call { call, args });
            }
            Task::In(negated, count) => {
                let start = values.len().checked_sub(count).ok_or("expression_shape")?;
                let args = values.split_off(start);
                let expression = Box::new(values.pop().ok_or("expression_shape")?);
                values.push(Expr::In {
                    expression,
                    values: args,
                    negated,
                });
            }
        }
    }
    if values.len() != 1 {
        return Err("expression_shape");
    }
    values.pop().ok_or("expression_shape")
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct NamedExpr {
    pub expression: Expr,
    #[serde(rename = "as", deserialize_with = "symbol")]
    pub alias: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SortKey {
    pub expression: Expr,
    pub direction: SortDirection,
    pub nulls: NullOrder,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub(crate) enum Pattern {
    Compact(CompactPattern),
    Sequence(SequencePattern),
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompactPattern {
    #[serde(deserialize_with = "symbol")]
    pub start_alias: String,
    #[serde(deserialize_with = "symbol")]
    pub end_alias: String,
    #[serde(default, deserialize_with = "optional_symbol")]
    pub edge_alias: Option<String>,
    #[serde(deserialize_with = "names")]
    pub relations: Vec<String>,
    pub direction: Direction,
    pub min_hops: u32,
    pub max_hops: u32,
    pub mode: PathMode,
    #[serde(default, deserialize_with = "optional_symbol")]
    pub path_alias: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SequencePattern {
    pub form: SequenceForm,
    pub start: PatternNode,
    pub steps: Vec<PatternStep>,
    pub mode: PathMode,
    #[serde(default, deserialize_with = "optional_symbol")]
    pub path_alias: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PatternNode {
    #[serde(deserialize_with = "symbol")]
    pub alias: String,
    #[serde(default, deserialize_with = "optional_expr")]
    pub id: Option<Expr>,
    #[serde(deserialize_with = "names")]
    pub labels: Vec<String>,
    pub properties: BTreeMap<String, Expr>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PatternEdge {
    #[serde(default, deserialize_with = "optional_symbol")]
    pub alias: Option<String>,
    #[serde(deserialize_with = "names")]
    pub relations: Vec<String>,
    pub direction: Direction,
    pub min_hops: u32,
    pub max_hops: u32,
    pub properties: BTreeMap<String, Expr>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PatternStep {
    pub edge: PatternEdge,
    pub node: PatternNode,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "op", content = "config", deny_unknown_fields)]
pub(crate) enum Config {
    NodeScan {
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
        #[serde(default, deserialize_with = "optional_name")]
        label: Option<String>,
    },
    EdgeScan {
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
        #[serde(default, deserialize_with = "optional_name")]
        relation: Option<String>,
    },
    RowScan {
        #[serde(deserialize_with = "symbol")]
        table: String,
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
    },
    AnnotationScan {
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
    },
    Values {
        #[serde(deserialize_with = "symbol")]
        param: String,
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
    },
    HistoryScan {
        kind: EntityKind,
        id: Expr,
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
    },
    ChangeScan {
        #[serde(deserialize_with = "decimal_u64")]
        after_seq: u64,
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
    },
    Match {
        pattern: Pattern,
        anchors: BTreeMap<String, Expr>,
        shortest: bool,
    },
    Filter {
        predicate: Expr,
    },
    Project {
        fields: Vec<NamedExpr>,
    },
    Distinct {},
    Knn {
        #[serde(deserialize_with = "symbol")]
        entity: String,
        #[serde(deserialize_with = "symbol")]
        collection: String,
        query: Expr,
        k: u32,
        mode: KnnMode,
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
    },
    Rerank {
        #[serde(deserialize_with = "symbol")]
        entity: String,
        #[serde(deserialize_with = "symbol")]
        collection: String,
        query: Expr,
        k: u32,
        fidelity: Fidelity,
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
    },
    Expand {
        pattern: Pattern,
        optional: bool,
    },
    AnnotationLookup {
        #[serde(deserialize_with = "symbol")]
        target: String,
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
        optional: bool,
    },
    LexicalMatch {
        #[serde(deserialize_with = "symbol")]
        entity: String,
        #[serde(deserialize_with = "symbol")]
        field: String,
        #[serde(deserialize_with = "profile_name")]
        index: String,
        query: Expr,
        k: u32,
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
    },
    Join {
        kind: JoinKind,
        condition: Expr,
    },
    Aggregate {
        group_by: Vec<NamedExpr>,
        aggregates: Vec<NamedExpr>,
    },
    Sort {
        keys: Vec<SortKey>,
    },
    Take {
        count: u64,
    },
    Offset {
        count: u64,
    },
    UnionAll {},
    ContextPack {
        text: Expr,
        evidence: Expr,
        tokens: u32,
        #[serde(deserialize_with = "name")]
        tokenizer: String,
        #[serde(rename = "as", deserialize_with = "symbol")]
        alias: String,
    },
}

fn arity(op: &QueryOpV2) -> usize {
    use QueryOpV2::*;
    match op {
        NodeScan | EdgeScan | RowScan | AnnotationScan | Values | HistoryScan | ChangeScan
        | Match => 0,
        Join | UnionAll => 2,
        _ => 1,
    }
}

impl Config {
    fn validate(&self) -> Result<(), QueryErrorV2> {
        match self {
            Self::Project { fields } if fields.is_empty() => {
                return Err(invalid("empty_projection"))
            }
            Self::Aggregate { aggregates, .. } if aggregates.is_empty() => {
                return Err(invalid("empty_aggregates"))
            }
            Self::Sort { keys } if keys.is_empty() => return Err(invalid("empty_sort")),
            Self::Knn { k, .. } | Self::Rerank { k, .. } | Self::LexicalMatch { k, .. }
                if *k > 10_000 =>
            {
                return Err(invalid("ranking_bound"))
            }
            Self::ContextPack { tokens, .. } if *tokens > 1_000_000 => {
                return Err(invalid("token_bound"))
            }
            Self::Match { anchors, .. } if anchors.keys().any(|s| !is_symbol(s)) => {
                return Err(invalid("invalid_anchor"))
            }
            _ => {}
        }
        if let Self::Match { pattern, .. } | Self::Expand { pattern, .. } = self {
            let valid_hops = |min: u32, max: u32| min <= max && max <= 32;
            match pattern {
                Pattern::Compact(p) if !valid_hops(p.min_hops, p.max_hops) => {
                    return Err(invalid("hop_bounds"))
                }
                Pattern::Sequence(p)
                    if !(1..=32).contains(&p.steps.len())
                        || p.steps
                            .iter()
                            .any(|s| !valid_hops(s.edge.min_hops, s.edge.max_hops)) =>
                {
                    return Err(invalid("pattern_bounds"))
                }
                _ => {}
            }
        }
        Ok(())
    }
}

// Check arbitrary JSON nesting as well as expression nesting before recursive
// serde sees a config. JSON wrappers (args arrays, field objects, pattern nodes)
// do not consume the expression depth budget. Literal payloads are not Exprs.
fn preflight_config(op: &QueryOpV2, config: &BTreeMap<String, Value>) -> Result<(), QueryErrorV2> {
    let mut json = config.values().map(|v| (v, 1usize)).collect::<Vec<_>>();
    while let Some((value, depth)) = json.pop() {
        if depth > MAX_DEPTH * 4 {
            return Err(invalid("json_depth"));
        }
        match value {
            Value::Object(map) => json.extend(map.values().map(|v| (v, depth + 1))),
            Value::Array(items) => json.extend(items.iter().map(|v| (v, depth + 1))),
            _ => {}
        }
    }
    let mut roots = Vec::new();
    use QueryOpV2::*;
    let fields: &[&str] = match op {
        HistoryScan => &["id"],
        Filter => &["predicate"],
        Knn | Rerank | LexicalMatch => &["query"],
        Join => &["condition"],
        ContextPack => &["text", "evidence"],
        _ => &[],
    };
    roots.extend(fields.iter().filter_map(|key| config.get(*key)));
    for key in ["fields", "group_by", "aggregates", "keys"] {
        if let Some(Value::Array(items)) = config.get(key) {
            roots.extend(items.iter().filter_map(|v| v.get("expression")));
        }
    }
    if let Some(Value::Object(anchors)) = config.get("anchors") {
        roots.extend(anchors.values());
    }
    if let Some(pattern) = config.get("pattern") {
        let mut parts = Vec::new();
        if let Some(start) = pattern.get("start") {
            parts.push(start);
        }
        if let Some(Value::Array(steps)) = pattern.get("steps") {
            for step in steps {
                parts.extend([step.get("node"), step.get("edge")].into_iter().flatten());
            }
        }
        for part in parts {
            if let Some(id) = part.get("id") {
                roots.push(id);
            }
            if let Some(Value::Object(properties)) = part.get("properties") {
                roots.extend(properties.values());
            }
        }
    }
    let mut expressions = roots.into_iter().map(|v| (v, 1usize)).collect::<Vec<_>>();
    while let Some((value, depth)) = expressions.pop() {
        if depth > MAX_DEPTH {
            return Err(invalid("expression_depth"));
        }
        if let Value::Object(map) = value {
            // Even invalid mixed discriminators are bounded before rejection.
            for key in ["left", "right", "arg", "in"] {
                if let Some(child) = map.get(key) {
                    expressions.push((child, depth + 1));
                }
            }
            for key in ["args", "values"] {
                if let Some(Value::Array(items)) = map.get(key) {
                    expressions.extend(items.iter().map(|v| (v, depth + 1)));
                }
            }
        }
    }
    Ok(())
}

// A caller may construct arbitrarily deep Values without the JSON parser.
// Dispose rejected raw JSON iteratively too; a recursive drop is not a refusal.
fn discard_raw(ir: QueryIrV2) {
    let mut values = ir
        .nodes
        .into_iter()
        .flat_map(|n| n.config.into_values())
        .collect::<Vec<_>>();
    while let Some(value) = values.pop() {
        match value {
            Value::Array(items) => values.extend(items),
            Value::Object(map) => values.extend(map.into_values()),
            _ => {}
        }
    }
}

fn validate_dag(ir: &QueryIrV2) -> Result<Vec<usize>, QueryErrorV2> {
    if ir.contract_version != "query-ir.v2" {
        return Err(QueryErrorV2::new(
            "VERSION_UNSUPPORTED",
            "contract",
            "query_ir_version",
        ));
    }
    if ir.nodes.is_empty() || ir.nodes.len() > MAX_NODES {
        return Err(invalid("node_count"));
    }
    if !is_symbol(&ir.root)
        || ir
            .parameter_types
            .iter()
            .any(|(key, ty)| !is_symbol(key) || ty.is_empty())
    {
        return Err(invalid("invalid_declaration"));
    }
    let mut ids = BTreeMap::new();
    for (index, node) in ir.nodes.iter().enumerate() {
        if !is_symbol(&node.id) || ids.insert(node.id.as_str(), index).is_some() {
            return Err(invalid("duplicate_or_invalid_node"));
        }
        if node.inputs.len() != arity(&node.op) {
            return Err(invalid("input_arity"));
        }
    }
    let root = *ids
        .get(ir.root.as_str())
        .ok_or_else(|| invalid("missing_root"))?;
    let mut indegree = vec![0; ir.nodes.len()];
    let mut outgoing = vec![Vec::new(); ir.nodes.len()];
    for (index, node) in ir.nodes.iter().enumerate() {
        indegree[index] = node.inputs.len();
        for input in &node.inputs {
            let parent = *ids
                .get(input.as_str())
                .ok_or_else(|| invalid("missing_input"))?;
            outgoing[parent].push(index);
        }
    }
    let mut reachable = vec![false; ir.nodes.len()];
    let mut pending = vec![root];
    while let Some(index) = pending.pop() {
        if std::mem::replace(&mut reachable[index], true) {
            continue;
        }
        pending.extend(ir.nodes[index].inputs.iter().map(|id| ids[id.as_str()]));
    }
    if reachable.iter().any(|v| !v) {
        return Err(invalid("orphan_node"));
    }
    let mut ready = ids
        .iter()
        .filter_map(|(id, index)| (indegree[*index] == 0).then_some(*id))
        .collect::<BTreeSet<_>>();
    let mut depths = vec![1usize; ir.nodes.len()];
    let mut order = Vec::with_capacity(ir.nodes.len());
    while let Some(id) = ready.pop_first() {
        let index = ids[id];
        if depths[index] > MAX_DEPTH {
            return Err(invalid("dag_depth"));
        }
        order.push(index);
        for &child in &outgoing[index] {
            depths[child] = depths[child].max(depths[index] + 1);
            indegree[child] -= 1;
            if indegree[child] == 0 {
                ready.insert(ir.nodes[child].id.as_str());
            }
        }
    }
    if order.len() != ir.nodes.len() {
        return Err(invalid("cycle"));
    }
    Ok(order)
}

pub(crate) fn decode_ir_v2(ir: QueryIrV2) -> Result<LogicalRequestV2, QueryErrorV2> {
    let checked = (|| {
        let order = validate_dag(&ir)?;
        for node in &ir.nodes {
            preflight_config(&node.op, &node.config)?;
        }
        Ok::<_, QueryErrorV2>(order)
    })();
    let order = match checked {
        Ok(order) => order,
        Err(error) => {
            discard_raw(ir);
            return Err(error);
        }
    };
    let mut nodes = ir.nodes.into_iter().map(Some).collect::<Vec<_>>();
    let mut typed = Vec::with_capacity(nodes.len());
    for index in order {
        let node = nodes[index].take().ok_or_else(|| invalid("node_order"))?;
        let mut tagged = Map::new();
        tagged.insert(
            "op".into(),
            serde_json::to_value(&node.op).map_err(|_| invalid("operator"))?,
        );
        tagged.insert(
            "config".into(),
            Value::Object(node.config.into_iter().collect()),
        );
        let config: Config =
            serde_json::from_value(Value::Object(tagged)).map_err(|_| invalid("closed_config"))?;
        config.validate()?;
        typed.push(LogicalNodeV2 {
            id: node.id,
            op: node.op,
            inputs: node.inputs,
            config,
        });
    }
    Ok(LogicalRequestV2 {
        nodes: typed,
        root: ir.root,
        parameter_types: ir.parameter_types,
        from_hql: false,
    })
}
