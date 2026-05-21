# SPEC--K-IMPACT (Engineering Version)

- **ID:** SPEC--K-IMPACT
- **Phase:** 2 (Spec)
- **Status:** stable
- **Author:** Rwang (T2 Agent)
- **Date:** 2026-05-21
- **Ref:** [[CONCEPT--GENESIS-GRAPH-BACKEND]], [[FRAMEWORK--KNOWLEDGE-3-TIER]]

## 1. Objective
The **K-Impact Index** in this system is a purely engineering-centric metric designed for a **Code Agent** environment. It eliminates dependencies on physiological cores (`ri`, `rim`) and instead measures the architectural weight, logical authority, and stability of knowledge within the GKS graph.

## 2. Core Dimensions (Digital Physiology)

### 2.1 Dependency Depth (DD)
Measures the structural importance of a node based on how much the system relies on it.
- **Calculation:** Derived from the recursive count of incoming `references`, `implements`, and `depends_on` edges.
- **Scale:** 0.0 (Leaf node/Utility) to 1.0 (Core Framework/Base Library).

### 2.2 Axiomatic Strictness (AS)
Measures the logical authority of the knowledge based on its Tier in the architecture.
- **Mapping:**
    - `Tier 0 (MASTER-- / FRAME--)`: 1.0 (Sacred/Immutable)
    - `Tier 1 (CONCEPT-- / SPEC--)`: 0.8 (Stable Logic)
    - `Tier 2 (FEAT-- / ADR--)`: 0.6 (Intent/Implementation)
    - `Tier 3 (LOG-- / EPISODE--)`: 0.3 (Transient Data)
- **Scale:** Fixed scalar based on atom prefix.

### 2.3 Stability Confidence (SC)
Measures the reliability of the information based on its lifecycle status.
- **Mapping:**
    - `status: stable`: 1.0
    - `status: active`: 0.8
    - `status: draft`: 0.4
    - `status: deprecated`: 0.1
- **Scale:** Fixed scalar based on `status:` frontmatter.

## 3. The K-Impact Formula (Rust Engine)

The final K-Impact score for any knowledge node `n` is calculated as a weighted average:

`K_Impact(n) = (DD * 0.5) + (AS * 0.3) + (SC * 0.2)`

### 3.1 Edge Impact
Edges inherit a portion of the impact from the nodes they connect, modulated by the relationship type.
- `supersedes` edges carry high impact (logical shift).
- `mentions` edges carry low impact (informational).

## 4. Implementation Strategy (P4.3)
1. **Metadata Ingestion:** Update the Rust `Storage` to parse `tier` and `status` from atom frontmatter during sync.
2. **Topology Analysis:** Implement a background routine in Rust to calculate recursive `DD` scores (Directed Acyclic Graph traversal).
3. **Index Persistence:** Store the calculated `K_Impact` in the JSONL log as a new event type `Event::ImpactUpdate`.
4. **Traversal Weighting:** Update `neighbors` and `cypher` engines to sort results by `K_Impact` by default.

## 5. Usage Example
A `MASTER--` atom with many `references` will have a `K_Impact` near **1.0**, ensuring it always appears at the top of query results and guides the agent's decision-making above all other data.

---

**Please review and approve this Engineering Version of the K-Impact spec. I will proceed with the Rust implementation once approved.**
