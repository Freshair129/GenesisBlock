# GenesisDB HNSW Hybrid Index Integration: SWE Design Document (v1.0)

**Document Status:** Draft Specification  
**Target Audience:** Core Systems Developers, Indexing Specialists  

---

## 1. Introduction and Rationale
This document specifies the integration of a Hierarchical Navigable Small World (HNSW) index into the GenesisDB storage and query pipeline. The primary goal is to enable high-performance semantic search (vector similarity) alongside existing structural graph traversals (adjacency searches). 

GenesisDB prioritizes in-memory dominance and ultra-low-latency traversal. The HNSW index will be an in-process data structure residing within the Node Arena memory space to eliminate IPC overhead.

## 2. Technical Architecture

### 2.1 Storage Layout & Memory Management
The `NodeMetadata` structure is augmented to include vector embeddings while maintaining cache locality.
*   **Contiguous Vector Storage:** All `vector_data` fields are stored contiguously in the `NodeArena` to maximize SIMD/Cache hits.
*   **Alignment:** `embedding_offset` is aligned to 64-byte cache line boundaries.
*   **Arena ID Mapping:** The HNSW structure uses the dense `u32 Arena ID` as the primary key.

### 2.2 HNSW Indexing Logic
*   **In-Memory Only:** The full HNSW graph structure (layers and neighbor indices) resides in RAM.
*   **Synchronous Mutation Hook:** Index updates occur during CREATE/UPDATE operations.
*   **Incremental Re-indexing:** Only affected neighbors are re-linked, maintaining amortized $O(1)$ mutation performance.

### 2.3 Zero-Copy FFI Contract
*   The `hybrid_search` FFI call accepts an input vector pointer (`f32*`) and returns a result buffer of `u32` Arena IDs.
*   Uses `napi_create_external_buffer` for zero-copy result passing to Node.js.

## 3. Requirements Specification

### 3.1 Functional Requirements (FR)
*   **FR 1.0 Vector Ingestion:** Support indexing of high-dimensional embeddings (up to 768 or 1536 dims).
*   **FR 2.0 Hybrid Query:** Execute queries combining vector similarity (HNSW) and metadata filtering (GKS Attributes).
*   **FR 3.0 Re-ranking:** Implement a scoring function that balances Vector Similarity with K-Impact.

### 3.2 Non-Functional Requirements (NFR)
*   **NFR 1.0 Performance:** P95 Search Latency < 50ms for 1M vectors.
*   **NFR 2.0 Scalability:** Support up to 100M vectors with linear memory scaling.
*   **NFR 3.0 Safety:** Atomicity of vector updates must match the WAL/Snapshot integrity standards of the engine.

## 4. Acceptance & Success Criteria

### 4.1 Acceptance Criteria (AC)
*   **Accuracy:** Reach at least 95% Recall@K compared to exhaustive search.
*   **Filtering:** Metadata filters (e.g., `tier: master`) must be enforced 100% correctly during vector traversal.
*   **Recovery:** Index must re-instantiate perfectly from binary snapshots + WAL replay.

### 4.2 Success Criteria (SC)
*   **Cognitive Speed:** 3-hop hybrid traversal must remain under 5ms, ensuring AI reasoning loops are not bottlenecked.
*   **Maintenance:** Engineering clarity allows a new developer to manage the index within 2 working days.
