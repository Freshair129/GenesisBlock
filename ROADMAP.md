# GENESISDB ROADMAP (MARK XIII → MARK XIV)
**Positioning:** Embedded analytics / agent-memory graph + vector engine —
the only embedded database with graph + vector + bitemporal + CRDT + governance
in a single binary. Nearest comparators: Kuzu, DuckDB+graph, LanceDB, LadybugDB.
**Master Specification:** [MASTER-SPEC--GENESIS-DB.md](docs/MASTER-SPEC--GENESIS-DB.md)
**Evidence:** [REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md](docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md) (P14–P30, all 8 competitors measured)
**Metrics:** [METRICS-REVIEW--2026-06-22-WEEKLY.md](docs/METRICS-REVIEW--2026-06-22-WEEKLY.md)

## Current Status (evidence-backed 2026-06-22)
- **Benchmark Credibility:** 8/8 named competitors measured (P15–P30). Vector vs
  Chroma/Qdrant/LanceDB; graph vs Neo4j/Kuzu/DuckDB+graph/RocksDB+graph/LadybugDB.
  hop1 p50 = 21.6 µs; LadybugDB (closest on-niche competitor) = 3,637 µs (**168×**).
- **Production Readiness:** advanced prototype. Suite green: 23 binaries,
  49 cargo tests / 7 npm tests pass. Open: `retract_edge` stub, gossip
  anti-entropy, per-query `ef_search`, GKS Dashboard.
