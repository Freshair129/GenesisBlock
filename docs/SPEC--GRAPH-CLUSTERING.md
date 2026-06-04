# Functional Specification: Automatic Graph Clustering (Mark V)

## 1. Objective
Enable **Unsupervised Reasoning** via Automatic Graph Clustering. The goal is to allow GenesisDB to discover implicit "Knowledge Communities" and automatically group related nodes without requiring human-curated tags or labels.

## 2. Technical Approach: Label Propagation Algorithm (LPA)
Given the real-time, highly mutable nature of Obsidian vaults, we will implement a synchronous **Label Propagation Algorithm (LPA)**. It is fast, operates well on dynamic graphs, and scales linearly $O(V+E)$.

### 2.1 Algorithm Flow
1.  **Initialization:** Every node starts with a unique `cluster_id` (its own `u32` ID).
2.  **Propagation:** During a clustering pass, each node updates its `cluster_id` to the most frequent `cluster_id` among its immediate neighbors. Ties are broken randomly or by K-Impact score.
3.  **Convergence:** The process repeats until no `cluster_id` changes or a maximum iteration limit (e.g., 5) is reached.

### 2.2 Trigger Mechanism
- **Background Maintenance:** Clustering shouldn't block real-time ingestion. It will be triggered manually via a new FFI method `cluster_graph()` or integrated into the existing `rebuild_index_parallel` background task.

## 3. Data Structure Changes
- **NodeMetadata:** Add a `cluster_id: u32` field.
- **NeighborOutput:** Expose the `cluster_id` to the client so the UI can visually group nodes.

## 4. Proposed Changes (`src/lib.rs`)
1.  Update `NodeMetadata` to include `pub cluster_id: u32`.
2.  Implement `pub fn detect_communities(&self) -> Result<()>`.
3.  Expose `detect_communities` via NAPI.

## 5. Value Proposition for AI Agents
When an agent queries a node, knowing its `cluster_id` allows the agent to instantly fetch the entire "Knowledge Community" for JIT context, significantly improving RAG (Retrieval-Augmented Generation) coherence.

## 6. Implementation Roadmap
1.  **Step 1:** Update `NodeMetadata` and internal state initialization.
2.  **Step 2:** Implement the `detect_communities` LPA logic.
3.  **Step 3:** Add an FFI endpoint for triggering clustering.
4.  **Step 4:** Add a benchmark/test to verify cluster convergence.

Please review and approve this specification. I will generate the code once approved.
