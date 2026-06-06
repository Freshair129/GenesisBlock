use crate::query::ast::{HqlCommand, HqlRel};

#[derive(Debug, Clone)]
pub enum PlanStep {
    VectorSearch { vector: Vec<f32>, k: usize },
    GraphTraversal { seed: String, depth: u32, rel: HqlRel, fuzzy: bool },
    ApplyKImpact { alpha: f64 },
}

pub struct QueryPlan {
    pub steps: Vec<PlanStep>,
}

pub struct LogicalPlanner;

impl LogicalPlanner {
    pub fn plan(command: HqlCommand) -> QueryPlan {
        let mut steps = Vec::new();
        match command {
            HqlCommand::Search { target: _, vector, k, fuzzy: _, lang: _, as_of: _ } => {
                steps.push(PlanStep::VectorSearch { 
                    vector: vector.into_iter().map(|v| v as f32).collect(), 
                    k: k as usize 
                });
            }
            HqlCommand::Traverse { seed, depth, rel, fuzzy, as_of: _ } => {
                steps.push(PlanStep::GraphTraversal { seed, depth, rel, fuzzy });
            }
            HqlCommand::Hybrid { target: _, vector, alpha, fuzzy: _, lang: _, as_of: _ } => {
                steps.push(PlanStep::VectorSearch { 
                    vector: vector.into_iter().map(|v| v as f32).collect(), 
                    k: 10 
                });
                steps.push(PlanStep::ApplyKImpact { alpha });
            }
        }
        QueryPlan { steps }
    }
}
