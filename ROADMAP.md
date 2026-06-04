# GENESISDB ROADMAP (MARK IV -> MARK V)
**Positioning:** Local Hybrid Knowledge Engine for AI Agents

## Current Status
- **Engineering Quality:** 10/10 (Clean Code, Fully Tested, Trigram Optimized)
- **Production Readiness:** 9/10 (WAL Group Commit, HNSW, AST HQL, Fuzzy ID)
- **Core Architecture:** Trigram Indexing, Axiomatic Guards, K-Impact Reasoning, Transitive Inference.
- **Verified Benchmark:** 121 TPS (Complex Ingestion) / < 40µs Latency (Query).

---

## Phase 16: Verifiable Reasoning (MARK IV - COMPLETED)
- [x] **WAL Group Commit (JSONL):** Durable high-throughput batching.
- [x] **AST Query Planner:** Formal HQL parsing via Pest.
- [x] **Trigram Fuzzy Search:** O(1) candidate lookup for Typo Tolerance.
- [x] **Axiomatic Guards:** Data governance and MASTER tier protection.
- [x] **K-Impact Engine:** Dynamic node utility ranking ($U(n)$).
- [x] **Transitive Inference:** Recursive relationship deduction (`INFER` keyword).

---

## Phase 17: Advanced Neural Integration (MARK V - UPCOMING)
*The focus shifts to collaborative reasoning and cross-lingual knowledge mapping.*

### 1. Cross-Lingual Knowledge Mapping (Neural Bridge)
- **Objective:** Support seamless Thai-English knowledge retrieval.
- **Implementation:** Implement a "Semantic Normalizer" that maps heterogeneous embedding models into a unified canonical space.
- **Goal:** A query in Thai can resolve concepts stored in English notes and vice-versa.

### 2. Automatic Graph Clustering (Unsupervised Reasoning)
- **Objective:** Discover implicit "Knowledge Atoms" without human tagging.
- **Implementation:** Integrate the **Louvain Method** or **Label Propagation** for real-time community detection.
- **Output:** Virtual "Cluster" labels applied to related nodes for faster JIT context assembly.

### 3. Collaborative WAL (Decentralized Sync)
- **Objective:** Enable multiple AI agents to sync their "Brains" across different hosts.
- **Implementation:** Develop a **Gossip Protocol** or **Merkle Tree-based WAL Reconciliation**.
- **Goal:** Ensure "Axioms" propagated by Agent A are verified and ingested by Agent B without conflicts.

### 4. Logic-Gated Context Windows
- **Objective:** Provide a "Ranked Context" for LLMs based on K-Impact.
- **Implementation:** A specific endpoint `/v1/reason/context` that returns a flat list of nodes ordered by $(HybridSimilarity \times ImpactScore)$.
