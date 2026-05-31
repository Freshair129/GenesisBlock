# FLOW--GENESIS-BLOCK-DATAFLOW

## 1. Data Ingress & Routing
How commands and data enter the native core.

- **FFI Path:** Node.js -> napi-rs bridge -> Rust Structs (NodeInput, EdgeInput).
- **Network Path:** External Client -> HTTP POST (Axum) -> JSON Deserialization -> Rust Structs.

## 2. Memory Architecture (The Hybrid Arena)
GenesisDB uses a contiguous memory model for 'Mechanical Sympathy'.

- **Vector Arena (f32):** Aligned to 64-byte boundaries (16 f32 elements). Ensures one vector per cache line for optimal SIMD/AVX retrieval.
- **Metadata Arena:** Dense array of \NodeMetadata\ (ArenaID, NodeID, Offsets). Optimized for O(1) attribute lookup during re-ranking.

## 3. Indexing & Processing Flow
- **Semantic Path:** Query Vector -> HNSW Graph (Navigation) -> Top-K Arena IDs.
- **Structural Path:** Node ID -> CSR Adjacency List (BFS/DFS) -> Connected Components.
- **Scoring Path:** SIMD Dot Product + K-Impact Weighted Sum -> Blended Result Slices.

## 4. Persistence Cycle
- **WAL (Hot):** Event Serialization -> JSONL Append (Synchronous).
- **Snapshot (Warm):** Bincode Serialization -> Full Binary Image (Asynchronous/Compact).
