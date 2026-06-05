# Software Requirements Document (SRD): Structural Insight Engine

## 1. Introduction
The **Structural Insight Engine** is an advanced analytical module for GenesisDB (Mark VI). Inspired by InfraNodus, it moves beyond retrieval-augmented generation (RAG) toward **Structural Thinking**. It identifies concepts, groups them into topical communities, and—most importantly—reveals "Gaps" in the user's knowledge graph.

## 2. Functional Requirements

### FR1: Automatic Concept Extraction (ACE)
- **Requirement:** The engine must automatically extract "Concept Nodes" from the raw text stored in a note's properties.
- **Logic:** Use term-frequency (TF) and co-occurrence within a sliding window to create internal edges between concepts.
- **Goal:** Transform a single note into a micro-network of ideas.

### FR2: Structural Gap Detection (SGD)
- **Requirement:** The engine must identify clusters of high-impact nodes that are logically close but physically disconnected.
- **Logic:** Calculate "Betweenness Centrality" and "Bridge Opportunities" between high K-Impact clusters.
- **Goal:** Surface suggestions like: "Concept A and Concept B are highly important but unrelated. How do they connect?"

### FR3: Knowledge Community Visualization
- **Requirement:** Expose a ranked list of "Topical Communities" derived via LPA, weighted by total cluster K-Impact.

## 3. User Experience (Obsidian Integration)
- **Sidebar Integration:** A "Mind Gaps" tab in the Genesis Sidebar.
- **Visual Feedback:** Highlighting potential "Bridge Notes" that the user should create.

---

# Technical Design Document (TDD): Thinking Module Implementation

## 1. Architecture: The Insight Pipeline
The insight engine will operate as a post-processor during the `rebuild_index_parallel` or `detect_communities` phase.

## 2. Data Structures

### 2.1 Virtual Concept Mapping
We will NOT pollute the physical WAL with millions of transient keywords. Instead, we use a **Volatile Concept Index**:
```rust
pub struct InsightEngine {
    pub concept_graph: DashMap<String, HashSet<String>>, // Concept -> Related Concepts
    pub cluster_gaps: Vec<GapSuggestion>,
}
```

### 2.2 Gap Detection Algorithm (JIT)
1.  **Cluster Centroids:** Calculate the centroid vector for each Community Cluster ($C_1, C_2, ...$).
2.  **Proximity Analysis:** Identify clusters whose centroids are semantically similar ($dist < 0.3$) but have zero physical edges connecting their member nodes.
3.  **Authority Weighting:** Only suggest gaps between clusters where the average $K-Impact > 0.6$.

## 3. Implementation Plan (Mark VI - Step 1)
- **Step 1:** Implement a simple keyword co-occurrence extractor in `src/lib.rs`.
- **Step 2:** Add `calculate_structural_gaps()` to the `Storage` struct.
- **Step 3:** Expose `/v1/reason/gaps` REST endpoint.

---
**Please review and approve this documentation (SRD & TDD). I will generate the code once approved.**
