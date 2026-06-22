pub mod ast;
pub use ast::HqlCommand;

// NOTE: `execute_hql` (in src/lib.rs) pattern-matches `HqlCommand` and dispatches
// to Storage methods directly — there is no intermediate logical plan. The old
// `LogicalPlanner` / `QueryPlan` / `PlanStep` types were dead code (never called)
// and were removed in MARK XIV. If a cost-based planner is reintroduced, add it
// here and route `execute_hql` through it.