- **Core Architecture:** Neural Bridge, LPA Clustering, Merkle Sync,
  Logic-Gated Context, Consensus Protocol + ed25519 vote signatures (PR #6).
- **Temporal Engine:** Bitemporal Querying, Event Sourcing, Vector Drift Tracking, TTL.
- **Cognitive Layer:** Graph Retrieval Layer (GRL) H0–H5; HQL `IN <collection>` (PR #3).
- **Distributed Intelligence:** CRDT Foundation, P2P Gossip, Logical Clocks.
- **Vector:** Multi-collection per-model/dim isolation; `add_vector` multi-vector
  per node (PR #4); async HNSW indexing (10× P95 improvement); configurable `ef`.
- **Graph:** u128 edge keys (collision rate 9e-26, PR #7); edge-id numeric
  interning Layer A+B (−44% RAM on edges).

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

## MARK XI: Enterprise Integration & Tooling (MOSTLY COMPLETE)
- [x] **Step 1: MCP Server:** Model Context Protocol implementation for LLM native tool integration. [Guide](docs/MCP-GUIDE.md)
- [x] **Step 2: Python SDK:** High-level bindings for Data Science and AI research. [Guide](docs/PYTHON-SDK-GUIDE.md)
- [x] **Step 3: Go SDK:** Official client for cloud-native infrastructure and high-performance backends.
- [ ] **Step 4: GKS Insight Dashboard:** Real-time visualization of swarm health and knowledge drift. *(moved to MARK XIV)*

---

## MARK XII: Benchmark Evidence & Hardening (COMPLETED 2026-06-21)
- [x] **Vector vs Chroma/Qdrant** + recall–latency frontier (P15/P20/P21).
- [x] **Graph traversal 10k/100k/1M** — O(neighborhood) (P22).
- [x] **Neo4j head-to-head** — embedded 7–185× (P23).
- [x] **Governance & K-Impact cost** — guard <0.1%, incremental O(V_affected) (P24/P25).
- [x] **Engine perf:** −44% RAM, ×6.1 concurrent ingest, ×7.8 bulk insert, configurable HNSW `ef` (P14/P16–P19).
- [x] **Interactive dashboard:** `docs/perf-comparison-dashboard.html`.

---

## MARK XIII: Scale & Full Benchmark Coverage (COMPLETED 2026-06-22)

*ทุก item ใน MARK XIII merge แล้วใน session 2026-06-22 (6 PRs + 3 engine levers)*

### Benchmark Completions
- [x] **Kuzu head-to-head** (P26): GenesisBlockDB hop1 **7–166×** faster; Kuzu ingest **60×**, memory **11×** — different sweet spots documented. [Audit](docs/AUDIT--P26-KUZU-HEAD-TO-HEAD.md)
- [x] **LanceDB head-to-head** (P27): GenesisBlockDB point-query **~9×** faster; LanceDB trades latency for disk-scale. [Audit](docs/AUDIT--P27-LANCEDB-HEAD-TO-HEAD.md)
- [x] **DuckDB+graph head-to-head** (P28): hop1 **54×**, tied hop6 — validates deep-traversal architecture. [Audit](docs/AUDIT--P28-DUCKDB-GRAPH-HEAD-TO-HEAD.md)
- [x] **RocksDB+graph head-to-head** (P29): hop1 **effectively tied** — confirms µs latency is real vs raw KV adjacency. [Audit](docs/AUDIT--P29-ROCKSDB-GRAPH-HEAD-TO-HEAD.md)
- [x] **LadybugDB head-to-head** (P30, closest on-niche competitor — Kuzu fork, regulated industries): hop1 **168×**, hop3 **6.7×**, hop6 **13.5×**; no payload caveat. [Audit](docs/AUDIT--P30-LADYBUGDB-HEAD-TO-HEAD.md)

### Engine Scale Improvements (PRs #2–#7)
- [x] **HNSW capacity OOM fix** (PR #2): fixed 133 MB/index over-allocation bug.
- [x] **HQL `IN <collection>` clause** (PR #3): scope vector search to named collection.
- [x] **`add_vector` — multi-vector per node** (PR #4): one embedding per collection per node.
- [x] **Background thread hygiene** (PR #5): join indexing threads on Drop.
- [x] **Consensus vote signature verification** (PR #6): ed25519 validation on inbound votes — closes security gap.
- [x] **u128 edge keys** (PR #7): collision rate 1.7e-6 → 9e-26. [ADR](docs/adr/ADR--GENESISDB-EDGE-NUMERIC-KEYS.md)

### Engine Levers (session 2026-06-22)
- [x] **Edge-id interning Layer A+B**: drop edge UUID strings + trigram pollution → edge RAM −44% (965 MB → 600 MB @100k/800k). [ADR](docs/adr/ADR--GENESISDB-EDGE-ID-INTERNING.md)
- [x] **Async HNSW indexing**: move HNSW insert off write hot path → P95 query latency 6.31 → 0.60 ms (≈10×) under concurrent ingest. [ADR](docs/adr/ADR--GENESISDB-ASYNC-INDEXING.md)
- [x] **Multi-collection vector space**: per-model/dim isolation; snapshot = per-collection `.bin` + manifest. [ADR](docs/adr/ADR--GENESISDB-MULTI-COLLECTION.md) [Spec](docs/SPEC--MULTI-COLLECTION-VECTOR-SPACE.md)

### Documentation
- [x] **Doc hygiene:** `API_REFERENCE.md` regenerated; version SSOT ([VERSION.md](docs/VERSION.md)); status index ([DOC-STATUS.md](docs/DOC-STATUS.md)).
- [x] **Competitive Market Brief:** full landscape analysis + positioning ([Section 9, REPORT](docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md))
- [x] **Weekly Metrics Review:** first evidence-backed metrics review ([METRICS-REVIEW--2026-06-22-WEEKLY.md](docs/METRICS-REVIEW--2026-06-22-WEEKLY.md))

---

## MARK XIV: Market Credibility & Scale Ceiling (Proposed)

> **Theme:** Convert benchmark wins into market presence; push RAM ceiling beyond 1M nodes; close remaining correctness stubs.

### Priority 1 — Scale Ceiling (🔴 สูงมาก)
- [x] **Recall@500k sweep:** **done** 2026-06-23. Reproduced the regression at the global
  default (ef=200 → recall@10 0.887); recall recovers with ef_search — 0.940 @400, **0.973 @800**
  — at ~3.1× p50 latency (1458→4528µs). Conclusion: a single global ef can't serve both scales →
  motivates per-query ef_search (P3, shipped). Numbers in `gb_vbench_500k/frontier_results.json`.
- [x] **Node RAM ceiling — id interning:** A1 (drop `u32_to_id` reverse map) + A3
  (`trigram_index` → roaring) **shipped** 2026-06-23; **A2 (`NodeMetadata.node_id`→u32)
  shipped** 2026-06-23 — drops the per-vector × per-collection String copy; on-disk
  `meta_*.bin` format gated by a manifest `mv` flag, pre-A2 snapshots migrate on load.
  [ADR](docs/adr/ADR--GENESISDB-NODE-ID-INTERNING.md). RSS quantification at 500k–2M
  still pending a probe run.

### Priority 2 — Market Evidence (🟡 กลาง)
- [ ] **Public benchmark page:** publish P15–P30 results as a standalone page
  (HTML or hosted). Competitor-specific tables with honest caveats already written.
  *(1 day — content exists)*
- [ ] **TypeScript type definitions + quickstart:** `npm install genesis-block`
  with full TS types and a working "5-minute" example. NAPI moat is locked behind
  poor DX without this. *(3–5 days)*
- [ ] **Competitive positioning copy:** update README + docs/landing to lead with
  "verifiable agent memory" narrative and MASTER tier benchmark. *(1 day)*

### Priority 3 — Correctness & Technical Debt (🟡 กลาง)
- [x] **`retract_edge` implementation:** **done** 2026-06-23 — bitemporal soft-delete
  (sets `valid_to`, advances clock); `neighbors` hides retracted edges from the current view
  unless `include_invalid`; exposed at REST `/v1/edge/retract`. History preserved for time-travel.
- [x] **`LogicalPlanner` dead code removal:** **done** 2026-06-23 — removed the unused
  `LogicalPlanner`/`QueryPlan`/`PlanStep` from `src/query/mod.rs` (`execute_hql` dispatches directly).
- [~] **Per-query / per-collection `ef_search`:** per-query override **shipped** 2026-06-23
  (`HybridSearchInput.ef_search: Option<u32>`, falls back to the global). Per-collection default
  still open.

### Priority 4 — Vector Quality (🟢 ต่ำ)
- [x] **Scalar / binary quantization:** **SQ8 + BQ shipped** 2026-06-23. SQ8 (full
  resident cut — arena+HNSW u8, 4× RAM *and* disk). **BQ (binary)**: 1 sign bit/dim packed
  into u64 words + custom popcount `DistBinaryHamming` HNSW (anndists' u64 `DistHamming` is
  word-inequality), ~32× vector RAM/disk. Symmetric, no-rerank first cut (matching SQ8); an
  f32-sidecar rerank stage is a future lever. Create with `quant: "bq"`.
  [ADR](docs/adr/ADR--GENESISDB-VECTOR-QUANTIZATION.md). Recall harness on real data pending.
- [ ] **GKS Insight Dashboard** *(carried from MARK XI Step 4)*: real-time swarm
  health and knowledge drift visualization. *(1–2 weeks)*

### Priority 5 — Distributed (🟢 ต่ำ; C-3 own session)
- [x] **Gossip `PullRequest` anti-entropy:** **shipped** 2026-06-23 — the PullRequest
  handler (was a `// TODO`) now replies with `events_since(from_clock)` (signed WAL events
  newer than the requester's clock, sorted by logical time, bounded to one UDP datagram).
  Peers apply via `reconcile_state` (LWW) and converge graph state over rounds; NAPI
  `events_since`. `tests/anti_entropy_tests.rs`. *(Note: exact Merkle convergence needs
  ordered/version-vector digests — pre-existing limitation; `Event::Vector` secondary
  embeddings not yet in deltas.)*

---

## Roadmap Changes This Update (2026-06-22)

| Change | รายละเอียด |
|---|---|
| MARK XIII → **COMPLETED** | ทุก item done ใน 6 PRs + 3 engine levers (session 2026-06-22) |
| MARK XIV **proposed** | 9 items ใน 5 priority buckets พร้อม effort estimate |
| MARK XI Step 4 | Dashboard ย้ายไป MARK XIV Priority 4 (ไม่ได้ cut — แค่ sequence) |
| Evidence links | อัพเดทจาก P14–P25 → P14–P30 (8/8 competitors measured) |
| Positioning | อัพเดทให้รวม LadybugDB ในฐานะ closest on-niche competitor |
| Header | อัพเดทจาก "MARK XI → MARK XII" → "MARK XIII → MARK XIV" |
