# Technical Design: Transitive Inference & Virtual Edges (Mark IV - Step 4)

## 1. Objective
Introduce "Transitive Inference" to allow GenesisDB to deduce implicit relationships that are not physically stored in the Write-Ahead Log. This enables high-level semantic queries like "Who is in my Org Chart?" based on low-level links like "Who do I report to?".

## 2. HQL Grammar Expansion (`hql.pest`)
Introduce the `INFER` keyword as a modifier for traversals.

**Proposed Syntax:**
- `TRAVERSE FROM "NodeA" DEPTH 5 REL INFER(REPORTS_TO)`
- This query will return all nodes reachable via physical `REPORTS_TO` edges AND virtual derived edges.

## 3. Inference Rule Engine
We will implement a simple "Virtual Rule" registry.

### 3.1 Pilot Rule: `TransitiveClosure`
- **Trigger Relation:** `REPORTS_TO`
- **Inferred Relation:** `IN_ORG_CHART`
- **Logic:** For any path `(A)-[REPORTS_TO]->(B)-[REPORTS_TO]->(C)`, create a virtual edge `(A)-[IN_ORG_CHART]->(C)`.

## 4. Traversal Refactor (`src/lib.rs`)
The `neighbors` method will be updated to:
1.  Check if the requested `rel` is a virtual relation (e.g., `IN_ORG_CHART`).
2.  If yes, look up the base relation (`REPORTS_TO`).
3.  Perform a recursive BFS to any depth until the leaf nodes are found.
4.  Return the results as `NeighborOutput` with `path` containing the chain of physical edges.

## 5. Performance Standard
- **Constraint:** Virtual edges are calculated **JIT (Just-In-Time)** during query execution to avoid "Inference Explosion" in the WAL.
- **Optimization:** Use a BitSet for visited nodes during virtual recursion to maintain $O(V+E)$ complexity.

## 6. Implementation Steps
1.  **Step 1:** Update `hql.pest` to support `INFER(...)`.
2.  **Step 2:** Add `InferenceRule` registry to the `Storage` struct.
3.  **Step 3:** Refactor `neighbors` to support recursive virtual expansion.
4.  **Step 4:** Add unit test `test_transitive_inference` in `tests/reasoning_tests.rs`.

Please review and approve this technical design. I will proceed with the HQL and engine updates once approved.
