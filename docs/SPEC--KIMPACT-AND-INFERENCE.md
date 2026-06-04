# Functional Specification: K-Impact Engine & Transitive Inference (Mark IV)

## 1. Objective
Transition the engine from static storage to active reasoning by implementing the **K-Impact Model** for node ranking and **Transitive Inference** for relationship derivation. This ensures that AI agents receive the most authoritative and logically complete context.

## 2. K-Impact Engine
Replace the hardcoded `impact: Some(0.7)` with a dynamic calculation.

### 2.1 The Formula
$$ K\_Impact(n) = (DD(n) \cdot 0.5) + (AS(n) \cdot 0.3) + (SC(n) \cdot 0.2) $$

- **Dependency Depth (DD):** Normalized saturating function based on incoming edge count: $min(1.0, \text{incoming\_edges} / 10.0)$.
- **Axiomatic Strictness (AS):** Determined by governance tier:
    - `MASTER`: 1.0
    - `SPEC`: 0.8
    - `ADR`: 0.6
    - `USER`: 0.3
- **Stability Confidence (SC):** Derived from the `stability` property in `props` (default to `active: 0.8`):
    - `stable`: 1.0, `active`: 0.8, `draft`: 0.4, `deprecated`: 0.1

### 2.2 Trigger Mechanism
- **Initial:** Calculated during `add_node`.
- **Propagation:** When an edge is added, the `DD` of the target node changes, potentially triggering a recursive update of its neighborhood.

## 3. Transitive Inference (Virtual Edges)
Allow HQL to resolve "Grandparent" relationships for specific semantic relations.

### 3.1 Inference Rule: `REPORTS_TO`
If `Node A -[REPORTS_TO]-> Node B` and `Node B -[REPORTS_TO]-> Node C`, then a query for `IN_ORG_CHART` from `A` will return `C`.

### 3.2 Implementation
Update the `neighbors` traversal logic to check for "Virtual Rule" matches when a specific relation is requested but not physically found.

## 4. Proposed Changes
- **src/lib.rs:** 
    - Implement `calculate_k_impact(node_id)`.
    - Update `add_node` and `add_edge` to trigger impact re-calculation.
- **src/query/planner.rs (New):**
    - Introduce a basic inference rule engine.

## 5. Implementation Roadmap
1.  **Step 1:** Implement the K-Impact formula and properties parser.
2.  **Step 2:** Implement impact propagation via localized BFS.
3.  **Step 3:** Add Transitive Inference support for `REPORTS_TO` as a pilot.
4.  **Step 4:** Update `shadow-sync-stress` to verify impact propagation speeds.

Please review and approve this Reasoning & Inference Specification. I will proceed with implementation once approved.
