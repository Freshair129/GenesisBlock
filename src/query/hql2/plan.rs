//! Deterministic scalar planning; no indexes, rewrites or data operators open.
use super::{
    bind::{BoundNode, BoundQueryV2, Column, Kernel},
    error::QueryErrorV2,
    result::{ColumnV2, ExplainNodeV2},
};

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
    pub(crate) fn explain_nodes(&self) -> Vec<ExplainNodeV2> {
        self.nodes
            .iter()
            .map(|n| ExplainNodeV2 {
                id: n.id.clone(),
                logical_op: n.op.clone(),
                physical_op: match n.kernel {
                    Kernel::Values(_) => "Values",
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
