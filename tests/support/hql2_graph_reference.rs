//! P7 fixture-only graph/temporal/annotation oracle. No engine imports.
//! Contract: ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY (accepted 0.1.0b).
//! This module is intentionally usable by standalone rustc integration tests.
//!
//! Ordering: scans use EntityRef order; history/changes use (tx_from, EntityRef).
//! Transforms preserve input bag order, then sort paths by (hop count, edge
//! references, vertices, bindings), or annotations by EntityRef. OPTIONAL applies
//! to the entire compound pattern. Uniqueness applies across all its segments.
//! Input relations carry only domain bindings; adapters own other scalar columns.
//! All sources/stages validate before reading rows, including empty input.
//! Limits fail the whole call, never return a truncated successful relation.
//! History is explicit fixture data, not reconstruction from current records.
//! ChangeScan requires RecordData::Change events; it does not invent a WAL feed.
//! No persistence, parser, production binder, lease, or policy service is implied.
//! Authorization follows target/subject dependencies with cycle detection and
//! depth <= 128. A cycle alone grants nothing; an independently readable target
//! can authorize an annotation. Dynamic capabilities follow selected dependencies.
//! Imported paths validate adjacent edge endpoints in either orientation.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Node,
    Edge,
    Row,
    Vector,
    Annotation,
    Artifact,
    Event,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identity {
    pub namespace: String,
    pub kind: Kind,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityRef {
    pub namespace: String,
    pub kind: Kind,
    pub id: String,
    pub revision: String,
}
impl EntityRef {
    pub fn identity(&self) -> Identity {
        Identity {
            namespace: self.namespace.clone(),
            kind: self.kind,
            id: self.id.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interval<T> {
    pub start: T,
    pub end: Option<T>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scalar {
    Text(String),
    Integer(i64),
    Boolean(bool),
    #[allow(dead_code)]
    Json(serde_json::Value),
}
pub type Fields = BTreeMap<String, Scalar>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Predicate {
    pub id: Option<String>,
    pub equals: Fields,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextSource {
    pub text: String,
    pub extraction_id: String,
    pub normalization_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetBinding {
    Frozen(EntityRef),
    Live(Identity),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Selector {
    Whole,
    /// Half-open offsets in Unicode scalar values, not bytes or UTF-16 units.
    TextPosition {
        start: usize,
        end: usize,
        source_hash: String,
        extraction_id: String,
        normalization_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Target {
    pub binding: TargetBinding,
    pub selector: Selector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Upsert,
    Retract,
    Correct,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordData {
    Plain,
    Edge {
        source: Identity,
        target: Identity,
        relation: String,
    },
    Text(TextSource),
    Annotation {
        targets: Vec<Target>,
    },
    /// A durable vector record; its owner identity supplies the P6 read ACL.
    Vector {
        owner: Identity,
    },
    /// An explicit fixture event; transaction.start is its local commit sequence.
    Change {
        subject: EntityRef,
        operation: ChangeKind,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    pub entity: EntityRef,
    pub transaction: Interval<u64>,
    pub valid: Interval<i64>,
    pub retracted: bool,
    pub fields: Fields,
    pub data: RecordData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryCapability {
    pub horizon: u64,
    pub available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    pub frontier: u64,
    pub history: BTreeMap<Kind, HistoryCapability>,
    pub revisions: Vec<Revision>,
}

/// Current policy, including for historical reads. Empty sets deny everything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Permissions {
    pub read: BTreeSet<Identity>,
    pub annotation_body: BTreeSet<Identity>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub namespace: String,
    pub transaction: u64,
    pub valid_at: i64,
    pub permissions: Permissions,
}

pub type Fixture = Catalog;
pub type Context = View;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path {
    pub vertices: Vec<EntityRef>,
    pub edges: Vec<EntityRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Binding {
    Null,
    Entity(EntityRef),
    Edges(Vec<EntityRef>),
    Path(Path),
}
pub type Row = BTreeMap<String, Binding>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingType {
    Entity(Kind),
    Edges,
    Path,
}
pub type Schema = BTreeMap<String, BindingType>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Relation {
    pub schema: Schema,
    pub rows: Vec<Row>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Out,
    In,
    Both,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathMode {
    Trail,
    Simple,
    Walk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub end_alias: String,
    pub edge_alias: Option<String>,
    pub direction: Direction,
    /// Empty means every relation; otherwise exact case-sensitive membership.
    pub relations: BTreeSet<String>,
    pub min_hops: usize,
    pub max_hops: usize,
    pub node_predicate: Predicate,
    pub edge_predicate: Predicate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expand {
    pub start_alias: String,
    pub segments: Vec<Segment>,
    pub path_alias: Option<String>,
    pub mode: PathMode,
    pub optional: bool,
    /// Final endpoint alias must already be bound by the input schema.
    pub shortest: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotationLookup {
    pub target_alias: String,
    pub alias: String,
    pub predicate: Predicate,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    Expand(Expand),
    Annotations(AnnotationLookup),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Scan {
        kind: Kind,
        alias: String,
        predicate: Predicate,
    },
    /// All retained revisions intersecting BOTH half-open windows, known at S.
    /// Includes tombstones; ordering is (transaction.start, entity reference).
    HistoryScan {
        kind: Kind,
        alias: String,
        predicate: Predicate,
        transactions: Interval<u64>,
        valid: Interval<i64>,
    },
    /// Explicit Change event revisions in (after, through], capped by S.
    /// Binds Event references, ordered by (commit sequence, event identity).
    ChangeScan {
        alias: String,
        predicate: Predicate,
        after: u64,
        through: u64,
    },
    /// Integration seam: explicit schema is required even when rows are empty.
    Input(Relation),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub source: Source,
    pub stages: Vec<Stage>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_hops: usize,
    pub max_work: usize,
    pub max_rows: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_hops: 8,
            max_work: 100_000,
            max_rows: 10_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidBounds,
    InvalidInterval,
    InvalidCatalog,
    NamespaceMismatch,
    Alias(String),
    Type(String),
    BeyondHorizon(Kind),
    HistoryUnavailable(Kind),
    FutureSnapshot,
    InvalidSource,
    InvalidSelector,
    MissingRevision,
    BudgetExceeded,
}

impl<T: Ord> Interval<T> {
    pub fn contains(&self, value: &T) -> bool {
        &self.start <= value && self.end.as_ref().is_none_or(|end| value < end)
    }
    pub fn overlaps(&self, other: &Self) -> bool {
        self.end.as_ref().is_none_or(|end| &other.start < end)
            && other.end.as_ref().is_none_or(|end| &self.start < end)
    }
    fn validate(&self) -> Result<(), Error> {
        if self.end.as_ref().is_some_and(|end| end <= &self.start) {
            Err(Error::InvalidInterval)
        } else {
            Ok(())
        }
    }
}

impl Predicate {
    fn matches(&self, revision: &Revision) -> bool {
        self.id.as_ref().is_none_or(|id| id == &revision.entity.id)
            && self
                .equals
                .iter()
                .all(|(k, v)| revision.fields.get(k) == Some(v))
    }
}

impl TargetBinding {
    fn identity(&self) -> Identity {
        match self {
            Self::Frozen(r) => r.identity(),
            Self::Live(id) => id.clone(),
        }
    }
}

fn add_alias(schema: &mut Schema, alias: &str, ty: BindingType, reuse: bool) -> Result<(), Error> {
    if alias.is_empty() {
        return Err(Error::Alias(alias.into()));
    }
    if let Some(old) = schema.get(alias) {
        if !reuse || *old != ty {
            return Err(Error::Alias(alias.into()));
        }
    } else {
        schema.insert(alias.into(), ty);
    }
    Ok(())
}

fn entity_alias(schema: &Schema, alias: &str) -> Result<Kind, Error> {
    match schema.get(alias) {
        Some(BindingType::Entity(kind)) => Ok(*kind),
        Some(_) => Err(Error::Type(alias.into())),
        None => Err(Error::Alias(alias.into())),
    }
}

struct Runtime<'a> {
    catalog: &'a Catalog,
    view: &'a View,
    limits: Limits,
    work: usize,
}

impl<'a> Runtime<'a> {
    fn tick(&mut self) -> Result<(), Error> {
        self.work = self.work.checked_add(1).ok_or(Error::BudgetExceeded)?;
        if self.work > self.limits.max_work {
            Err(Error::BudgetExceeded)
        } else {
            Ok(())
        }
    }
    fn push<T>(&mut self, rows: &mut Vec<T>, row: T) -> Result<(), Error> {
        self.tick()?;
        if rows.len() >= self.limits.max_rows {
            return Err(Error::BudgetExceeded);
        }
        rows.push(row);
        Ok(())
    }
    fn capability(&self, kind: Kind, since: u64, historical: bool) -> Result<(), Error> {
        let capability = self
            .catalog
            .history
            .get(&kind)
            .ok_or(Error::HistoryUnavailable(kind))?;
        if since < capability.horizon {
            return Err(Error::BeyondHorizon(kind));
        }
        if historical && !capability.available {
            return Err(Error::HistoryUnavailable(kind));
        }
        Ok(())
    }
    fn dependencies(
        &mut self,
        kind: Kind,
        since: u64,
        historical: bool,
        seen: &mut BTreeSet<Kind>,
    ) -> Result<(), Error> {
        if !seen.insert(kind) {
            return Ok(());
        }
        self.capability(kind, since, historical)?;
        if kind == Kind::Edge {
            self.dependencies(Kind::Node, since, historical, seen)?;
        }
        // Only structural dependencies are known before row selection. Annotation
        // targets/event subjects are checked when a selected readable record uses
        // them, not by scanning future, denied or out-of-window catalog records.
        Ok(())
    }
    fn require(&mut self, kind: Kind) -> Result<(), Error> {
        self.dependencies(
            kind,
            self.view.transaction,
            self.view.transaction < self.catalog.frontier,
            &mut BTreeSet::new(),
        )
    }
    fn preflight(&mut self, plan: &Plan) -> Result<Vec<Schema>, Error> {
        if self.view.transaction > self.catalog.frontier {
            return Err(Error::FutureSnapshot);
        }
        let mut schema = Schema::new();
        match &plan.source {
            Source::Scan { kind, alias, .. } => {
                if *kind == Kind::Vector {
                    return Err(Error::InvalidSource);
                }
                self.require(*kind)?;
                add_alias(&mut schema, alias, BindingType::Entity(*kind), false)?;
            }
            Source::Input(input) => {
                for (alias, ty) in &input.schema {
                    add_alias(&mut schema, alias, *ty, false)?;
                    match ty {
                        BindingType::Entity(kind) => self.require(*kind)?,
                        BindingType::Edges | BindingType::Path => self.require(Kind::Edge)?,
                    }
                }
            }
            Source::HistoryScan {
                kind,
                alias,
                transactions,
                valid,
                ..
            } => {
                transactions.validate()?;
                valid.validate()?;
                self.dependencies(*kind, transactions.start, true, &mut BTreeSet::new())?;
                self.require(*kind)?;
                add_alias(&mut schema, alias, BindingType::Entity(*kind), false)?;
            }
            Source::ChangeScan {
                alias,
                after,
                through,
                ..
            } => {
                if after > through {
                    return Err(Error::InvalidBounds);
                }
                // The exclusive starting cursor must itself be within retention.
                self.dependencies(Kind::Event, *after, true, &mut BTreeSet::new())?;
                self.require(Kind::Event)?;
                add_alias(&mut schema, alias, BindingType::Entity(Kind::Event), false)?;
            }
        }
        let mut schemas = vec![schema.clone()];
        for stage in &plan.stages {
            self.tick()?;
            match stage {
                Stage::Annotations(a) => {
                    let kind = entity_alias(&schema, &a.target_alias)?;
                    self.require(kind)?;
                    self.require(Kind::Annotation)?;
                    add_alias(
                        &mut schema,
                        &a.alias,
                        BindingType::Entity(Kind::Annotation),
                        false,
                    )?;
                }
                Stage::Expand(x) => {
                    if entity_alias(&schema, &x.start_alias)? != Kind::Node {
                        return Err(Error::Type(x.start_alias.clone()));
                    }
                    self.require(Kind::Node)?;
                    self.require(Kind::Edge)?;
                    if x.segments.is_empty() {
                        return Err(Error::InvalidBounds);
                    }
                    if x.shortest {
                        let end = &x.segments.last().unwrap().end_alias;
                        if entity_alias(&schema, end)? != Kind::Node {
                            return Err(Error::Type(end.clone()));
                        }
                    }
                    let mut total = 0usize;
                    for segment in &x.segments {
                        if segment.min_hops > segment.max_hops {
                            return Err(Error::InvalidBounds);
                        }
                        total = total
                            .checked_add(segment.max_hops)
                            .ok_or(Error::InvalidBounds)?;
                        if total > self.limits.max_hops {
                            return Err(Error::InvalidBounds);
                        }
                        add_alias(
                            &mut schema,
                            &segment.end_alias,
                            BindingType::Entity(Kind::Node),
                            true,
                        )?;
                        if let Some(alias) = &segment.edge_alias {
                            let ty = if segment.min_hops == 1 && segment.max_hops == 1 {
                                BindingType::Entity(Kind::Edge)
                            } else {
                                BindingType::Edges
                            };
                            add_alias(&mut schema, alias, ty, false)?;
                        }
                    }
                    if let Some(alias) = &x.path_alias {
                        add_alias(&mut schema, alias, BindingType::Path, false)?;
                    }
                }
            }
            schemas.push(schema.clone());
        }
        Ok(schemas)
    }

    fn validate_catalog(&mut self) -> Result<(), Error> {
        let mut references = BTreeSet::new();
        for (i, r) in self.catalog.revisions.iter().enumerate() {
            self.tick()?;
            r.transaction.validate()?;
            r.valid.validate()?;
            if r.entity.namespace.is_empty()
                || r.entity.id.is_empty()
                || r.entity.id.len() > 256
                || r.entity.revision.is_empty()
                || !references.insert(r.entity.clone())
            {
                return Err(Error::InvalidCatalog);
            }
            for earlier in &self.catalog.revisions[..i] {
                self.tick()?;
                if earlier.entity.identity() == r.entity.identity()
                    && earlier.transaction.overlaps(&r.transaction)
                    && earlier.valid.overlaps(&r.valid)
                {
                    return Err(Error::InvalidCatalog);
                }
            }
            match &r.data {
                RecordData::Edge { source, target, .. } => {
                    if r.entity.kind != Kind::Edge
                        || source.kind != Kind::Node
                        || target.kind != Kind::Node
                    {
                        return Err(Error::InvalidCatalog);
                    }
                    if source.namespace != r.entity.namespace
                        || target.namespace != r.entity.namespace
                    {
                        return Err(Error::NamespaceMismatch);
                    }
                }
                RecordData::Annotation { targets } => {
                    if r.entity.kind != Kind::Annotation || targets.is_empty() {
                        return Err(Error::InvalidCatalog);
                    }
                    let mut seen = BTreeSet::new();
                    for target in targets {
                        self.tick()?;
                        if target.binding.identity().namespace != r.entity.namespace {
                            return Err(Error::NamespaceMismatch);
                        }
                        // Duplicate identity includes selector: disjoint spans on one
                        // revision are distinct targets, but still emit one annotation.
                        if !seen.insert(target.clone()) {
                            return Err(Error::InvalidCatalog);
                        }
                        if let TargetBinding::Frozen(reference) = &target.binding {
                            let source = self.find(reference)?.ok_or(Error::MissingRevision)?;
                            if source.retracted || source.transaction.start > r.transaction.start {
                                return Err(Error::MissingRevision);
                            }
                            validate_target_selector(source, &target.selector)?;
                        } else if let Selector::TextPosition { start, end, .. } = &target.selector {
                            if start > end {
                                return Err(Error::InvalidSelector);
                            }
                        }
                    }
                }
                RecordData::Vector { owner } => {
                    if r.entity.kind != Kind::Vector || owner.kind != Kind::Node {
                        return Err(Error::InvalidCatalog);
                    }
                    if owner.namespace != r.entity.namespace {
                        return Err(Error::NamespaceMismatch);
                    }
                    if owner.id.is_empty() {
                        return Err(Error::InvalidCatalog);
                    }
                }
                RecordData::Change { subject, .. } => {
                    if r.entity.kind != Kind::Event {
                        return Err(Error::InvalidCatalog);
                    }
                    if subject.namespace != r.entity.namespace {
                        return Err(Error::NamespaceMismatch);
                    }
                    let source = self.find(subject)?.ok_or(Error::MissingRevision)?;
                    if source.transaction.start > r.transaction.start {
                        return Err(Error::InvalidCatalog);
                    }
                }
                _ if r.entity.kind == Kind::Edge
                    || r.entity.kind == Kind::Vector
                    || r.entity.kind == Kind::Annotation =>
                {
                    return Err(Error::InvalidCatalog)
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn find(&mut self, reference: &EntityRef) -> Result<Option<&'a Revision>, Error> {
        for r in &self.catalog.revisions {
            self.tick()?;
            if &r.entity == reference {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }
    fn visible(&self, r: &Revision) -> bool {
        r.entity.namespace == self.view.namespace
            && !r.retracted
            && r.transaction.contains(&self.view.transaction)
            && r.valid.contains(&self.view.valid_at)
    }
    fn permitted(&self, id: &Identity) -> bool {
        id.namespace == self.view.namespace
            && self.view.permissions.read.contains(id)
            && (id.kind != Kind::Annotation || self.view.permissions.annotation_body.contains(id))
    }
    fn resolve(&mut self, id: &Identity) -> Result<Option<&'a Revision>, Error> {
        if !self.permitted(id) {
            return Ok(None);
        }
        for r in &self.catalog.revisions {
            self.tick()?;
            if r.entity.identity() == *id && self.visible(r) {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }
    fn target(&mut self, target: &Target) -> Result<Option<&'a Revision>, Error> {
        self.target_inner(target, &mut BTreeSet::new())
    }
    fn target_inner(
        &mut self,
        target: &Target,
        visiting: &mut BTreeSet<EntityRef>,
    ) -> Result<Option<&'a Revision>, Error> {
        if !self.permitted(&target.binding.identity()) {
            return Ok(None);
        }
        let record = match &target.binding {
            TargetBinding::Live(id) => self.resolve(id)?,
            TargetBinding::Frozen(reference) => self
                .find(reference)?
                .filter(|r| !r.retracted && r.transaction.start <= self.view.transaction),
        };
        if let Some(r) = record {
            if !self.readable_inner(r, visiting)? {
                return Ok(None);
            }
            validate_target_selector(r, &target.selector)?;
        } else {
            // An unavailable historical target cannot be mistaken for an absent
            // one simply because no visible candidate exists in these fixtures.
            self.require(target.binding.identity().kind)?;
        }
        Ok(record)
    }
    fn readable(&mut self, r: &Revision) -> Result<bool, Error> {
        self.readable_inner(r, &mut BTreeSet::new())
    }
    fn readable_inner(
        &mut self,
        r: &Revision,
        visiting: &mut BTreeSet<EntityRef>,
    ) -> Result<bool, Error> {
        self.tick()?;
        let subject_permitted = match &r.data {
            RecordData::Vector { owner } => self.permitted(owner),
            _ => self.permitted(&r.entity.identity()),
        };
        if !subject_permitted || r.transaction.start > self.view.transaction {
            return Ok(false);
        }
        // Fail closed on a circular proof of readability, but let the caller try
        // another target. Do not memoize this context-dependent negative result.
        if !visiting.insert(r.entity.clone()) {
            return Ok(false);
        }
        if visiting.len() > 128 {
            return Err(Error::BudgetExceeded);
        }
        let allowed = match &r.data {
            RecordData::Edge { source, target, .. } => {
                self.permitted(source) && self.permitted(target)
            }
            RecordData::Annotation { targets } => {
                let mut allowed = false;
                for target in targets {
                    if self.target_inner(target, visiting)?.is_some() {
                        allowed = true;
                        break;
                    }
                }
                allowed
            }
            RecordData::Change { subject, .. } => {
                let subject = self.find(subject)?.ok_or(Error::MissingRevision)?;
                self.readable_inner(subject, visiting)?
            }
            _ => true,
        };
        visiting.remove(&r.entity);
        if allowed {
            self.require(r.entity.kind)?;
        }
        Ok(allowed)
    }
    fn active(&mut self, r: &Revision) -> Result<bool, Error> {
        if !self.visible(r) || !self.readable(r)? {
            return Ok(false);
        }
        if let RecordData::Edge { source, target, .. } = &r.data {
            return Ok(self.resolve(source)?.is_some() && self.resolve(target)?.is_some());
        }
        Ok(true)
    }
    fn input_ref(&mut self, entity: &EntityRef) -> Result<bool, Error> {
        if entity.namespace != self.view.namespace {
            return Err(Error::NamespaceMismatch);
        }
        let r = self.find(entity)?.ok_or(Error::MissingRevision)?;
        if r.transaction.start > self.view.transaction {
            return Err(Error::MissingRevision);
        }
        self.readable(r)
    }
    fn input(&mut self, input: &Relation) -> Result<Vec<Row>, Error> {
        let mut rows = Vec::new();
        for row in &input.rows {
            self.tick()?;
            if row.len() != input.schema.len() || row.keys().ne(input.schema.keys()) {
                return Err(Error::InvalidSource);
            }
            let mut allowed = true;
            for (alias, value) in row {
                match (value, input.schema[alias]) {
                    (Binding::Null, _) => {}
                    (Binding::Entity(r), BindingType::Entity(kind)) if r.kind == kind => {
                        allowed &= self.input_ref(r)?
                    }
                    (Binding::Edges(edges), BindingType::Edges) => {
                        for edge in edges {
                            if edge.kind != Kind::Edge {
                                return Err(Error::Type(alias.clone()));
                            }
                            allowed &= self.input_ref(edge)?;
                        }
                    }
                    (Binding::Path(path), BindingType::Path) => {
                        if path.vertices.len() != path.edges.len() + 1 {
                            return Err(Error::InvalidSource);
                        }
                        for node in &path.vertices {
                            if node.kind != Kind::Node {
                                return Err(Error::Type(alias.clone()));
                            }
                            allowed &= self.input_ref(node)?;
                        }
                        for (index, edge) in path.edges.iter().enumerate() {
                            if edge.kind != Kind::Edge {
                                return Err(Error::Type(alias.clone()));
                            }
                            allowed &= self.input_ref(edge)?;
                            let revision = self.find(edge)?.ok_or(Error::MissingRevision)?;
                            let RecordData::Edge { source, target, .. } = &revision.data else {
                                return Err(Error::InvalidSource);
                            };
                            let left = path.vertices[index].identity();
                            let right = path.vertices[index + 1].identity();
                            if !((source == &left && target == &right)
                                || (source == &right && target == &left))
                            {
                                return Err(Error::InvalidSource);
                            }
                        }
                    }
                    _ => return Err(Error::Type(alias.clone())),
                }
            }
            if allowed {
                self.push(&mut rows, row.clone())?;
            }
        }
        Ok(rows)
    }
    fn source(&mut self, source: &Source) -> Result<Vec<Row>, Error> {
        if let Source::Input(input) = source {
            return self.input(input);
        }
        let mut selected = Vec::new();
        let alias = match source {
            Source::Scan { alias, .. }
            | Source::HistoryScan { alias, .. }
            | Source::ChangeScan { alias, .. } => alias,
            Source::Input(_) => unreachable!(),
        };
        for r in &self.catalog.revisions {
            self.tick()?;
            if r.entity.namespace != self.view.namespace {
                continue;
            }
            let matches = match source {
                Source::Scan {
                    kind, predicate, ..
                } => r.entity.kind == *kind && predicate.matches(r) && self.active(r)?,
                Source::HistoryScan {
                    kind,
                    predicate,
                    transactions,
                    valid,
                    ..
                } => {
                    r.entity.kind == *kind
                        && predicate.matches(r)
                        && r.transaction.start <= self.view.transaction
                        && transactions.start <= self.view.transaction
                        && r.transaction.overlaps(transactions)
                        && r.valid.overlaps(valid)
                        && self.readable(r)?
                }
                Source::ChangeScan {
                    predicate,
                    after,
                    through,
                    ..
                } => {
                    matches!(r.data, RecordData::Change { .. })
                        && predicate.matches(r)
                        && *after < r.transaction.start
                        && r.transaction.start <= *through
                        && r.transaction.start <= self.view.transaction
                        && self.readable(r)?
                }
                Source::Input(_) => unreachable!(),
            };
            if matches {
                self.push(&mut selected, r)?;
            }
        }
        if matches!(source, Source::Scan { .. }) {
            selected.sort_by(|a, b| a.entity.cmp(&b.entity));
        } else {
            selected.sort_by(|a, b| {
                (a.transaction.start, &a.entity).cmp(&(b.transaction.start, &b.entity))
            });
        }
        Ok(selected
            .into_iter()
            .map(|r| BTreeMap::from([(alias.clone(), Binding::Entity(r.entity.clone()))]))
            .collect())
    }

    fn annotations(&mut self, input: &[Row], lookup: &AnnotationLookup) -> Result<Vec<Row>, Error> {
        let mut output = Vec::new();
        for row in input {
            self.tick()?;
            let mut matches = Vec::new();
            if let Binding::Entity(reference) = &row[&lookup.target_alias] {
                for annotation in &self.catalog.revisions {
                    self.tick()?;
                    if !self.visible(annotation)
                        || !lookup.predicate.matches(annotation)
                        || !self.permitted(&annotation.entity.identity())
                    {
                        continue;
                    }
                    if let RecordData::Annotation { targets } = &annotation.data {
                        let mut matched = false;
                        for target in targets {
                            // First test exact identity to avoid inspecting unrelated bodies/sources.
                            let same = match &target.binding {
                                TargetBinding::Frozen(r) => r == reference,
                                TargetBinding::Live(id) => *id == reference.identity(),
                            };
                            if same && self.target(target)?.is_some_and(|r| r.entity == *reference)
                            {
                                matched = true;
                                break;
                            }
                        }
                        if matched {
                            self.push(&mut matches, annotation.entity.clone())?;
                        }
                    }
                }
            }
            matches.sort();
            if matches.is_empty() && lookup.optional {
                let mut next = row.clone();
                next.insert(lookup.alias.clone(), Binding::Null);
                self.push(&mut output, next)?;
            } else {
                for reference in matches {
                    let mut next = row.clone();
                    next.insert(lookup.alias.clone(), Binding::Entity(reference));
                    self.push(&mut output, next)?;
                }
            }
        }
        Ok(output)
    }

    fn walk_segment(
        &mut self,
        seed: WalkState,
        segment: &Segment,
        mode: PathMode,
    ) -> Result<Vec<WalkState>, Error> {
        let base_hops = seed.path.edges.len();
        let mut stack = vec![seed];
        let mut output = Vec::new();
        while let Some(state) = stack.pop() {
            self.tick()?;
            let hops = state.path.edges.len() - base_hops;
            let current = state.path.vertices.last().unwrap();
            let node = self.find(current)?.ok_or(Error::MissingRevision)?;
            if hops >= segment.min_hops && segment.node_predicate.matches(node) {
                let endpoint = Binding::Entity(current.clone());
                if state
                    .row
                    .get(&segment.end_alias)
                    .is_none_or(|old| old == &endpoint)
                {
                    let mut matched = state.clone();
                    matched.row.insert(segment.end_alias.clone(), endpoint);
                    if let Some(alias) = &segment.edge_alias {
                        let edges = &matched.path.edges[base_hops..];
                        let binding = if segment.min_hops == 1 && segment.max_hops == 1 {
                            Binding::Entity(edges[0].clone())
                        } else {
                            Binding::Edges(edges.to_vec())
                        };
                        matched.row.insert(alias.clone(), binding);
                    }
                    self.push(&mut output, matched)?;
                }
            }
            if hops == segment.max_hops {
                continue;
            }
            for edge in &self.catalog.revisions {
                self.tick()?;
                let RecordData::Edge {
                    source,
                    target,
                    relation,
                } = &edge.data
                else {
                    continue;
                };
                if !segment.edge_predicate.matches(edge)
                    || (!segment.relations.is_empty() && !segment.relations.contains(relation))
                    || !self.active(edge)?
                {
                    continue;
                }
                let id = current.identity();
                // A self-loop in BOTH is one oriented choice, not a duplicate.
                let next = if segment.direction != Direction::In && *source == id {
                    Some(target)
                } else if segment.direction != Direction::Out && *target == id {
                    Some(source)
                } else {
                    None
                };
                let Some(next) = next else {
                    continue;
                };
                if mode == PathMode::Trail
                    && state
                        .path
                        .edges
                        .iter()
                        .any(|r| r.identity() == edge.entity.identity())
                {
                    continue;
                }
                if mode == PathMode::Simple
                    && state.path.vertices.iter().any(|r| r.identity() == *next)
                {
                    continue;
                }
                let Some(next_node) = self.resolve(next)? else {
                    continue;
                };
                let mut branch = state.clone();
                branch.path.edges.push(edge.entity.clone());
                branch.path.vertices.push(next_node.entity.clone());
                self.push(&mut stack, branch)?;
            }
        }
        Ok(output)
    }
    fn expand(
        &mut self,
        input: &[Row],
        expand: &Expand,
        schema: &Schema,
    ) -> Result<Vec<Row>, Error> {
        let mut output = Vec::new();
        for row in input {
            self.tick()?;
            let mut states = Vec::new();
            if let Binding::Entity(reference) = &row[&expand.start_alias] {
                if self
                    .resolve(&reference.identity())?
                    .is_some_and(|r| r.entity == *reference)
                {
                    states.push(WalkState {
                        row: row.clone(),
                        path: Path {
                            vertices: vec![reference.clone()],
                            edges: vec![],
                        },
                    });
                }
            }
            for segment in &expand.segments {
                let mut next = Vec::new();
                for state in states {
                    for matched in self.walk_segment(state, segment, expand.mode)? {
                        self.push(&mut next, matched)?;
                    }
                }
                states = next;
            }
            states.sort_by(|a, b| {
                a.path
                    .edges
                    .len()
                    .cmp(&b.path.edges.len())
                    .then(a.path.edges.cmp(&b.path.edges))
                    .then(a.path.vertices.cmp(&b.path.vertices))
                    .then(a.row.cmp(&b.row))
            });
            if expand.shortest {
                states.truncate(1);
            }
            if states.is_empty() && expand.optional {
                let mut next = row.clone();
                for alias in schema.keys() {
                    next.entry(alias.clone()).or_insert(Binding::Null);
                }
                self.push(&mut output, next)?;
            } else {
                for mut state in states {
                    if let Some(alias) = &expand.path_alias {
                        state.row.insert(alias.clone(), Binding::Path(state.path));
                    }
                    self.push(&mut output, state.row)?;
                }
            }
        }
        Ok(output)
    }
}

#[derive(Clone)]
struct WalkState {
    row: Row,
    path: Path,
}

pub fn execute(
    catalog: &Catalog,
    view: &View,
    plan: &Plan,
    limits: Limits,
) -> Result<Relation, Error> {
    execute_metered(catalog, view, plan, limits).map(|(relation, _)| relation)
}

/// Executes with the same per-call limits and returns consumed runtime work.
/// Counts include preflight, catalog validation, source reads and transforms.
/// The caller can subtract the successful count from a shared remaining quota
/// and pass that remainder as Limits::max_work on the next call. Work units are
/// implementation counters, not elapsed time or an estimate of byte allocation.
/// Errors return no partial relation/count: abort the enclosing query on error.
pub fn execute_metered(
    catalog: &Catalog,
    view: &View,
    plan: &Plan,
    limits: Limits,
) -> Result<(Relation, usize), Error> {
    let mut runtime = Runtime {
        catalog,
        view,
        limits,
        work: 0,
    };
    let schemas = runtime.preflight(plan)?;
    runtime.validate_catalog()?;
    let mut rows = runtime.source(&plan.source)?;
    for (index, stage) in plan.stages.iter().enumerate() {
        rows = match stage {
            Stage::Expand(expand) => runtime.expand(&rows, expand, &schemas[index + 1])?,
            Stage::Annotations(lookup) => runtime.annotations(&rows, lookup)?,
        };
    }
    Ok((
        Relation {
            schema: schemas.last().unwrap().clone(),
            rows,
        },
        runtime.work,
    ))
}

/// Domain transform integration seam. Bag order and duplicate input rows survive.
///
/// A relational adapter may pass one input row's domain bindings and merge each
/// emitted row with that input's scalar fields. First call with the full domain
/// schema and zero rows: configuration is still validated, and the returned
/// schema lets the adapter reject collisions with all carried scalar aliases.
/// The adapter must also enforce a cumulative budget across per-row calls;
/// Limits here apply to one call, including catalog validation and intermediates.
/// Use transform_metered to debit that shared budget, including empty-input calls.
pub fn transform(
    catalog: &Catalog,
    view: &View,
    input: &Relation,
    stages: &[Stage],
    limits: Limits,
) -> Result<Relation, Error> {
    transform_metered(catalog, view, input, stages, limits).map(|(relation, _)| relation)
}

/// Metered counterpart of transform; see execute_metered for work/error semantics.
pub fn transform_metered(
    catalog: &Catalog,
    view: &View,
    input: &Relation,
    stages: &[Stage],
    limits: Limits,
) -> Result<(Relation, usize), Error> {
    execute_metered(
        catalog,
        view,
        &Plan {
            source: Source::Input(input.clone()),
            stages: stages.to_vec(),
        },
        limits,
    )
}

/// Returns a new history (closed prior transaction version + new fragments).
/// The supplied revision and its immutable payload are never modified.
pub fn correct_revision(
    old: &Revision,
    at: u64,
    window: Interval<i64>,
    fields: Fields,
    revision_ids: [&str; 3],
) -> Result<Vec<Revision>, Error> {
    old.valid.validate()?;
    old.transaction.validate()?;
    window.validate()?;
    if old.retracted
        || old.transaction.end.is_some()
        || at <= old.transaction.start
        || window.start < old.valid.start
        || old
            .valid
            .end
            .is_some_and(|end| window.end.is_none_or(|w| w > end))
    {
        return Err(Error::InvalidInterval);
    }
    let ids: BTreeSet<_> = revision_ids.iter().copied().collect();
    if ids.len() != 3 || ids.contains("") || ids.contains(old.entity.revision.as_str()) {
        return Err(Error::InvalidCatalog);
    }
    let mut retired = old.clone();
    retired.transaction.end = Some(at);
    let mut history = vec![retired];
    let windows = [
        (old.valid.start < window.start).then_some(Interval {
            start: old.valid.start,
            end: Some(window.start),
        }),
        Some(window.clone()),
        window
            .end
            .filter(|end| old.valid.end.is_none_or(|old_end| *end < old_end))
            .map(|start| Interval {
                start,
                end: old.valid.end,
            }),
    ];
    for (index, valid) in windows.into_iter().enumerate() {
        if let Some(valid) = valid {
            let mut revision = old.clone();
            revision.entity.revision = revision_ids[index].into();
            revision.transaction = Interval {
                start: at,
                end: None,
            };
            revision.valid = valid;
            if index == 1 {
                revision.fields = fields.clone();
            }
            history.push(revision);
        }
    }
    Ok(history)
}

/// SHA-256 of the UTF-8 bytes of the already-normalized fixture extraction text.
/// Local std-only implementation, verified against fixed published test vectors.
pub fn source_hash(text: &str) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut bytes = text.as_bytes().to_vec();
    let bits = (bytes.len() as u64).wrapping_mul(8);
    bytes.push(0x80);
    while bytes.len() % 64 != 56 {
        bytes.push(0);
    }
    bytes.extend_from_slice(&bits.to_be_bytes());
    for block in bytes.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
    state.iter().map(|word| format!("{word:08x}")).collect()
}

pub fn validate_selector(source: &TextSource, selector: &Selector) -> Result<(), Error> {
    if let Selector::TextPosition {
        start,
        end,
        source_hash: hash,
        extraction_id,
        normalization_id,
    } = selector
    {
        if start > end || *end > source.text.chars().count() {
            return Err(Error::InvalidSelector);
        }
        if extraction_id.is_empty()
            || normalization_id.is_empty()
            || *hash != source_hash(&source.text)
            || *extraction_id != source.extraction_id
            || *normalization_id != source.normalization_id
        {
            return Err(Error::InvalidSource);
        }
    }
    Ok(())
}

fn validate_target_selector(revision: &Revision, selector: &Selector) -> Result<(), Error> {
    match (selector, &revision.data) {
        (Selector::Whole, _) => Ok(()),
        (_, RecordData::Text(source)) => validate_selector(source, selector),
        _ => Err(Error::InvalidSource),
    }
}
