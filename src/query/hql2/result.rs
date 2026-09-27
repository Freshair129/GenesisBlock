use super::value::QueryValueV2;
use crate::uee_v2::QueryOpV2;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ColumnV2 {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CatalogStampV2 {
    pub observed_frontier: u64,
    pub policy_revision: u64,
    pub schema_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EstimateV2 {
    pub rows_min: u64,
    pub rows_max: Option<u64>,
    pub confidence: EstimateConfidenceV2,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimateConfidenceV2 {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ActualCountersV2 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_rows: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_rows: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_evaluations: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded_nodes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded_edges: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_records_examined: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_accounted_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_micros: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplainNodeV2 {
    pub id: String,
    pub logical_op: QueryOpV2,
    pub physical_op: String,
    pub inputs: Vec<String>,
    pub columns: Vec<ColumnV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimates: Option<EstimateV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<ActualCountersV2>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplainResultV2 {
    pub request_id: String,
    pub catalog: CatalogStampV2,
    pub plan: Vec<ExplainNodeV2>,
    pub root: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SnapshotV2 {
    pub database_id: String,
    pub tx: String,
    pub valid_at: String,
    pub catalog_generation: String,
    pub policy_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SemanticsV2 {
    pub candidate_search: CandidateSearchV2,
    pub distance_fidelity: DistanceFidelityV2,
    pub index_coverage: IndexCoverageV2,
    pub scope: ResultScopeV2,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSearchV2 {
    Exact,
    Approximate,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DistanceFidelityV2 {
    Original,
    Quantized,
    NotApplicable,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexCoverageV2 {
    Complete,
    Lagging,
    NotApplicable,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultScopeV2 {
    WholeInput,
    RerankCandidates,
    NotApplicable,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CompletenessV2 {
    pub status: CompletionStatusV2,
    pub reason: Option<String>,
    pub eligible_count_known: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionStatusV2 {
    Complete,
    Truncated,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QueryResultV2 {
    pub request_id: String,
    pub snapshot: SnapshotV2,
    pub columns: Vec<ColumnV2>,
    pub rows: Vec<BTreeMap<String, QueryValueV2>>,
    pub semantics: SemanticsV2,
    pub completeness: CompletenessV2,
    pub index_frontiers: BTreeMap<String, String>,
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<ExplainResultV2>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QueryOutcomeV2 {
    Rows(QueryResultV2),
    Plan(ExplainResultV2),
}
