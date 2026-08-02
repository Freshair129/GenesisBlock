---
status: current
---

# Whitepaper: Genesis Knowledge System (GKS)
**Version:** 2.0.0 (Mark VIII) · evidence-backed revision 2026-06-21
**Status:** Embedded agent-memory engine — benchmarked (audits P14–P25)
**Architect:** Rwang (อาหวัง)

## 1. Executive Summary
The Genesis Knowledge System (GKS) is a high-performance distributed semantic substrate designed for **Human-Machine Autonomy**. It serves as the unified "Shadow Brain" where human thought (Obsidian) and machine reasoning (AI Agents) merge. By Mark VIII, GKS has evolved from a local engine into a decentralized, bitemporal ecosystem capable of collective consensus and eventual consistency across high-latency networks.

## 2. Core Architecture: The "Mechanical Sympathy" Engine

### 2.1 Storage Model (WAL & CRDT)
- **L0 Durability:** JSONL Write-Ahead Log with **Group Commit** logic (< 5ms latency).
- **L3 Collaboration:** Conflict-free Replicated Data Types (**CRDT**) using **Lamport Timestamps**. Every mutation is deterministically ordered across agents, enabling masterless synchronization.

### 2.2 Neural-Hybrid Indexing
- **Vector substrate:** HNSW similarity search. Measured (bge-m3 1024-dim, 100k):
  recall@10 0.984 at ~1.1 ms p50 — at parity with Chroma on the same
  recall↔latency frontier (audit P21/P20). *(The earlier "P95 < 30 µs" was a
  measurement artifact and is retracted; 30 µs is closer to a 1-hop graph
  lookup — see below.)*
- **Graph substrate:** index-backed traversal; 1-hop p50 ~22 µs, O(neighborhood)
  not O(N), and 7–185× faster than server Neo4j on k-hop (audit P22/P23).
- **Lexical substrate:** **Thai-aware Tokenization** filtering combining marks (vowels/tones) to unify fuzzy lookups (e.g., "บ้าน" vs "บาน").
- **Neural Bridge:** Cross-lingual English-Thai mean-centering for language-agnostic retrieval.

## 3. Distributed Reasoning Pillars (Mark VI - VIII)

### 3.1 K-Impact Score ($R(n)$)
Evaluates the authority of knowledge atoms based on graph topology (Dependency Depth), governance tier (Axiomatic Strictness), and stability metadata.

### 3.2 Structural Insight Engine
A proactive reasoning loop that analyzes the meta-graph to:
- **Detect Community Centroids:** Grouping atoms into emergent themes.
- **Identify Logical Gaps:** Prompting agents to bridge semantically close but disconnected ideas.
- **Track Semantic Drift:** Measuring vector drift of themes over time to observe the evolution of knowledge consensus.

### 3.3 Bitemporal Event Sourcing
Absolute auditability via the **Causality Chain**. Updates follow a "supersession" pattern where the history of every node is preserved. The `caused_by` field links mutations to triggering events like ADRs or Consensus Votes.

## 4. Governance: The Axiomatic Guard Protocol
A strict hierarchy of truth:
- **MASTER (Tier 0):** Immutable system axioms. Can only be modified via **Multi-Agent Neural Consensus**.
- **SPEC / ADR (Tier 1-2):** Formal specifications and architecture decisions.
- **USER (Tier 3):** Exploratory data and personal notes.

## 5. Deployment: The Collaborative Swarm
- **Shadow Sync:** The Obsidian plugin provides human-centric visualization and real-time vault indexing.
- **Agentic API:** Axum REST and NAPI bindings for seamless LLM context injection.
- **P2P Gossip:** Future-ready infrastructure for decentralized knowledge replication across agent swarms.

## 6. Conclusion
The Genesis Knowledge System is the infrastructure for **Collective Intelligence**. It provides the speed of local hardware, the flexibility of neural models, and the rigor of axiomatic governance, forming a resilient and evolving memory for the future of AI.

## 7. Evidence
Performance and competitive claims are measured and reproducible — see
`REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md` and audits P14–P25
(vector frontier vs Chroma/Qdrant, graph traversal 10k–1M, Neo4j head-to-head,
governance & incremental K-Impact cost). Harnesses live in `benches/`.

---
**Benchmarked, not narrated. Hardened for Thai. Embedded by design.**
