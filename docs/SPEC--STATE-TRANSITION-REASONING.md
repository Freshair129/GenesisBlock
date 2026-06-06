# Software Requirements Document (SRD): State-Transition Reasoning (Mark VII, Step 3)

## 1. Introduction
**State-Transition Reasoning** enables GenesisDB to observe and reason about the evolution of knowledge structures. By snapshotting cluster centroids over time, the engine can detect "Semantic Drift"—the gradual movement of concepts in vector space. This allows AI agents to identify emerging trends, shifting consensus, or the merging of once-distinct knowledge domains.

## 2. Functional Requirements

### FR1: Meta-Graph Snapshotting
- **Requirement:** The system must periodically capture the state of all `SuperNodes` (cluster centroids, member counts, and themes).
- **Storage:** These snapshots must be persisted to enable historical comparison.

### FR2: Vector Drift Calculation
- **Requirement:** Provide a utility to calculate the semantic distance between two snapshots of the same cluster.
- **Metric:** `Drift = 1.0 - CosineSimilarity(Centroid_T_now, Centroid_T_prev)`.

### FR3: Trend Detection (Insight Engine)
- **Requirement:** The `Structural Insight Engine` must identify clusters that are "unstable" (high drift) or "converging" (multiple clusters moving toward a common centroid).

---

# Technical Design Document (TDD): State-Transition Engine

## 1. Data Structures (`src/lib.rs`)

### 1.1 `SuperNode` Expansion
```rust
#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SuperNode {
    pub cluster_id: u32,
    pub theme: String,
    pub member_count: u32,
    pub impact: f64,
    pub centroid: Vec<f64>,
    pub timestamp: String, // When this snapshot was taken
    pub drift: Option<f64>, // Distance from previous snapshot
}
```

### 1.2 `Storage` History Index
```rust
pub struct Storage {
    // ...
    pub meta_history: DashMap<u32, Vec<SuperNode>>, // ClusterID -> Historical Snapshots
}
```

## 2. Implementation Logic

### 2.1 Semantic Distance
Implement Cosine Similarity to measure the angular distance between high-dimensional vectors, providing a robust metric for "meaning shift" regardless of vector magnitude.

### 2.2 Autonomic Monitoring
In `perform_autonomic_optimization`, the engine will:
1. Generate current `SuperNodes`.
2. Compare them against the `meta_history`.
3. Calculate `drift`.
4. Store the results for trend analysis.

## 3. Implementation Plan
- **Step 1:** Update `SuperNode` struct and add `meta_history` to `Storage`.
- **Step 2:** Implement `cosine_similarity` in `src/lib.rs`.
- **Step 3:** Update `generate_meta_graph` to record history and calculate drift.
- **Step 4:** Add REST endpoint `/v1/insight/drift` to expose the history.
- **Step 5:** Add unit tests simulating cluster evolution.

---
**Please review and approve this documentation (SRD & TDD). I will generate the code once approved.**
