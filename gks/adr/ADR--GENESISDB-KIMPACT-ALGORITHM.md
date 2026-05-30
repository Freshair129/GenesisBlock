---
id: ADR--GENESISDB-KIMPACT-ALGORITHM
phase: 2
type: adr
status: stable
vault_id: GKS-CORE
tier: process
source_type: learned
title: "ADR: Strategic Selection of K-Impact over generic Ranking Algorithms"
tags: [architecture, genesisdb, decision, ranking, k-impact]
aliases: [genesisdb-kimpact-rationale]
created_at: 2026-05-30T03:00:00+07:00
crosslinks:
  references: [GENESIS--BACKEND-ENGINE]
  implements: [ALGO--KIMPACT-CALCULATION]
attributes:
  domain: logic-engine
---

# ADR--GENESISDB-KIMPACT-ALGORITHM

## Context
Modern graph databases typically rely on PageRank or HITS for node ranking. However, for a Knowledge Graph (GKS), "popularity" (number of links) does not always equal "truth" or "architectural importance." A high-tier `MASTER` rule may have few incoming links but must outweigh a thousand low-tier `EPISODE` logs.

## Decision
We adopt the **K-Impact Model** as the primary ranking engine for GenesisDB. The technical execution of the formulas is delegated to [[ALGO--KIMPACT-CALCULATION]].

### 2.1 Rationale for Weighted Dimension
1.  **Structure (DD) > Authority (AS):** We allocate 50% weight to Dependency Depth because structural reliance is the strongest indicator of "Criticality."
2.  **Tiered Governance:** By including Axiomatic Strictness (30%), we guarantee that foundation-level rules (Master/Frame) are naturally prioritized in the AI's context window.
3.  **Stability over Novelty:** We allocate 20% to Stability Confidence to ensure that `stable` architectures are preferred over `draft` experiments during autonomous reasoning.

### 2.2 Rejection of Time-Decay (Pure Version)
We intentionally omitted exponential time-decay from the core formula to prevent "Sacred Knowledge" (like GKS Principles) from losing impact simply because they haven't been modified recently. Decay is instead handled at the Query/Context layer if needed.

## Consequences
*   **Positive:** Deterministic ranking aligned with GKS principles. Predictable results for AI reasoning loops.
*   **Negative:** Requires specialized logic outside of standard graph algorithms.
*   **Alignment:** Directly linked to implementation in [[ALGO--KIMPACT-CALCULATION]].

---
### Related Links
- **Orchestrator:** [[GENESIS--BACKEND-ENGINE]]
- **Impact Algorithm:** [[ALGO--KIMPACT-CALCULATION]]
