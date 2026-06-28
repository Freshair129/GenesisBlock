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
- **Production Readiness:** advanced prototype. Suite green: 34 test binaries /
  7 npm tests pass. MARK XIV closed all prior correctness stubs (`retract_edge`,
  gossip anti-entropy + `Event::Vector` sync + ordered Merkle, per-query &
  per-collection `ef_search`, GKS Dashboard + force-graph, SQ8/BQ + rerank).
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

## MARK XIV: Market Credibility & Scale Ceiling (COMPLETED 2026-06-23)

> **Theme:** Convert benchmark wins into market presence; push RAM ceiling beyond 1M nodes; close remaining correctness stubs.
>
> **Status:** all code items closed. P1 scale ceiling measured + node-id interning/quant
> shipped; P2 DX/positioning shipped; P3 per-collection `ef_search`; P4 f32-rerank +
> force-graph viz; P5 `Event::Vector` sync + ordered Merkle. Remaining items are
> compute/ops only: RSS probe 500k–2M ([runbook](benches/scripts/rss_probe.md)),
> SQ8/BQ recall on real data ([runbook](benches/scripts/recall_harness.md)), and enabling
> GitHub Pages for the published benchmark page.

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
- [x] **Per-query / per-collection `ef_search`:** **done** 2026-06-23 — per-query override
  (`HybridSearchInput.ef_search: Option<u32>`) plus a **per-collection default**
  (`create_collection(..., ef_search)`, persisted in the manifest). Resolution: per-query →
  per-collection → engine-global (PR #20). Motivated by the Recall@500k frontier
  ([AUDIT--P32](docs/AUDIT--P32-RECALL-500K-FRONTIER.md)): a single global ef can't serve
  100k (recall 0.982) and 500k (0.887) at once.

### Priority 4 — Vector Quality (🟢 ต่ำ)
- [x] **Scalar / binary quantization:** **SQ8 + BQ shipped** 2026-06-23. SQ8 (full
  resident cut — arena+HNSW u8, 4× RAM *and* disk). **BQ (binary)**: 1 sign bit/dim packed
  into u64 words + custom popcount `DistBinaryHamming` HNSW (anndists' u64 `DistHamming` is
  word-inequality), ~32× vector RAM/disk. Create with `quant: "bq"`.
  [ADR](docs/adr/ADR--GENESISDB-VECTOR-QUANTIZATION.md).
  **f32-sidecar rerank shipped** 2026-06-23 (PR #21): a quantized collection created with
  `rerank: true` keeps the exact f32 vectors in a `fvec_<name>.bin` sidecar, over-fetches
  quantized candidates, and re-scores them exactly — recovers recall (especially BQ, where
  same-sign vectors of different magnitude collapse to identical codes). Recall harness on
  real data still pending ([runbook](benches/scripts/recall_harness.md)).
- [x] **GKS Insight Dashboard** *(carried from MARK XI Step 4)*: **shipped** 2026-06-23.
  New read-only REST `GET /v1/insight/communities` (meta-graph SuperNodes + MetaEdges) and
  `GET /v1/insight/gaps` (structural gaps), plus `POST /v1/insight/rebuild` (recompute via
  `detect_communities`+`generate_meta_graph`). The dashboard's placeholder is replaced by an
  `InsightPanel` (community cards with size/impact/drift bars + structural-gap list + Rebuild),
  `useInsight` hook, `api` methods. `dashboard` `tsc -b && vite build` green.
  **Force-graph cluster viz shipped** 2026-06-23 (PR #22): a `CommunityGraph` (react-force-graph-2d)
  toggleable with the card list — SuperNodes sized by member count, tinted by drift; MetaEdges
  weighted; structural gaps overlaid as dashed links.

### Priority 5 — Distributed (🟢 ต่ำ; C-3 own session)
- [x] **Gossip `PullRequest` anti-entropy:** **shipped** 2026-06-23 — the PullRequest
  handler (was a `// TODO`) now replies with `events_since(from_clock)` (signed WAL events
  newer than the requester's clock, sorted by logical time, bounded to one UDP datagram).
  Peers apply via `reconcile_state` (LWW) and converge graph state over rounds; NAPI
  `events_since`. `tests/anti_entropy_tests.rs`.
- [x] **`Event::Vector` secondary embeddings in deltas:** **done** 2026-06-23 (PR #23) —
  `VectorEvent` gained a logical clock, so `events_since` now includes secondary
  embeddings; a peer auto-provisions the collection on receive. `compact()` preserves them
  and now writes readable `SignedEvent` lines (latent-bug fix).
- [x] **Exact Merkle convergence:** **done** 2026-06-23 (PR #24) — `get_merkle_root` is now an
  order-independent digest of the graph state (nodes + edges by id + clock + validity),
  so peers at the same state share a root regardless of WAL write order.

---

---

## MARK XVI: Mobile SDK & Embedded App (PROPOSED 2026-06-29)

> **Theme:** GenesisBlockDB as a first-class embedded mobile engine — local-first,
> in-process, no server required. Two artifacts: a standalone mobile app with graph +
> retriever UI (Level A), and a reusable SDK any developer can embed (Level B).
>
> **Spec:** [SPEC--MOBILE-SDK.md](docs/SPEC--MOBILE-SDK.md)
>
> **Complexity:** C-3. Phase 0 is a prerequisite for all subsequent phases.

### Phase 0 — Foundation (~1 week)
- [x] **`mobile` Cargo feature:** added `mobile` + `ffi` features; `sysinfo` made optional and owned by `bins` (it is bench-only, not used in `src/`). `cargo build --no-default-features --features mobile` exits 0 with no `sysinfo` compiled in
- [ ] **Cross-compile probe:** `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `aarch64-linux-android` all exit 0 _(in progress)_
- [ ] **C FFI layer (`src/ffi.rs`):** 7 core symbols (`open`, `close`, `add_node`, `execute_hql`, `search`, `retrieve_context`, `free_string`) with JSON-in/JSON-out contract _(in progress)_
- [ ] **WAL path injection:** document sandboxed path convention (`app_data_dir()` → DB root); no core change required — `Storage::open` already takes the DB root via `OpenOptions.path`

> **Note (2026-06-29):** 0-A done. The original plan assumed `sysinfo` was used inside `src/lib.rs` and needed `#[cfg(feature = "mobile")]` gating; a grep proved it is **bench-only** (used by `benches/{edge_interning_audit,graph_bench,vbench_genesis}.rs`), so the fix was purely `Cargo.toml` dependency wiring — no core code changed. 0-B and 0-C are being completed in parallel.

### Phase A — GenesisBlock Mobile App, Level A (~3 weeks)
- [ ] **Tauri v2 mobile project (`genesisblock-mobile/`):** iOS + Android targets, `genesis-block-native` as `--no-default-features --features mobile` dep
- [ ] **Tauri commands (`commands.rs`):** `add_node`, `search`, `execute_hql`, `retrieve_context`, `get_graph_snapshot`, `flush_index` — mirrors NAPI surface
- [ ] **Graph view:** sigma.js WebGL; node color = governance tier; only valid edges (bitemporal now); pinch-zoom + tap-to-inspect
- [ ] **Retriever panel:** HQL `CONTEXT` → H0–H5 tier cards; tap node on graph triggers retrieval; highlights result nodes
- [ ] **CRDT sync toggle:** mobile ↔ desktop sync via existing gossip layer (UDP; optional)
- [ ] **Device validation:** installs + runs on physical iPhone (arm64) and physical Android (arm64); WAL persists across app restarts

### Phase B — SDK for Other Apps, Level B (~6 weeks)
- [ ] **iOS xcframework + Swift wrapper:** static `.a` for `aarch64-apple-ios` + sim slice; `actor GenesisDB` async/await API; distributed via Swift Package Manager
- [ ] **Android `.aar` + Kotlin wrapper:** JNI bridge (`src/jni.rs`); `class GenesisDB` with coroutines API; distributed via Maven/Gradle
- [ ] **React Native package (`react-native-genesisdb`):** native module wrapping iOS xcframework + Android `.aar`; TypeScript types mirror `index.d.ts`; distributed via npm
- [ ] **Flutter plugin:** deferred — `flutter_rust_bridge` auto-generated Dart bindings; added when user demand confirmed

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
