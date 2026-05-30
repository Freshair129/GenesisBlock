---
id: ADR--GENESISDB-GOVERNANCE-LOGIC
phase: 3
type: adr
status: proposed
vault_id: GKS-CORE
tier: process
source_type: learned
title: "ADR: Formal Axiomatic System and Transitive Contradiction Checking"
tags: [architecture, genesisdb, governance, logic, constraints]
aliases: [genesisdb-axiomatic-logic]
attributes:
  domain: governance-engine
---

# ADR--GENESISDB-GOVERNANCE-LOGIC

## 1. Context
Initial versions of GenesisDB implemented "Axiomatic Governance" merely as a Tier Permission Rule (e.g., `Tier(source) >= Tier(target)`). A true Axiomatic System requires logical consistency checking to prevent the Knowledge Graph from holding mutually exclusive states or cyclic paradoxes, which would cause an AI agent to hallucinate or deadlock during reasoning.

## 2. Decision
We elevate the governance layer from permission checks to a **Formal Axiomatic Evaluation Model** executing synchronously on the write-path.

### 2.1 The Contradiction Graph Validation
Before an edge of type `supersedes` or `contradicts` is committed, the engine must prove it does not violate Transitive Logic or create paradoxes.
*   **Transitive Check:** If Node A `contradicts` Node B, and Node B `implies` Node C, the system enforces that Node A cannot `imply` Node C.
*   **Cycle Detection:** `supersedes` edges must form a Strict Directed Acyclic Graph (DAG). An edge insertion that creates a cycle (A supersedes B supersedes A) is rejected with an `AxiomaticParadoxError`.

### 2.2 BitSet Reachability Engine
To execute logical consistency checks without blocking writes for too long:
*   The engine maintains a transient `BitSet` of ancestors and descendants for active sub-graphs.
*   Reachability checks (e.g., "Is A a descendant of B in the supersedes subgraph?") are reduced to SIMD-optimized bitwise AND operations (`A.ancestors & B.id_mask`), completing in `< 2µs`.

### 2.3 Policy Inheritance
Lower-tier nodes (e.g., `FEAT--`) explicitly inherit the constraints of their higher-tier ancestors (e.g., `MASTER--`). An action performed on a leaf node that violates an inherited constraint is blocked.

## 3. Status
**Proposed**

## 4. Consequences
*   **Positive:** GenesisDB becomes a true "Logical Knowledge Engine," not just a graph store. It guarantees non-contradictory context delivery to AI models.
*   **Negative:** Write latency for `supersedes` and `contradicts` edge types increases by $O(V_{subgraph})$. This necessitates capping the depth of transitive checks to a hard limit (e.g., 5 levels) to guarantee latency bounds.
