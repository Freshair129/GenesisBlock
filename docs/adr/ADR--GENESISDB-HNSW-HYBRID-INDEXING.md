---
proposed_id: ADR--GENESISDB-HNSW-HYBRID-INDEXING
type: adr
status: candidate
aliases:
  - ADR
phase: 2
tier: process
cluster: implementation_flow
role: Architecture decision record
enforcement_state: inactive
proposed_at: 2026-05-30T19:10:05.575Z
proposed_by: agent
rationale: Drafting detailed technical ADR based on SWE Design Doc.
---

# ADR: Strategic Adoption of HNSW for Embedded Vector Indexing

## 1. Context\nGenesisDB is fundamentally optimized for in-memory dominance and ultra-low-latency graph traversals. The integration of high-dimensional semantic search (vector similarity) is required to enhance query capabilities, specifically enabling semantic search alongside existing structural graph searches (adjacency queries).\n\nThe primary challenge is integrating a complex, high-dimensional indexing structure (HNSW) into the existing in-memory NodeArena without introducing significant I/O bottlenecks (IPC) or compromising the required low-latency performance for both graph traversal and vector similarity search. The goal is to achieve hybrid search performance that maintains the system's core latency guarantees.\n\n## 2. Decision\nWe will integrate the Hierarchical Navigable Small World (HNSW) index directly into the NodeArena memory space, utilizing the dense u32 Arena ID for indexing. HNSW is selected as the indexing algorithm because it provides an optimal balance between high search recall (necessary for semantic search) and low search latency (necessary for real-time AI reasoning loops).\n\n### Rationale for HNSW:\n1. **Hybrid Search Suitability:** HNSW naturally supports the necessary hybrid query pattern: fast approximate nearest neighbor (ANN) search for vectors combined with metadata filtering (GKS Attributes).\n2. **Low Latency:** HNSW, by building a navigable graph structure, allows for sub-millisecond approximate nearest neighbor searches, directly supporting the NFR for \Cognitive
