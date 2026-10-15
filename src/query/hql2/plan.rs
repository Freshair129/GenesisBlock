//! Deterministic scalar planning; no indexes, rewrites or data operators open.
use super::{
    bind::{BoundNode, BoundQueryV2, Column, Kernel, VectorRankKind},
    error::QueryErrorV2,
    result::{
        CandidateSearchV2, ColumnV2, DistanceFidelityV2, ExplainNodeV2, IndexCoverageV2,
        ResultScopeV2, SemanticsV2,
    },
    source::BoundSourceV2,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub(crate) const PLANNER_VERSION_V2: &str = "hql2-rule-v1";

#[derive(Serialize)]
struct PlanIdentityV2 {
    planner_version: &'static str,
    root: String,
    plan: Vec<ExplainNodeV2>,
}

#[derive(Debug)]
pub(crate) struct PhysicalPlanV2 {
    nodes: Vec<BoundNode>,
    root: usize,
}
pub(crate) fn plan_v2(query: BoundQueryV2) -> Result<PhysicalPlanV2, QueryErrorV2> {
    let (nodes, root) = query.into_parts();
    Ok(PhysicalPlanV2 { nodes, root })
}
fn columns(columns: &[Column]) -> Vec<ColumnV2> {
    columns
        .iter()
        .map(|c| ColumnV2 {
            name: c.name.clone(),
            data_type: c.ty.name(),
            nullable: c.ty.nullable(),
        })
        .collect()
}
impl PhysicalPlanV2 {
    pub(crate) fn plan_hash(&self) -> Result<String, QueryErrorV2> {
        let canonical = PlanIdentityV2 {
            planner_version: PLANNER_VERSION_V2,
            root: self.root().to_owned(),
            plan: self.explain_nodes(),
        };
        let bytes = serde_json::to_vec(&canonical)
            .map_err(|_| QueryErrorV2::new("ENCODE_ERROR", "encode", "plan_hash"))?;
        let mut hasher = Sha256::new();
        hasher.update(b"genesis.hql2.plan.v2:");
        hasher.update(bytes);
        Ok(hex::encode(hasher.finalize()))
    }

    pub(crate) fn columns(&self) -> Vec<ColumnV2> {
        columns(&self.nodes[self.root].columns)
    }
    pub(crate) fn root(&self) -> &str {
        &self.nodes[self.root].id
    }
    pub(super) fn nodes(&self) -> &[BoundNode] {
        &self.nodes
    }
    pub(super) fn root_index(&self) -> usize {
        self.root
    }
    pub(crate) fn source_scans(&self) -> Vec<(String, BoundSourceV2)> {
        self.nodes
            .iter()
            .filter_map(|node| match &node.kernel {
                Kernel::SourceScan(source) => Some((node.id.clone(), source.clone())),
                _ => None,
            })
            .collect()
    }
    pub(crate) fn requires_graph_snapshot(&self) -> bool {
        self.nodes.iter().any(|node| {
            matches!(
                node.kernel,
                Kernel::MatchCompact { .. }
                    | Kernel::MatchSequence { .. }
                    | Kernel::Expand { .. }
                    | Kernel::ExpandSequence { .. }
                    | Kernel::LexicalMatch { .. }
            )
        })
    }
    pub(crate) fn requires_node_labels(&self) -> bool {
        self.nodes.iter().any(|node| match &node.kernel {
            Kernel::MatchSequence { start, steps, .. }
            | Kernel::ExpandSequence { start, steps, .. } => {
                !start.labels.is_empty() || steps.iter().any(|step| !step.node.labels.is_empty())
            }
            _ => false,
        })
    }
    pub(crate) fn requires_vector_snapshot(&self) -> bool {
        self.nodes
            .iter()
            .any(|node| matches!(node.kernel, Kernel::VectorRank { .. }))
    }
    pub(crate) fn semantics(&self) -> SemanticsV2 {
        let mut has_knn = false;
        let mut has_rerank = false;
        for node in &self.nodes {
            if let Kernel::VectorRank { kind, .. } = node.kernel {
                match kind {
                    VectorRankKind::Knn => has_knn = true,
                    VectorRankKind::Rerank => has_rerank = true,
                }
            }
        }
        if has_knn || has_rerank {
            SemanticsV2 {
                candidate_search: CandidateSearchV2::Exact,
                distance_fidelity: DistanceFidelityV2::Original,
                index_coverage: IndexCoverageV2::NotApplicable,
                scope: if has_rerank {
                    ResultScopeV2::RerankCandidates
                } else {
                    ResultScopeV2::WholeInput
                },
            }
        } else {
            SemanticsV2 {
                candidate_search: CandidateSearchV2::Exact,
                distance_fidelity: DistanceFidelityV2::NotApplicable,
                index_coverage: IndexCoverageV2::NotApplicable,
                scope: ResultScopeV2::WholeInput,
            }
        }
    }
    pub(crate) fn explain_nodes(&self) -> Vec<ExplainNodeV2> {
        self.nodes
            .iter()
            .map(|n| ExplainNodeV2 {
                id: n.id.clone(),
                logical_op: n.op.clone(),
                physical_op: match n.kernel {
                    Kernel::Values(_) => "Values",
                    Kernel::SourceScan(BoundSourceV2::Node(_)) => "AuthorizedNodeScan",
                    Kernel::SourceScan(BoundSourceV2::Edge(_)) => "AuthorizedEdgeScan",
                    Kernel::SourceScan(BoundSourceV2::Row(_)) => "AuthorizedRowScan",
                    Kernel::SourceScan(BoundSourceV2::Annotation) => "AuthorizedAnnotationScan",
                    Kernel::SourceScan(BoundSourceV2::History { .. }) => "AuthorizedHistoryScan",
                    Kernel::SourceScan(BoundSourceV2::Changes { .. }) => "AuthorizedChangeScan",
                    Kernel::AnnotationLookup { .. } => "AuthorizedAnnotationLookup",
                    Kernel::VectorRank { .. } => "ExactVectorScan",
                    Kernel::LexicalMatch { .. } => "ExactLexicalScan",
                    Kernel::ContextPack { .. } => "ContextPack",
                    Kernel::MatchCompact {
                        shortest: false, ..
                    }
                    | Kernel::MatchSequence {
                        shortest: false, ..
                    } => "BoundedMatch",
                    Kernel::MatchCompact { shortest: true, .. }
                    | Kernel::MatchSequence { shortest: true, .. } => "BoundedShortestMatch",
                    Kernel::Expand { .. } => "BoundedExpand",
                    Kernel::ExpandSequence { .. } => "BoundedExpandSequence",
                    Kernel::Filter(_) => "Filter",
                    Kernel::Project(_) => "Project",
                    Kernel::Distinct => "Distinct",
                    Kernel::Sort(_) => "StableSort",
                    Kernel::Take(_) => "Take",
                    Kernel::Offset(_) => "Offset",
                    Kernel::UnionAll => "UnionAll",
                    Kernel::Aggregate { .. } => "Aggregate",
                    Kernel::Join { .. } => "NestedLoopJoin",
                }
                .into(),
                inputs: n.inputs.iter().map(|i| self.nodes[*i].id.clone()).collect(),
                columns: columns(&n.columns),
                estimates: None,
                actual: None,
            })
            .collect()
    }
}
