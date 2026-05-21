# SPEC--K-IMPACT

- **ID:** SPEC--K-IMPACT
- **Phase:** 2 (Spec)
- **Status:** draft
- **Author:** Rwang (T2 Agent)
- **Date:** 2026-05-21
- **Ref:** [[CONCEPT--GENESIS-GRAPH-BACKEND]], [[MASTER--ATOM-CONTRADICTION-POLICY]]

## 1. Objective
The **K-Impact Index** is a cognitive weighting system for the Genesis Block native engine. It transforms a flat knowledge graph into a prioritized "Cognitive Map" by assigning a scalar impact score (0.0 to 1.0) to every Node and Edge.

Its primary role is to serve as a **Ranked Secondary Index** to:
1. **Prune Noise:** Filter out low-impact data during deep graph traversals.
2. **Prioritize Recall:** Complement vector search (`pgvector`) by boosting high-impact relevant nodes.
3. **Enforce Integrity:** Penalize data that contradicts Master Blocks (Axiomatic Alignment).

## 2. Core Dimensions
K-Impact is calculated using three primary dimensions, porting concepts from the legacy EVA 6.0 `k_impact_engine`.

### 2.1 Structural Impact (SI)
Measures the "Connectivity Density" of a node.
- **Formula:** `SI = log(1 + in_degree + out_degree) / log(1 + max_possible_density)`
- **Logic:** Nodes that are highly referenced or reference many others (e.g., central Concepts or Features) naturally carry more weight.

### 2.2 Axiomatic Alignment (AA)
Measures compliance with the system's "Ground Truth" (Master Blocks).
- **Formula:** `AA = 1.0` if no contradictions; `AA = -1.0` (Force Purge) if it violates a `MASTER--` rule.
- **Logic:** Even a highly connected node becomes "Impact 0" if it is proven wrong by an Axiom.

### 2.3 Temporal Recency (TR)
Measures the freshness of the knowledge.
- **Formula:** `TR = exp(-lambda * (current_time - valid_from))`
- **Logic:** Newer facts have higher immediate impact; older facts "decay" unless reinforced by new edges (Resonance).

## 3. The K-Impact Formula (Rust Engine)
The final index for a node `n` at time `t`:

`K_Impact(n, t) = clamp((SI * w1 + TR * w2) * AA, 0.0, 1.0)`

- **w1 (Structure Weight):** 0.6 (Prioritizes fundamental concepts)
- **w2 (Time Weight):** 0.4 (Prioritizes recent events)

## 4. Operational Integration
- **Index Storage:** K-Impact scores are cached in the native `Storage` struct and persisted in the JSONL log as metadata.
- **Query Time:** `MATCH (a)-[r*1..5]->(b)` will use K-Impact to explore high-score paths first and stop if a path's cumulative impact falls below a `min_impact` threshold.
- **API Extension:**
  ```typescript
  // Find top 10 most impactful nodes for a concept
  query({ rel: 'references', minImpact: 0.7, limit: 10 })
  ```

## 5. Non-Redundancy Guarantee
| Dimension | pgvector | K-Impact |
|---|---|---|
| **Input** | String/Embedding | Graph Topology + Time + Axioms |
| **Search** | "Find what is similar" | "Find what is important/true" |
| **State** | Static (per document) | Dynamic (changes as the graph grows) |

---

**Please review and approve this specification. I will proceed to implement the logic in the Rust engine and update the TS adapter once approved.**
