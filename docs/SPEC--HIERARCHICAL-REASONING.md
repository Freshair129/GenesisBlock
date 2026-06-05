# Software Requirements Document (SRD): Hierarchical Reasoning

## 1. Introduction
**Hierarchical Reasoning** is the next evolutionary step for GenesisDB (Mark VI). While previous versions focused on individual nodes and clusters, this module enables the engine to reason about **Abstract Themes**. It transforms clusters of knowledge into "Super-Nodes," allowing AI agents to perform high-level strategic analysis across different domains of knowledge.

## 2. Functional Requirements

### FR1: Cluster Abstraction (Super-Nodes)
- **Requirement:** For every community identified by the LPA (Step 2), the engine must generate a virtual `META_CLUSTER` node.
- **Logic:** The Super-Node represents the collective identity of its member nodes.
- **Properties:** Must include `member_count`, `centroid_vector`, and `aggregate_impact`.

### FR2: Structural Summarization
- **Requirement:** Automatically derive "Theme Labels" for Super-Nodes.
- **Logic:** Use the most frequent labels and keywords from member nodes to describe the cluster.

### FR3: Inter-Cluster Relationship Deduction
- **Requirement:** Abstract physical edges into weighted "Meta-Edges."
- **Logic:** If Node A (Cluster 1) links to Node B (Cluster 2), a virtual edge is created between Super-Node 1 and Super-Node 2. The weight is determined by the total number of cross-cluster links.

### FR4: Meta-Graph Traversal
- **Requirement:** Expand HQL to support queries on the Meta-Graph.
- **Goal:** Enable queries like: "What broad themes are related to my current research cluster?"

---

# Technical Design Document (TDD): Hierarchical Abstraction Layer

## 1. Architecture: The Meta-Graph substrate
The Meta-Graph exists as a volatile, high-level index that sits on top of the physical storage.

## 2. Data Structures

### 2.1 SuperNode
```rust
pub struct SuperNode {
    pub cluster_id: u32,
    pub theme: String,
    pub members: Vec<u32>,
    pub impact: f64,
    pub embedding: Vec<f32>, // Centroid
}
```

### 2.2 MetaEdge
```rust
pub struct MetaEdge {
    pub from_cluster: u32,
    pub to_cluster: u32,
    pub weight: u32, // Count of underlying physical edges
}
```

## 3. Algorithm: Meta-Graph Assembly
1.  **Ingest Clusters:** Group nodes by `cluster_id`.
2.  **Synthesize Nodes:** For each group, calculate the centroid vector and average K-Impact.
3.  **Project Edges:** Iterate over physical edges. If an edge crosses cluster boundaries, increment the weight of the corresponding `MetaEdge`.
4.  **HNSW Meta-Index:** Index Super-Nodes into a separate, smaller HNSW index for high-level thematic search.

## 4. Implementation Plan
- **Step 1:** Implement the `SuperNode` and `MetaEdge` storage in `src/lib.rs`.
- **Step 2:** Implement the `generate_meta_graph()` background task.
- **Step 3:** Expose `/v1/reason/meta/query` REST endpoint.

---
**Please review and approve this documentation (SRD & TDD). I will generate the code once approved.**
