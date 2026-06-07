# GenesisDB: A Distributed Semantic-Graph Substrate for Multi-Agent Autonomy
**Whitepaper v2.0.0 - Towards Collective Intelligence**

## Abstract
GenesisDB evolves from a local embedded database into a **Distributed Semantic Substrate**. By synthesizing high-dimensional vector embeddings, structured graph relationships, and bitemporal event sourcing, GenesisDB provides the foundational memory layer for the **Genesis Knowledge System (GKS)**. This paper details the architecture of our **Neural Bridge (Thai-English)**, the **CRDT-based synchronization protocol**, and the **Autonomic Substrate** that enables self-optimizing knowledge structures.

---

## 1. The Multi-Agent Knowledge Problem
As AI systems shift from single-agent task executors to multi-agent swarms, the primary bottleneck is **Shared Context**. Existing solutions like Vector Databases provide similarity search but lack relational reasoning; Graph Databases provide relationships but struggle with the fuzzy nature of natural language and cross-lingual drift. 

**GenesisDB** solves this by unifying these dimensions into a single, microsecond-latency engine.

## 2. Core Innovations (Mark VIII)

### 2.1 The Thai-English Neural Bridge
Unlike standard databases, GenesisDB is **linguistically aware**. 
- **Mean-Centering:** We implement language-specific centroids to bridge the semantic gap between Thai and English embeddings. 
- **Thai-Aware Fuzzy Search:** Our lexical engine uses custom tokenization that handles Thai combining marks (vowels/tones), ensuring high-recall searching even in the presence of linguistic noise or typos.

### 2.2 Bitemporal Event Sourcing & Causality
GenesisDB replaces destructive updates with a **Causality Chain**. 
- **`supersede_node`:** Every state change creates a new version, preserving the old state with logical time (`valid_to`).
- **Auditability:** Every mutation is linked to a `caused_by` event (e.g., an agentic decision or an ADR), allowing the system to reason about *why* knowledge evolved.

### 2.3 Distributed Consistency (CRDT & Logical Clocks)
To support multi-agent collaboration without a central master, we implement:
- **Lamport Timestamps:** Ensuring a deterministic total ordering of events across the swarm.
- **LWW (Last-Write-Wins) CRDT:** Reconciling divergent graph states through a Merkle-tree based synchronization protocol.

## 3. The Thinking Substrate: Autonomic Maintenance
The engine is not passive; it possesses an **Autonomic Loop** that performs background reasoning:
1.  **Community Detection (LPA):** Automatically groups related atoms of knowledge into high-level themes.
2.  **Semantic Drift Tracking:** Measures the "Vector Drift" of these themes over time to identify shifting consensus or emerging concepts.
3.  **Structural Gap Detection:** Finds semantically related clusters that lack physical links, prompting agents to explore new logical connections.

## 4. Technical Performance
- **Query Latency:** < 30µs (HNSW + DashMap optimized).
- **Ingestion Throughput:** > 120 TPS (fsync-compliant, batched WAL).
- **Embedded Architecture:** Zero-copy memory access via NAPI-RS for high-performance host integration.

## 5. Conclusion: The Path to Axiomatic Truth
Through the **Consensus Protocol**, GenesisDB allows a swarm of agents to promote unverified "USER" data into "MASTER" axioms. This represents the final step in human-machine collaboration: a system that not only stores information but actively curates and evolves its own foundational truth.
