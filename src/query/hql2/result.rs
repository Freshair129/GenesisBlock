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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterUnitV2 {
    Rows,
    Bytes,
    WorkUnits,
    IndexProbes,
    DistanceEvaluations,
    GraphExpansions,
    Nanoseconds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterUnknownReasonV2 {
    NotInstrumented,
    NotApplicable,
    Overflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CounterMeasurementV2 {
    Measured { value: u64 },
    Unknown { reason: CounterUnknownReasonV2 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CounterReadingV2 {
    pub unit: CounterUnitV2,
    pub measurement: CounterMeasurementV2,
}

impl CounterReadingV2 {
    pub(crate) const fn measured(unit: CounterUnitV2, value: u64) -> Self {
        Self {
            unit,
            measurement: CounterMeasurementV2::Measured { value },
        }
    }

    pub(crate) const fn measured_or_overflow(unit: CounterUnitV2, value: Option<u64>) -> Self {
        match value {
            Some(value) => Self::measured(unit, value),
            None => Self::unknown(unit, CounterUnknownReasonV2::Overflow),
        }
    }

    pub(crate) const fn unknown(unit: CounterUnitV2, reason: CounterUnknownReasonV2) -> Self {
        Self {
            unit,
            measurement: CounterMeasurementV2::Unknown { reason },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterSamplingV2 {
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterClockSourceV2 {
    MonotonicInstant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterElapsedScopeV2 {
    OperatorExecution,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActualCountersV2 {
    pub rows_in: CounterReadingV2,
    pub rows_out: CounterReadingV2,
    pub bytes_read: CounterReadingV2,
    pub work_units: CounterReadingV2,
    pub index_probes: CounterReadingV2,
    pub distance_evaluations: CounterReadingV2,
    pub graph_expansions: CounterReadingV2,
    pub memory_peak_bytes: CounterReadingV2,
    pub spill_bytes: CounterReadingV2,
    pub elapsed_ns: CounterReadingV2,
    pub sampling: CounterSamplingV2,
    pub clock_source: CounterClockSourceV2,
    pub elapsed_scope: CounterElapsedScopeV2,
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
    pub contract_version: String,
    pub planner_version: String,
    pub plan_hash: String,
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
// This result enum keeps the public rows/plan variants directly accessible.
#[allow(clippy::large_enum_variant)]
pub enum QueryOutcomeV2 {
    Rows(QueryResultV2),
    Plan(ExplainResultV2),
}
