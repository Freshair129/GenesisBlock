//! Source syntax, not trusted/bound execution plans. All offsets are UTF-8 bytes.

use std::collections::BTreeMap;

pub use super::syntax::Rule as SyntaxKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spanned<T> {
    pub span: Span,
    pub value: T,
}

pub type Name = Spanned<String>;
pub type Expr = Spanned<ExprKind>;
pub type Source = Spanned<SourceKind>;
pub type Stage = Spanned<StageKind>;
pub type SelectItem = Spanned<SelectKind>;
pub type Unsigned = Spanned<UnsignedKind>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatementKind {
    Read,
    Explain,
    AnalyzeRead,
    Mutation,
    Ddl,
    Admin,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hql2Statement {
    pub source: String,
    pub span: Span,
    pub kind: StatementKind,
    pub query: Option<Query>,
    /// Complete grammar-checked syntax, including currently unsupported families.
    pub syntax: SyntaxNode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxNode {
    /// Pinned grammar production (or keyword terminal), not an execution opcode.
    pub kind: SyntaxKind,
    pub span: Span,
    /// Only leaves retain source text; interior nodes refer to their children.
    pub text: Option<String>,
    pub children: Vec<SyntaxNode>,
}

impl Drop for SyntaxNode {
    fn drop(&mut self) {
        // Grammar wrapper depth exceeds expression depth. Drain descendants on
        // the heap so returning a legal AST to a small-stack caller stays safe.
        let mut pending = std::mem::take(&mut self.children);
        while let Some(mut node) = pending.pop() {
            pending.append(&mut node.children);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Query {
    pub span: Span,
    pub scope: Option<Scope>,
    pub source: Source,
    pub stages: Vec<Stage>,
    pub returning: Vec<SelectItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub span: Span,
    pub namespace: Name,
    pub tx: Option<Unsigned>,
    pub valid_at: Option<Spanned<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SourceKind {
    Values {
        parameter: Name,
        alias: Name,
    },
    Nodes {
        label: Option<Name>,
        alias: Name,
    },
    Edges {
        relation: Option<Name>,
        alias: Name,
    },
    History {
        kind: Spanned<String>,
        id: Expr,
        alias: Name,
    },
    Changes {
        after_seq: Unsigned,
        alias: Name,
    },
    Rows {
        table: Name,
        alias: Name,
    },
    Annotations {
        alias: Name,
    },
    Match {
        start_alias: Name,
        start_id: Option<Expr>,
        start_labels: Vec<Name>,
        start_properties: BTreeMap<String, Expr>,
        steps: Vec<GraphSequenceStep>,
        mode: GraphPathMode,
        path_alias: Option<Name>,
        shortest: bool,
    },
    UnionAll {
        left: Box<Query>,
        right: Box<Query>,
    },
    /// Fully parsed; lowering must explicitly implement or reject this family.
    Unsupported(SyntaxNode),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StageKind {
    Filter(Expr),
    Project(Vec<SelectItem>),
    Distinct,
    Join {
        table: Name,
        alias: Name,
        kind: JoinKind,
        condition: Expr,
    },
    AnnotationLookup {
        target: Name,
        alias: Name,
        optional: bool,
    },
    Knn {
        entity: Name,
        collection: Name,
        query: Expr,
        k: Unsigned,
        mode: VectorSearchMode,
        alias: Name,
    },
    Rerank {
        entity: Name,
        collection: Name,
        query: Expr,
        k: Unsigned,
        alias: Name,
    },
    LexicalMatch {
        entity: Name,
        field: Name,
        query: Expr,
        index: Name,
        k: Unsigned,
        alias: Name,
    },
    ContextPack {
        text: Expr,
        evidence: Expr,
        tokens: Unsigned,
        tokenizer: Spanned<String>,
        alias: Name,
    },
    Expand {
        start_alias: Name,
        end_alias: Name,
        edge_alias: Option<Name>,
        relations: Vec<Name>,
        direction: GraphDirection,
        min_hops: u32,
        max_hops: u32,
        mode: GraphPathMode,
        path_alias: Option<Name>,
        optional: bool,
    },
    ExpandSequence {
        start_alias: Name,
        start_id: Option<Expr>,
        start_labels: Vec<Name>,
        start_properties: BTreeMap<String, Expr>,
        steps: Vec<GraphSequenceStep>,
        mode: GraphPathMode,
        path_alias: Option<Name>,
        optional: bool,
    },
    Aggregate {
        group_by: Vec<SelectItem>,
        aggregates: Vec<SelectItem>,
    },
    Order(Vec<OrderItem>),
    Take(Unsigned),
    Skip(Unsigned),
    Unsupported(SyntaxNode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Semi,
    Anti,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorSearchMode {
    Exact,
    Approx,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphDirection {
    Out,
    In,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphPathMode {
    Trail,
    Simple,
    Walk,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphSequenceStep {
    pub end_alias: Name,
    pub node_id: Option<Expr>,
    pub node_labels: Vec<Name>,
    pub node_properties: BTreeMap<String, Expr>,
    pub edge_alias: Option<Name>,
    pub edge_properties: BTreeMap<String, Expr>,
    pub relations: Vec<Name>,
    pub direction: GraphDirection,
    pub min_hops: u32,
    pub max_hops: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectKind {
    Star,
    Expression {
        expression: Expr,
        alias: Option<Name>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderItem {
    pub span: Span,
    pub expression: Expr,
    /// None retains omission for the binder to apply the language default.
    pub direction: Option<Spanned<SortDirection>>,
    pub nulls: Option<Spanned<NullOrder>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NullOrder {
    First,
    Last,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnsignedKind {
    Literal(u64),
    Parameter(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    Utf8(String),
    Parameter(String),
    Field(Vec<Name>),
    List(Vec<Expr>),
    Object(Vec<ObjectField>),
    Call {
        name: Name,
        arguments: Vec<Expr>,
        star: Option<Span>,
    },
    Group(Box<Expr>),
    Not(Box<Expr>),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    IsNull {
        expression: Box<Expr>,
        negated: bool,
    },
    In {
        expression: Box<Expr>,
        values: Box<Expr>,
        negated: bool,
    },
    Between {
        expression: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectField {
    pub span: Span,
    pub key: Name,
    pub expression: Expr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Contains,
    StartsWith,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}
