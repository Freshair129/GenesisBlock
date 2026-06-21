# GENESISDB ROADMAP (MARK XI -> MARK XII)
**Positioning:** Embedded analytics / agent-memory graph + vector engine
(comparators: Kuzu, DuckDB+graph, RocksDB+graph; Neo4j/Qdrant as references).
**Master Specification:** [MASTER-SPEC--GENESIS-DB.md](docs/MASTER-SPEC--GENESIS-DB.md)
**Evidence:** [REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md](docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md) (audits P14–P25)

## Current Status (evidence-backed 2026-06-21)
- **Benchmark Credibility:** vector (vs Chroma/Qdrant + recall-latency frontier),
  graph (10k–1M, vs Neo4j), and cost proofs (governance, incremental K-Impact)
  — all measured and reproducible. Earlier "<30 µs / 120 TPS" figures retracted.
- **Production Readiness:** advanced prototype. Durable WAL + snapshot/replay
  verified; full suite green. Open: deferred indexing, edge-id interning RAM,
  gossip anti-entropy stub, `retract_edge` stub.
- **Core Architecture:** Neural Bridge, LPA Clustering, Merkle Sync, Logic-Gated Context, Consensus Protocol.
- **Temporal Engine:** Bitemporal Querying, Event Sourcing, Vector Drift Tracking, TTL.
- **Cognitive Layer:** Graph Retrieval Layer (GRL) with H0-H5 Scaling Protocol.
- **Distributed Intelligence:** CRDT Foundation, P2P Gossip, Logical Clocks.

---

## MARK VII: Temporal Reasoning & Event Sourcing (COMPLETED)
- [x] **Multi-Dimensional Temporal Queries:** HQL `AS OF` syntax for time-travel.
- [x] **Causality Chains (Event Sourcing):** `caused_by` audit logs and immutable `supersede_node` updates.
- [x] **State-Transition Reasoning:** Longitudinal vector drift tracking for theme evolution.
- [x] **Ephemeral Nodes & TTL:** Self-pruning task context and short-term memory atoms.

---

## MARK VIII: Distributed Intelligence (COMPLETED)
- [x] **Step 1: CRDT Foundation:** Logical Clocks (Lamport) and `reconcile_state` for eventual consistency.
- [x] **Step 1.5: Thai Fuzzy Hardening:** Thai-aware character-level indexing and typo-tolerant thresholds.
- [x] **Step 2: Graph Retrieval Layer (GRL):** Implementation of the H0-H5 Context Scaling Tier and HQL `CONTEXT` command.
- [x] **Step 3: P2P Gossip Protocol:** Peer discovery and decentralized state synchronization across agent swarms.

---

## MARK IX: System Hardening & Persistence (COMPLETED)
- [x] **Step 1: Serialization:** Instant-load binary indices and arenas (.bin).
- [x] **Step 2: Transactional Batching:** Atomic multi-event mutations via `execute_batch`.
- [x] **Step 3: Index Compaction:** Memory garbage collection and HNSW rebuilding.

---

## MARK X: Swarm Hardening & Cryptographic Identity [COMPLETED]
- [x] **Step 1: ed25519 Peer Identities:** Automated keypair generation and PeerID mapping.
- [x] **Step 2: Signed Mutations:** Digital signatures for every event (WAL & Gossip).
- [x] **Step 3: Quorum-based Governance:** Multi-signature approval for MASTER tier axioms.

---

## MARK XI: Enterprise Integration & Tooling (Current)
- [x] **Step 1: MCP Server:** Model Context Protocol implementation for LLM native tool integration. [Guide](docs/MCP-GUIDE.md)
- [x] **Step 2: Python SDK:** High-level bindings for Data Science and AI research. [Guide](docs/PYTHON-SDK-GUIDE.md)
- [x] **Step 3: Go SDK:** Official client for cloud-native infrastructure and high-performance backends.
- [ ] **Step 4: GKS Insight Dashboard:** Real-time visualization of swarm health and knowledge drift.

---

## MARK XII: Benchmark Evidence & Hardening (COMPLETED 2026-06-21)
- [x] **Vector vs Chroma/Qdrant** + recall–latency frontier (P15/P20/P21).
- [x] **Graph traversal 10k/100k/1M** — O(neighborhood) (P22).
- [x] **Neo4j head-to-head** — embedded 7–185× (P23).
- [x] **Governance & K-Impact cost** — guard <0.1%, incremental O(V_affected) (P24/P25).
- [x] **Engine perf:** −44% RAM, ×6.1 concurrent ingest, ×7.8 bulk insert, configurable HNSW `ef` (P14/P16–P19).
- [x] **Interactive dashboard:** `docs/perf-comparison-dashboard.html`.

## MARK XIII: Next (proposed)
- [x] **Kuzu head-to-head** (embedded↔embedded, P26): GenesisBlockDB wins traversal
  latency 7–166×; Kuzu wins ingest ~60× & memory ~11× — different sweet spots.
- [ ] **Edge-id interning rework** (u64 ids) to push graph scale past 1M / 32 GB.
- [ ] **Deferred/async indexing** to keep query latency flat during bulk load.
- [ ] **Multi-collection vector space** (per-model/dim; [SPEC](docs/SPEC--MULTI-COLLECTION-VECTOR-SPACE.md)).
- [x] **Doc hygiene:** `API_REFERENCE.md` regenerated from `main.rs`; version SSOT
  ([VERSION.md](docs/VERSION.md)); status index ([DOC-STATUS.md](docs/DOC-STATUS.md)).
