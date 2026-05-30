---
id: ALGO--KIMPACT-CALCULATION
phase: 2
type: algo
status: stable
vault_id: GKS-CORE
tier: process
source_type: axiomatic
title: "Algorithm: Recursive K-Impact Scoring Engine"
tags: [gks, algo, k-impact, ranking, graph-theory]
aliases: [k-impact-algorithm]
created_at: 2026-05-30T03:00:00+07:00
crosslinks:
  references: [SPEC--K-IMPACT, ADR--GENESISDB-KIMPACT-ALGORITHM, GENESIS--BACKEND-ENGINE]
attributes:
  complexity: O(V_affected + E_affected)
---

# ALGO--KIMPACT-CALCULATION

## 1. Overview
This algorithm implements the Knowledge Impact (K-Impact) scoring for the GenesisDB engine. It calculates the relative importance of a node within a Knowledge Graph using structural depth, axiomatic authority, and stability metrics.

## 2. Core Equation
For any node $n$:
$$ K\_Impact(n) = (DD(n) \cdot 0.5) + (AS(n) \cdot 0.3) + (SC(n) \cdot 0.2) $$

### 2.1 Dependency Depth (DD) Calculation
$DD(n)$ measures the node's recursive influence in the DAG.
1.  **Direct In-degree Factor ($F_{in}$):** $1 - e^{-0.1 \cdot |E_{in}(n)|}$
2.  **Recursive Depth Factor ($F_{depth}$):** $\frac{MaxRecursiveDepth(n, \text{limit}=10)}{SystemMaxDepth}$
3.  **Final DD:** $\max(F_{in}, F_{depth})$

## 3. Implementation Logic (Pseudocode)

```rust
fn calculate_k_impact(node_id: String, storage: &Storage) -> f64 {
    let node = storage.get_node(&node_id);
    
    // 1. AS (Axiomatic Strictness) from Tier
    let as_score = match node.tier {
        Tier::Master => 1.0,
        Tier::Concept => 0.8,
        Tier::Feat => 0.6,
        _ => 0.3,
    };

    // 2. SC (Stability Confidence) from Status
    let sc_score = match node.status {
        Status::Stable => 1.0,
        Status::Active => 0.8,
        Status::Draft => 0.4,
        Status::Deprecated => 0.1,
    };

    // 3. DD (Dependency Depth)
    let in_degree = storage.get_incoming_edges(&node_id).len();
    let f_in = 1.0 - (-0.1 * in_degree as f64).exp();
    let f_depth = storage.get_recursive_depth(&node_id, 10) as f64 / 10.0;
    let dd_score = f_in.max(f_depth);

    (dd_score * 0.5) + (as_score * 0.3) + (sc_score * 0.2)
}

fn trigger_incremental_refresh(target_node_id: String, storage: &mut Storage) {
    let mut affected = storage.get_downstream_dependents(&target_node_id);
    affected.push(target_node_id);
    
    for id in affected {
        let new_score = calculate_k_impact(id, storage);
        storage.update_node_impact(id, new_score);
    }
}
```

## 4. Optimization: BitSet-based Depth
To prevent deep recursion, the engine utilizes pre-computed BitSets for reachability, allowing $F_{depth}$ to be estimated in $O(1)$ bitwise operations after an initial $O(E)$ batch compute.

---
### Related Links
- **Orchestrator:** [[GENESIS--BACKEND-ENGINE]]
- **K-Impact Rationale:** [[ADR--GENESISDB-KIMPACT-ALGORITHM]]
- **Base Specification:** [[SPEC--K-IMPACT]]
