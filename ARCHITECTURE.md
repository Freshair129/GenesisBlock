# Architecture: Mark VIII - Genesis Knowledge System (GKS)

For the authoritative technical specification, see: [MASTER-SPEC--GENESIS-DB.md](docs/MASTER-SPEC--GENESIS-DB.md)

## Positioning
GenesisDB is a **local-first hybrid knowledge engine** optimized for human-machine collaboration. It prioritizes **low-latency reasoning** and **distributed eventual consistency** via CRDTs.

## Core Architecture Layers

### 1. Storage & Persistence (L0)
- **WAL-Based Durability**: Every mutation is logged to `genesis-graph.wal` (JSONL).
- **Zero-Copy Memory Map**: Utilizes **DashMap** for thread-safe concurrent access.
- **Bitemporal Pattern**: Updates follow the "Retract and Re-insert" pattern for absolute auditability.

### 2. Semantic & Indexing (L1)
- **HNSW Vector Index**: High-speed similarity search for high-dimensional embeddings.
- **Thai-Aware Lexical Index**: Trigram indexing optimized for Thai character combining marks.
- **Neural Bridge**: Language-agnostic context matching via mean-centering.

### 3. Reasoning & Autonomic Loop (L2)
- **K-Impact Scoring**: Evaluates the authority of information based on connectivity and governance.
- **Community Detection (LPA)**: Automatically clusters related knowledge into themes.
- **Structural Insight**: Detects logical gaps and semantic drift over time.

### 4. Distributed Collaboration (L3)
- **Logical Clocks**: Lamport timestamps for deterministic event ordering.
- **CRDT Synchronization**: Reconciles divergent states across multiple agents without a central authority.
- **Neural Consensus**: Multi-agent voting protocol for promoting data to the "MASTER" axiom tier.

## Interface
- **HQL (Hybrid Query Language)**: A unified syntax for semantic search and graph traversal.
- **NAPI-RS**: Native asynchronous bindings for TypeScript/Obsidian.
- **Axum REST API**: Standardized endpoint for remote AI agents.
