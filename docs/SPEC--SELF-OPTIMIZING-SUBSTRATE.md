# Software Requirements Document (SRD): Self-Optimizing Substrate

## 1. Introduction
The **Self-Optimizing Substrate** is the autonomic nervous system of GenesisDB (Mark VI). As the knowledge graph grows through Shadow Sync and AI interactions, it may accumulate redundancy or fragmented indexes. This module automates the maintenance tasks required to sustain peak performance and logical clarity.

## 2. Functional Requirements

### FR1: Stagnant Node Consolidation
- **Requirement:** Identify nodes with low `K-Impact` and no recent mutations/reads.
- **Logic:** Periodically check the `timestamp` in `NodeMetadata`.
- **Action:** Archive or merge low-authority, stagnant nodes into their closest high-impact neighbor (based on semantic similarity).

### FR2: Dynamic Index Re-sharding
- **Requirement:** Automatically adjust HNSW parameters based on graph density.
- **Logic:** If `member_count` in a `META_CLUSTER` exceeds a threshold, trigger a localized index re-build with higher `ef_construction` for that specific shingle.

### FR3: Knowledge Pruning (Entropy Control)
- **Requirement:** Remove "Dead-End" edges or orphaned nodes that don't contribute to any `MASTER` tier reasoning.
- **Goal:** Maintain high signal-to-noise ratio in the context window.

## 3. Performance Requirements
- **Background Execution:** Maintenance tasks must run on a low-priority thread to avoid impacting real-time HQL query latency (< 30µs).
- **Graceful Degradation:** Use the `is_rebuilding` flag to signal maintenance states to the Obsidian UI.

---

# Technical Design Document (TDD): Autonomic Maintenance Engine

## 1. Architecture: The Maintenance Loop
The maintenance engine will be a dedicated background loop spawned during `Storage::open`.

## 2. Data Structures

### 2.1 OptimizationTask
```rust
pub enum OptimizationTask {
    ConsolidateStagnant,
    RebuildClusterIndex(u32),
    PruneEntropy,
}
```

## 3. Algorithm: Stagnant Consolidation
1.  **Scan:** Iterate through `nodes` where `impact < 0.3` and `last_seen > 30 days`.
2.  **Match:** Find the closest `MASTER` or `SPEC` node using `hybrid_search`.
3.  **Merge:** Transfer properties to the target node and delete the stagnant node.
4.  **Log:** Record the merge event in the WAL.

## 4. Implementation Plan
- **Step 1:** Implement the `AutonomicLoop` thread in `src/lib.rs`.
- **Step 2:** Implement the `prune_orphaned_nodes()` method.
- **Step 3:** Integrate optimization triggers into `rebuild_index_parallel`.

---
**Please review and approve this documentation (SRD & TDD). I will generate the code once approved.**
