//! Isolated structural contracts for the UEE/HQL2 v2 boundary.
//!
//! This module deliberately does not connect to `Storage`, the v1 router, the
//! current journal, or any migration path. It validates the wire envelope and
//! the invariants that can be checked without a catalog, authenticated
//! principal, snapshot, or execution engine. Those checks belong to later G1-
//! G3 stages.

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use uuid::Uuid;

pub const API_CONTRACT_V2: &str = "genesis.api.v2";
pub const QUERY_IR_CONTRACT_V2: &str = "query-ir.v2";
pub const TRANSACTION_CONTRACT_V2: &str = "genesis.tx.v2";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractError {
    code: &'static str,
    message: String,
}

impl ContractError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ContractError {}

fn invalid(message: impl Into<String>) -> ContractError {
    ContractError {
        code: "UEE_V2_CONTRACT_INVALID",
        message: message.into(),
    }
}

fn decode<T: DeserializeOwned>(value: Value) -> Result<T, ContractError> {
    serde_json::from_value(value).map_err(|error| invalid(format!("decode: {error}")))
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn is_namespace(value: &str) -> bool {
    let length = value.len();
    (1..=63).contains(&length)
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase())
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn is_decimal(value: &str) -> bool {
    if value == "0" {
        return true;
    }
    !value.is_empty()
        && value.as_bytes()[0].is_ascii_digit()
        && value.as_bytes()[0] != b'0'
        && value.as_bytes()[1..].iter().all(u8::is_ascii_digit)
}

fn validate_namespace(namespace: &str) -> Result<(), ContractError> {
    if is_namespace(namespace) {
        Ok(())
    } else {
        Err(invalid(format!("invalid namespace `{namespace}`")))
    }
}

fn validate_identifier(kind: &str, value: &str) -> Result<(), ContractError> {
    if is_identifier(value) {
        Ok(())
    } else {
        Err(invalid(format!("invalid {kind} identifier `{value}`")))
    }
}

fn validate_decimal(kind: &str, value: &str) -> Result<(), ContractError> {
    if is_decimal(value) {
        Ok(())
    } else {
        Err(invalid(format!("invalid decimal {kind} `{value}`")))
    }
}

fn validate_interval(valid: &Option<ValidityV2>) -> Result<(), ContractError> {
    if let Some(interval) = valid {
        if interval.to.is_some_and(|to| to < interval.from) {
            return Err(invalid("valid interval ends before it starts"));
        }
    }
    Ok(())
}

fn validate_hash(kind: &str, value: &str) -> Result<(), ContractError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(invalid(format!("invalid {kind} hash")))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndexPolicyV2 {
    MergeDelta,
    Wait,
    Eventual,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum HqlLanguageVersionV2 {
    #[serde(rename = "hql.v1")]
    HqlV1,
    #[serde(rename = "hql.v2")]
    HqlV2,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExplainV2 {
    None,
    Plan,
    Analyze,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryFormatV2 {
    Json,
    Stream,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QueryTemporalV2 {
    #[serde(default)]
    pub tx_as_of: Option<String>,
    #[serde(default)]
    pub valid_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QueryBudgetV2 {
    #[serde(default)]
    pub max_memory_bytes: Option<u64>,
    #[serde(default)]
    pub max_spill_bytes: Option<u64>,
    #[serde(default)]
    pub max_elapsed_ms: Option<u64>,
    #[serde(default)]
    pub max_expanded_nodes: Option<u64>,
    #[serde(default)]
    pub max_expanded_edges: Option<u64>,
    #[serde(default)]
    pub max_vector_candidates: Option<u64>,
    #[serde(default)]
    pub max_distance_evaluations: Option<u64>,
    #[serde(default)]
    pub max_result_rows: Option<u64>,
    #[serde(default)]
    pub max_result_bytes: Option<u64>,
}

impl QueryBudgetV2 {
    fn validate(&self) -> Result<(), ContractError> {
        for (name, value) in [
            ("max_memory_bytes", self.max_memory_bytes),
            ("max_elapsed_ms", self.max_elapsed_ms),
            ("max_expanded_nodes", self.max_expanded_nodes),
            ("max_expanded_edges", self.max_expanded_edges),
            ("max_vector_candidates", self.max_vector_candidates),
            ("max_distance_evaluations", self.max_distance_evaluations),
            ("max_result_rows", self.max_result_rows),
            ("max_result_bytes", self.max_result_bytes),
        ] {
            if value == Some(0) {
                return Err(invalid(format!("{name} must be positive")));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum QueryOpV2 {
    AnnotationLookup,
    AnnotationScan,
    Aggregate,
    ChangeScan,
    ContextPack,
    Distinct,
    EdgeScan,
    Expand,
    Filter,
    HistoryScan,
    Join,
    Knn,
    LexicalMatch,
    Match,
    NodeScan,
    Offset,
    Project,
    Rerank,
    RowScan,
    Sort,
    Take,
    UnionAll,
    Values,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QueryNodeV2 {
    pub id: String,
    pub op: QueryOpV2,
    pub inputs: Vec<String>,
    pub config: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QueryIrV2 {
    pub contract_version: String,
    pub nodes: Vec<QueryNodeV2>,
    pub root: String,
    #[serde(default)]
    pub parameter_types: BTreeMap<String, String>,
}

impl QueryIrV2 {
    pub fn from_value(value: Value) -> Result<Self, ContractError> {
        let ir: Self = decode(value)?;
        ir.validate()?;
        Ok(ir)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.contract_version != QUERY_IR_CONTRACT_V2 {
            return Err(invalid(format!(
                "unsupported query IR contract `{}`",
                self.contract_version
            )));
        }
        if self.nodes.is_empty() || self.nodes.len() > 10_000 {
            return Err(invalid("query IR node count is outside 1..=10000"));
        }
        validate_identifier("root", &self.root)?;

        let mut node_ids = HashSet::with_capacity(self.nodes.len());
        for node in &self.nodes {
            validate_identifier("query node", &node.id)?;
            if !node_ids.insert(node.id.clone()) {
                return Err(invalid(format!("duplicate query node `{}`", node.id)));
            }
        }
        if !node_ids.contains(&self.root) {
            return Err(invalid(format!(
                "query root `{}` does not exist",
                self.root
            )));
        }
        for parameter in self.parameter_types.keys() {
            validate_identifier("parameter", parameter)?;
        }
        if self.parameter_types.values().any(String::is_empty) {
            return Err(invalid("query parameter type cannot be empty"));
        }

        let mut indegree: HashMap<String, usize> =
            self.nodes.iter().map(|node| (node.id.clone(), 0)).collect();
        let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
        for node in &self.nodes {
            for input in &node.inputs {
                if !node_ids.contains(input) {
                    return Err(invalid(format!(
                        "query node `{}` references unknown input `{input}`",
                        node.id
                    )));
                }
                *indegree
                    .get_mut(&node.id)
                    .expect("node IDs were inserted above") += 1;
                outgoing
                    .entry(input.clone())
                    .or_default()
                    .push(node.id.clone());
            }
        }

        let mut ready: Vec<String> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
            .collect();
        let mut visited = 0;
        while let Some(id) = ready.pop() {
            visited += 1;
            if let Some(children) = outgoing.get(&id) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("outgoing node IDs were inserted above");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push(child.clone());
                    }
                }
            }
        }
        if visited != self.nodes.len() {
            return Err(invalid("query IR contains a cycle"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QueryRequestV2 {
    pub contract_version: String,
    pub request_id: String,
    pub namespace: String,
    #[serde(default)]
    pub hql: Option<String>,
    #[serde(default)]
    pub ir: Option<QueryIrV2>,
    #[serde(default)]
    pub language_version: Option<HqlLanguageVersionV2>,
    pub params: BTreeMap<String, Value>,
    #[serde(default)]
    pub temporal: Option<QueryTemporalV2>,
    #[serde(default)]
    pub index_policy: Option<IndexPolicyV2>,
    #[serde(default)]
    pub required_frontier: Option<String>,
    #[serde(default)]
    pub transaction_id: Option<Uuid>,
    #[serde(default)]
    pub budget: Option<QueryBudgetV2>,
    #[serde(default)]
    pub allow_partial: Option<bool>,
    #[serde(default)]
    pub explain: Option<ExplainV2>,
    #[serde(default)]
    pub format: Option<QueryFormatV2>,
}

impl QueryRequestV2 {
    pub fn from_value(value: Value) -> Result<Self, ContractError> {
        let request: Self = decode(value)?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.contract_version != API_CONTRACT_V2 {
            return Err(invalid(format!(
                "unsupported API contract `{}`",
                self.contract_version
            )));
        }
        if self.request_id.is_empty() || self.request_id.chars().count() > 256 {
            return Err(invalid("request_id must contain 1..=256 characters"));
        }
        validate_namespace(&self.namespace)?;
        match (&self.hql, &self.ir, &self.language_version) {
            (Some(hql), None, Some(_)) if !hql.is_empty() && hql.chars().count() <= 262_144 => {}
            (None, Some(ir), None) => ir.validate()?,
            (Some(_), None, Some(_)) => {
                return Err(invalid("hql must be non-empty and <= 262144 characters"))
            }
            (Some(_), Some(_), _) => {
                return Err(invalid("query request cannot contain both hql and ir"))
            }
            (None, None, _) => return Err(invalid("query request requires hql or ir")),
            (Some(_), None, None) => return Err(invalid("hql requires language_version")),
            (None, Some(_), Some(_)) => return Err(invalid("ir cannot contain language_version")),
        }
        if let Some(temporal) = &self.temporal {
            if let Some(tx_as_of) = &temporal.tx_as_of {
                validate_decimal("tx_as_of", tx_as_of)?;
            }
        }
        if let Some(frontier) = &self.required_frontier {
            validate_decimal("required_frontier", frontier)?;
        }
        if let Some(budget) = &self.budget {
            budget.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValidityV2 {
    pub from: DateTime<Utc>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NodePutMutationV2 {
    pub id: String,
    pub labels: Vec<String>,
    pub props: BTreeMap<String, Value>,
    #[serde(default)]
    pub expected_revision: Option<Uuid>,
    #[serde(default)]
    pub valid: Option<ValidityV2>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EdgePutMutationV2 {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation: String,
    pub props: BTreeMap<String, Value>,
    #[serde(default)]
    pub expected_revision: Option<Uuid>,
    #[serde(default)]
    pub valid: Option<ValidityV2>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RowPutMutationV2 {
    pub table: String,
    pub key: BTreeMap<String, Value>,
    pub values: BTreeMap<String, Value>,
    #[serde(default)]
    pub expected_revision: Option<Uuid>,
    #[serde(default)]
    pub valid: Option<ValidityV2>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VectorPutMutationV2 {
    pub owner_id: String,
    pub collection: String,
    pub space_id: String,
    pub values: Vec<f64>,
    #[serde(default)]
    pub expected_revision: Option<Uuid>,
    #[serde(default)]
    pub valid: Option<ValidityV2>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnnotationPutMutationV2 {
    pub annotation: Value,
    #[serde(default)]
    pub expected_revision: Option<Uuid>,
    #[serde(default)]
    pub valid: Option<ValidityV2>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecordKindV2 {
    Node,
    Edge,
    Row,
    Annotation,
    Artifact,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecordRetractMutationV2 {
    pub kind: RecordKindV2,
    pub id: String,
    #[serde(default)]
    pub expected_revision: Option<Uuid>,
    #[serde(default)]
    pub valid: Option<ValidityV2>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SchemaPutMutationV2 {
    pub name: String,
    pub version: u64,
    pub previous_version: u64,
    pub schema: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactPutMutationV2 {
    pub id: String,
    pub uri: String,
    pub content_hash: String,
    pub media_type: String,
    #[serde(default)]
    pub blob_ref: Option<String>,
    #[serde(default)]
    pub expected_revision: Option<Uuid>,
    #[serde(default)]
    pub valid: Option<ValidityV2>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "op")]
pub enum MutationV2 {
    #[serde(rename = "node.put")]
    NodePut(NodePutMutationV2),
    #[serde(rename = "edge.put")]
    EdgePut(EdgePutMutationV2),
    #[serde(rename = "row.put")]
    RowPut(RowPutMutationV2),
    #[serde(rename = "vector.put")]
    VectorPut(VectorPutMutationV2),
    #[serde(rename = "annotation.put")]
    AnnotationPut(AnnotationPutMutationV2),
    #[serde(rename = "record.retract")]
    RecordRetract(RecordRetractMutationV2),
    #[serde(rename = "schema.put")]
    SchemaPut(SchemaPutMutationV2),
    #[serde(rename = "artifact.put")]
    ArtifactPut(ArtifactPutMutationV2),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum DurabilityV2 {
    #[serde(rename = "durable_published")]
    DurablePublished,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransactionV2 {
    pub contract_version: String,
    pub transaction_id: Uuid,
    pub namespace: String,
    #[serde(default)]
    pub expected_frontier: Option<String>,
    pub mutations: Vec<MutationV2>,
    #[serde(default)]
    pub actor_context: BTreeMap<String, Value>,
    #[serde(default)]
    pub durability: Option<DurabilityV2>,
}

impl TransactionV2 {
    pub fn from_value(value: Value) -> Result<Self, ContractError> {
        let transaction: Self = decode(value)?;
        transaction.validate()?;
        Ok(transaction)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.contract_version != TRANSACTION_CONTRACT_V2 {
            return Err(invalid(format!(
                "unsupported transaction contract `{}`",
                self.contract_version
            )));
        }
        validate_namespace(&self.namespace)?;
        if self.mutations.is_empty() || self.mutations.len() > 10_000 {
            return Err(invalid("transaction mutation count is outside 1..=10000"));
        }
        if let Some(frontier) = &self.expected_frontier {
            validate_decimal("expected_frontier", frontier)?;
        }
        for mutation in &self.mutations {
            mutation.validate()?;
        }
        Ok(())
    }
}

impl MutationV2 {
    fn validate(&self) -> Result<(), ContractError> {
        match self {
            Self::NodePut(mutation) => {
                validate_nonempty("node id", &mutation.id)?;
                validate_labels(&mutation.labels)?;
                validate_interval(&mutation.valid)?;
            }
            Self::EdgePut(mutation) => {
                validate_nonempty("edge id", &mutation.id)?;
                validate_nonempty("edge source_id", &mutation.source_id)?;
                validate_nonempty("edge target_id", &mutation.target_id)?;
                validate_nonempty("edge relation", &mutation.relation)?;
                validate_interval(&mutation.valid)?;
            }
            Self::RowPut(mutation) => {
                validate_identifier("table", &mutation.table)?;
                if mutation.key.is_empty() {
                    return Err(invalid("row key cannot be empty"));
                }
                validate_interval(&mutation.valid)?;
            }
            Self::VectorPut(mutation) => {
                validate_nonempty("vector owner_id", &mutation.owner_id)?;
                validate_identifier("vector collection", &mutation.collection)?;
                validate_nonempty("vector space_id", &mutation.space_id)?;
                if mutation.values.is_empty() || mutation.values.len() > 65_535 {
                    return Err(invalid("vector dimension is outside 1..=65535"));
                }
                if mutation.values.iter().any(|value| !value.is_finite()) {
                    return Err(invalid("vector values must be finite"));
                }
                validate_interval(&mutation.valid)?;
            }
            Self::AnnotationPut(mutation) => {
                if !mutation.annotation.is_object() {
                    return Err(invalid("annotation must be an object"));
                }
                validate_interval(&mutation.valid)?;
            }
            Self::RecordRetract(mutation) => {
                validate_nonempty("retracted record id", &mutation.id)?;
                validate_interval(&mutation.valid)?;
            }
            Self::SchemaPut(mutation) => {
                validate_identifier("schema", &mutation.name)?;
                if mutation.version == 0 || mutation.schema.is_empty() {
                    return Err(invalid("schema version and body must be non-empty"));
                }
            }
            Self::ArtifactPut(mutation) => {
                validate_nonempty("artifact id", &mutation.id)?;
                validate_nonempty("artifact uri", &mutation.uri)?;
                validate_nonempty("artifact media_type", &mutation.media_type)?;
                validate_hash("artifact content_hash", &mutation.content_hash)?;
                if let Some(blob_ref) = &mutation.blob_ref {
                    validate_hash("artifact blob_ref", blob_ref)?;
                }
                validate_interval(&mutation.valid)?;
            }
        }
        Ok(())
    }
}

fn validate_nonempty(kind: &str, value: &str) -> Result<(), ContractError> {
    if value.is_empty() {
        Err(invalid(format!("{kind} cannot be empty")))
    } else {
        Ok(())
    }
}

fn validate_labels(labels: &[String]) -> Result<(), ContractError> {
    let mut unique = HashSet::with_capacity(labels.len());
    for label in labels {
        validate_nonempty("label", label)?;
        if !unique.insert(label) {
            return Err(invalid(format!("duplicate label `{label}`")));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStateV2 {
    DurablePublished,
    DurableRecoveryRequired,
    Rejected,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommitReceiptV2 {
    pub transaction_id: Uuid,
    pub namespace: String,
    pub database_id: Uuid,
    pub state: ReceiptStateV2,
    #[serde(default)]
    pub durable_frontier: Option<String>,
    #[serde(default)]
    pub published_frontier: Option<String>,
    #[serde(default)]
    pub index_frontiers: BTreeMap<String, String>,
}

impl CommitReceiptV2 {
    pub fn from_value(value: Value) -> Result<Self, ContractError> {
        let receipt: Self = decode(value)?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        validate_namespace(&self.namespace)?;
        for (kind, frontier) in [
            ("durable_frontier", self.durable_frontier.as_deref()),
            ("published_frontier", self.published_frontier.as_deref()),
        ] {
            if let Some(frontier) = frontier {
                validate_decimal(kind, frontier)?;
            }
        }
        for frontier in self.index_frontiers.values() {
            validate_decimal("index_frontier", frontier)?;
        }
        Ok(())
    }
}
