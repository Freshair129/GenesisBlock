---
proposed_id: ADR--GENESISDB-COMPETITIVE-SUPERIORITY
type: adr
status: current
tier: strategy
cluster: implementation_flow
role: "ADR — competitive-superiority audit & refinement plan: >=20% head-to-head vs same-tier, enterprise-parity floor, evidence-gated"
date: 2026-07-03
proposed_by: agent (multi-model workflow: Sonnet-5 workers -> Opus review gates -> Fable audit/final gate/integration)
deciders: Boss
related:
  - POSITIONING
  - REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE
  - AUDIT--P31-POST-MARKXIII-REGRESSION
  - AUDIT--P32-RECALL-500K-FRONTIER
  - AUDIT--P33-RSS-QUANT-MATRIX
  - AUDIT--ONDISK-RERANK-RSS
  - RCA--VECTOR-QUANTIZATION
  - PLAN--VECTOR-QUANTIZATION-REFINEMENT
---

# ADR: Competitive Superiority — Architecture, Data Flow, Performance & Refinement Plan

**Status:** Proposed
**Date:** 2026-07-03
**Deciders:** Boss
**Method:** 3-tier multi-model audit — 6 Claude Sonnet 5 evidence workers (code/audit-doc analysis + live web research), 8 Claude Opus adversarial review gates (every claim re-verified against primary sources: repo files opened, URLs fetched), Fable final gate + integration. 28 agents, ~2.5M tokens, 159 claims individually verdicted. **No number in this document is a guess** — every figure carries its source and conditions; refuted/stale claims were corrected or dropped at the gates (correction log in §9).

---

## บทสรุปผู้บริหาร (Thai Executive Summary)

- **จุดแข็งที่อ้างไว้ "จริง" แต่แคบกว่าที่ตลาดถูกเล่า**: hop1 latency ชนะจริง 189×/177×/54× (Kuzu/LadybugDB/DuckDB) แต่ POSITIONING.md เลือกโชว์เฉพาะแกนที่ชนะ — hop6 vs DuckDB คือ "เสมอ", RocksDB คือ "tied-class", และ "9× vs LanceDB" วัดที่ recall ต่ำกว่าคู่แข่ง (0.948 vs 0.998)
- **สองแกนที่ไม่มีใครแตะได้ทั้ง same-tier และ enterprise**: bitemporal model + ed25519-signed governance — ตรวจแล้วกับเอกสารคู่แข่งปัจจุบันทุกราย (รวม SurrealDB 3.0) ไม่มีใครมี — นี่คือ moat จริง
- **แผนที่การแข่งขันเปลี่ยนใหญ่**: Kuzu ตายแล้ว (Apple ซื้อทีม ต.ค. 2025, repo archived) → LadybugDB (v0.18.0, MIT) คือผู้สืบทอด; HelixDB (YC, Rust) คือคู่แข่ง live ที่**เรายังไม่เคยวัด head-to-head เลย**; Neo4j ปิดช่องว่าง hybrid search แล้ว (SEARCH clause GA 2026.02) ตรงตาม threat table; SurrealDB CRDT nightmare **ไม่เกิดขึ้น**
- **เป้า ≥20% same-tier**: ผ่านแล้ว 6 แกน (วัดจริง), แพ้จริง 4 แกน (bulk ingest 13–19×, RAM 7.1×, larger-than-memory ไม่มีเลย, recall ceiling −0.009), เสมอ 3 แกน, ยังไม่วัด 3 แกน (HelixDB!)
- **เป้า enterprise "ห้ามต่ำกว่า"**: ตอนนี้**ต่ำกว่าใน 10/16 ข้อ** — ทั้งหมดคือ ops floor (TLS, RBAC, metrics, backup, HA, multi-tenancy, compliance) ไม่ใช่ engine — แก้ได้เป็นลำดับ (บางข้อระดับ "วันเดียว")
- **ยุทธศาสตร์**: ทำ**ทั้งสองอย่างแบบเรียงลำดับ** — เสริมจุดแข็ง (ship enterprise floor ให้ moat ขายได้) พร้อมกับปิด 2 blocker เชิงโครงสร้าง (bulk-load path แบบ external-file + on-disk vector index) โดย**ลอกเทคนิคคู่แข่งที่ license เปิด** (RocksDB SST, Kuzu CSR — MIT, DiskANN — MIT, pgvector iterative_scan)

---

## 1. Context

Goal set by the product owner: (a) beat **same-tier** competitors (embedded graph/vector engines) head-to-head by **>=20%** on the axes that matter; (b) be **at parity or better** vs direct + indirect **enterprise** competitors — no axis strictly below; (c) evidence-only, no guessing; (d) copying a competitor's technique is allowed and preferred when it is better/faster than inventing.

### 1.1 The competitive map shifted under us (verified 2026-07-03)

| Event | Evidence | Impact |
|---|---|---|
| **Kuzu is dead**: Apple acqui-hired Kuzu Inc (~2025-10-09, EU merger filing disclosed 2026-02-11); `kuzudb/kuzu` archived read-only 2025-10-10 | betakit.com, 9to5mac.com, theregister.com 2025-10-14 | AUDIT--P26 comparator defunct; its MIT code is now freely minable with no roadmap risk |
| **LadybugDB is the successor** (MIT, community fork): v0.18.0 (2026-07-01), ~monthly releases, HNSW vectors, npm `@ladybugdb/core`, v0.18 added WAL group commits + ART indexes | github.com/LadybugDB/ladybug/releases | Our P30 audit measured v0.15.3 → **3 releases stale**; re-run required |
| **HelixDB** (YC X25, Apache-2.0, Rust) at v3.0.7, 5.5k stars; vendor bench claims 12–16× Neo4j on graph ops; **no vector bench published** | helix-db.com/blog/benchmarks (vendor-published) | The closest **live** same-tier Rust rival — **no direct GenesisBlockDB-vs-HelixDB benchmark exists**. Biggest measurement gap |
| **Agent-memory ecosystem scrambling post-Kuzu**: Cognee migrated off; Graphiti/Zep deprecated Kuzu backend, now recommends server DBs (Neo4j/FalkorDB) | github issues topoteretes/cognee#2098 (closed), getzep/graphiti#1132 (open) | A market window: embedded graph+vector agent memory currently has **no incumbent** |
| **Neo4j threat materialized on schedule**: Cypher 25 `SEARCH` clause (vector + in-index filtering) GA 2026.02; GraphRAG HybridRetriever shipped | neo4j.com docs | The 2026-06 threat table's "6–12 month" item landed; Neo4j is closing the hybrid gap faster than we are closing the ops-floor gap |
| **SurrealDB nightmare scenario did NOT materialize**: SurrealDS (3.0) is quorum-consensus, NOT CRDT; no ed25519 signing, no verifiable audit trail, no bitemporal | surrealdb.com/platform/surrealds + security docs | Our verifiability moat window is still open (threat remains on watch) |
| **No enterprise vendor ships bitemporal or signed/verifiable storage natively** (Qdrant/Weaviate/Milvus/Neo4j/SurrealDB/pgvector/Elastic all checked) | Opus-gated vendor-doc sweep 2026-07-03 | Both uniqueness claims **survive external verification** — for the first time (REPORT §9.3 asserted this without competitor citations) |

### 1.2 Current architecture & data flow (code-verified, all claims Opus-CONFIRMED)

**Write path:** client → NAPI/REST → `add_node` → `validate_governance` (engine-level MASTER lockout, ~524 ns, <0.1% of a write) → id interning (u32, SeqCst) → per-collection arena staging + **async HNSW enqueue** (bounded-4096 channel, one indexing thread per Storage) → `insert_node_lean` (DashMap) → `persist()` = ed25519-sign + WAL group-commit (5 ms / 1024-event batch, fsync, ack) — durable-synchronous, index-eventually-consistent (`flush_index()` = read-your-write).

**Read paths:** `hybrid_search` (per-collection dim-validated HNSW, optional on-disk f32 sidecar rerank, K-Impact opt-in at α=0.0 default) · `neighbors` (bitemporal-aware BFS over sharded adjacency) · `retrieve_context`/GRL (BFS + all-or-nothing SuperNode compression) · `execute_hql` dispatches directly, no planner.

**Production-relevant structural findings** (previously undocumented; each is an action item in §7):

1. **Front-end asymmetry**: the REST server wraps the whole `Storage` in one global `parking_lot::RwLock` and calls blocking storage code **directly on the async executor** (9 write handlers, no `spawn_blocking`) — the NAPI front-end uses bare `Arc<Storage>` + `spawn_blocking`. REST-layer contention has **never been benchmarked** (all P7–P33 harnesses hit the core, not Axum). `src/router.rs:172-315`, `src/lib.rs:5714-5719`.
2. **The standalone REST server never snapshots**: `start_autonomic_loop` (3600 s snapshot+compact) is started only by the NAPI constructor; `src/main.rs` never calls it. A long-running REST deployment only compacts if a client explicitly triggers it.
3. **Single indexing thread** per Storage (deliberate, per ADR--GENESISDB-ASYNC-INDEXING) is a structural ingest ceiling across all collections.
4. **`/v1/query` bypasses bitemporal** (raw edge scan, no `valid_from/valid_to`/retraction filtering) — a correctness trap for consumers who assume engine-wide time-travel semantics.
5. **WAL channel is unbounded** (backpressure only via per-caller ack); index channel is bounded-4096.
6. GRL token-budget compression is binary (drop all atoms → SuperNodes only), not graduated.
7. Docs drift: CLAUDE.md says lib.rs ~4,700 lines → actually **5,979**; FLOW doc calls snapshotting "asynchronous" → it is synchronous.

---

## 2. Head-to-head scorecard — same-tier (>=20% target)

All rows measured on our own harnesses unless noted. Conditions: 100k nodes/800k edges fanout-8 (graph) or 100k×1024-dim (vector), C: SSD, durable WAL on our side. **Competitor-side numbers are pre-MARK-XIII (2026-06-21) and were not re-run against our improved baseline** — a methodological caveat that cuts both ways.

### 2.1 Axes where >=20% superiority is MET (measured)

| Axis | Competitor | Us | Them | Margin | Evidence |
|---|---|---|---|---|---|
| hop1 traversal | Kuzu (archived) | 22.6 µs | ~3,653 µs | **~189× faster** | P26 §3, P31 §3 *(corrected at final gate — the workflow scorecard had inverted this row; P26/P31 unambiguously show a win)* |
| hop1 traversal | LadybugDB 0.15.3 | 22.6 µs | ~3,637 µs | **~177× faster** (no payload caveat — we return full node+path, they return bare ids, and we still win) | P30 §3, P31 §3 |
| hop1 traversal | DuckDB+graph | 21.6 µs | 1,169.8 µs | **~54× faster** (+5,314%) | P28 §3 — *collapses at depth; see 2.2* |
| vector p50 | Qdrant (server) | 974 µs @ recall 0.979 | 3,301 µs @ 0.999 | **~3.4× faster** (+239%) — embedded-vs-server tax + lower recall stated | P20 §3-4 |
| vector p50 | LanceDB | 935.6 µs @ recall 0.948 | 8,392 µs @ 0.998 | **~9× faster** (+797%) — **at lower recall**; condition mandatory | P27 §3 |
| index crash-safety | DuckDB vss | HNSW WAL-covered, durable | persistence experimental-flagged, **WAL recovery not implemented** (documented data-loss risk), RAM-only, f32-only | qualitative win | duckdb.org/docs vss (fetched 2026-07-03) |
| quantization lineup | DuckDB vss / Chroma | SQ8 4.00× / BQ 32.0× (structural, post-P0) / F16, per-collection, WAL-durable | none (both) | capability win | vss docs; Chroma: no native quantization (third-party, 2026-07-03) |
| bitemporal model | all six same-tier | supersede/retract, tested (`tests/bitemporal.rs`, `retract_edge_tests.rs`) | none found in any | **sole capability** | absence verified in same-tier research pass |
| verifiability + governance | all six same-tier | ed25519 SignedEvent + engine-enforced MASTER tier, tested | none found in any | **sole capability** | same pass |

### 2.2 Axes where the target is NOT met (measured losses / ties — the honest list)

| Axis | Competitor | Us vs Them | Gap | Root cause (evidence) |
|---|---|---|---|---|
| bulk edge ingest | Kuzu/LadybugDB (COPY) | 7.8 s vs 0.4–0.6 s | **13–19× slower** (was 60×; our 3.1× self-improvement, competitor not re-run) | They build columnar/CSR **outside** the live write path; we serialize per-op Events through WAL even batched (P31 §3, P26 §3) |
| bulk edge ingest | DuckDB / RocksDB | 7.8 s vs 0.7–1.9 s | 4–11× slower | same mechanism (P28/P29) |
| RSS @100k/800k | Kuzu/LadybugDB | 686 MB vs ~97 MB | **7.1× more RAM** (was 11×) | No node-id interning yet (edge side done, MARK XIII); hnsw_rs keeps an independent 2nd in-RAM copy of every vector (RCA) |
| RSS @100k/800k | RocksDB | 686 MB vs 33 MB | 20.8× | same + payload-rich model (P29/P31) |
| durable vector ingest | Chroma / LanceDB | 1,751–1,982 vs 3,270 / 2,750 vec/s | 1.6–1.7× slower | durability asymmetry (we fsync, Chroma is in-memory) — fair caveat but still a lost benchmark row (P20/P27) |
| hop6 deep traversal | DuckDB | 4,902 µs vs 4,669 µs (P28 baseline) / 5,336 µs (P31 re-run) | tied (−5%…+8%) | per-result `NeighborOutput{node,path}` cloning dominates deep-hop time, not graph-walk cost (P29 §4) |
| hop1 | RocksDB+graph | 22.6 vs 17.4 µs | tied-class (variance flip) | both in 17–27 µs band (P31 §4) |
| recall ceiling | Qdrant | 0.990 (ef=512) vs 0.999 | −0.009 abs on identical corpus | ef-bounded; SQ8+rerank hits 0.9875=f32-parity but on a different (real bge-m3, n=3k) corpus (P20/P21/P33) |
| larger-than-memory vectors | LanceDB (IVF_PQ on disk) | **no capability** vs shipped | structural −100% | hnsw_rs is RAM-only; grep confirms no on-disk index exists (only "NEVER mmap" comments) |
| recall @500k (self) | — | 0.887 @ default ef=200 (0.982 @100k) | scale regression | fixed architecturally (per-query/per-collection ef, P32) but headline claims must carry scale conditions |

### 2.3 Unmeasured (blocking an honest "we beat our tier" statement)

1. **HelixDB v3.0.7** — no direct head-to-head exists at all (their vendor bench uses different hardware/topology).
2. **LadybugDB 0.18.0** — P30 measured 0.15.3; successor added WAL group commits + ART indexes since.
3. **Post-P0 empirical RSS** — the 4.00×/32.0× restoration is a structural proof + green tests; the 500k/1M sweep command exists but was never executed (AUDIT--ONDISK-RERANK-RSS §3).
4. Competitor ingest/RAM ratios post-MARK-XIII; any competitor at 1M; p99 anywhere; concurrent mixed read/write QPS; crash-recovery time; REST-layer throughput.

---

## 3. Enterprise parity — "ห้ามต่ำกว่า" checklist (16 capabilities)

Bar set by best-of (Qdrant / Weaviate / Milvus / Neo4j / SurrealDB / pgvector / Elastic-OpenSearch), each vendor claim URL-verified 2026-07-03.

| Capability | Status | Us → Bar |
|---|---|---|
| Bitemporal data model | **ABOVE** | engine-native + tested → no vendor ships it |
| Signed/verifiable audit trail | **ABOVE** | ed25519 SignedEvent + Merkle → no vendor ships it (incl. SurrealDB 3.0) |
| Single-node durability/crash recovery | **PARITY** | WAL group-commit + 12 h soak (4.72M nodes, bounded disk) → clustered vendors differ on HA, not single-node durability |
| Quantization | **PARITY** | SQ8/BQ/F16 per-collection, WAL-durable → Milvus RaBitQ (32× 1-bit), Elastic BBQ (~95% mem, vendor-bench). Hedge: our 4×/32× empirical re-run pending |
| Filtered ANN (in-index) | **UNKNOWN** | not audited → pgvector 0.8 `iterative_scan`; Neo4j SEARCH clause GA. Must audit our filter path before claiming anything |
| Snapshot/PITR | **UNKNOWN** | full-snapshot only → but **no vendor was confirmed to ship true PITR either** (possible industry-wide gap = opportunity) |
| Backup/restore | **BELOW** | full re-serialize, REST server never auto-snapshots → Qdrant incremental (delta) backups; Neo4j online per-member backup |
| Replication/HA | **BELOW** | none (CRDT sync exists but not a documented/tested HA mode) → Qdrant 3-node/2-replica; Neo4j causal clustering; Milvus CDC |
| AuthN/AuthZ | **BELOW** | single shared-secret bearer (warns "unauthenticated" if unset) → RBAC everywhere, SSO at Qdrant/Neo4j |
| TLS | **BELOW** | none in Axum (reverse-proxy assumption, undocumented) → default TLS across vendors |
| Encryption at rest | **BELOW** | plain JSON/bincode on disk → Qdrant volumes + CMK; OpenSearch KMS free tier |
| Multi-tenancy | **BELOW** | collections = partitioning primitive, not tenants (no quota/RBAC-per-tenant) → Weaviate 1M+ tenants; Milvus 4 levels |
| Observability/metrics | **BELOW** | no /metrics endpoint at all → Qdrant & Weaviate native Prometheus |
| Hybrid dense+sparse | **BELOW** | K-Impact is a graph signal (α=0 default), not lexical; NotiKeeper had to build RRF+BM25 **outside** the engine → Qdrant/Weaviate/Milvus native sparse+dense; Neo4j HybridRetriever |
| Ops tooling (CLI/console) | **BELOW** | /v1/status + dashboard only → neo4j-admin, Attu, Qdrant console |
| Compliance (SOC2/HIPAA) | **BELOW** | none → Qdrant Cloud SOC2 Type 2 + HIPAA |

**Verdict: the enterprise floor is violated on 10/16 axes — every violation is operational (server shell), none is engine-core.** This is fixable without touching the storage engine, and several items are days not months.

---

## 4. Strengths/weaknesses validation — is our self-image correct?

Claims audited against primary audits (20/20 Opus-CONFIRMED):

**Real and stable (the moat):** bitemporal + ed25519-signed governance survived every internal AND external verification pass, across two audit rounds three weeks apart, with zero stale-number risk. These are also the *only* axes simultaneously ahead of the whole same-tier and enterprise fields. Incremental K-Impact (O(V_affected), up to 398,105× vs full recompute) and <0.1% governance overhead are real but currently opt-in/default-off (α=0.0 since 2026-06-29, ADR--KIMPACT-AS-SIGNAL).

**Real but narrower than marketed (fix the copy, keep the claim):**
- "52× vs DuckDB" is hop1-only; same audit says hop6 is "effectively tied" — POSITIONING.md omits it.
- "~9× vs LanceDB" was measured at recall 0.948 vs their 0.998 — condition omitted.
- "7–185× vs Neo4j" is largely embedded-vs-server tax per our own audit; ingest/memory are ~par — framing omitted.
- "recall 0.984 @ ~1.1 ms" is a 100k ef=128 frontier point; the same defaults gave 0.887 at 500k (P32) — scale condition omitted.
- "hop1 ~22 µs across 10k→1M" + competitor ratios conflates our 3-point curve with ratios measured only at 100k.
- "Bulk insert trails ~1.5–2×" (vector-only) hides the 30–60× graph-ingest losses documented two sections earlier in the same report.

**Wrong (retract):**
- **"Multi-vector per node" (REPORT §9.3) is false** — `NodeInput` has exactly one `embedding` field routed to one collection (`src/lib.rs:99-110`). Retract or reword to "multiple collections, one vector per node per collection."
- "Node.js NAPI addon = sole in market" is now stale — LadybugDB ships `@ladybugdb/core`, LanceDB/DuckDB have embedded Node SDKs. The defensible claim is the **combined** embedded bitemporal+signed+graph+vector binary, not Node embeddability alone.
- Quantization "4×/32×" was false for the 9-day window 2026-06-23→07-02 (resident sidecar, RCA) and is now structurally true again (PR #51) — but stays "structural + tests" until the pending empirical sweep runs.

---

## 5. Decision

**Adopt strategy C — strengthen the moat AND close the structural blockers, sequenced in three waves (§7), with an evidence-first rule: no external competitive claim ships without a current measured number and its conditions.**

## 6. Options considered

### Option A — Strengthen strengths only (double down on bitemporal/verifiability marketing + latency)
| Dimension | Assessment |
|---|---|
| Cost / time | Low (docs, positioning, small features) |
| Same-tier 20% goal | **Unreachable on 2 axes forever** (larger-than-memory = no capability; bulk ingest = shared technique we lack) |
| Enterprise floor | Stays violated on 10/16 — disqualified in any enterprise evaluation regardless of moat |
| Risk | Neo4j closes hybrid gap while we stand still; HelixDB benchmarks us first |

### Option B — Close weaknesses only (RAM/ingest/on-disk grind)
| Dimension | Assessment |
|---|---|
| Cost / time | High, multi-quarter |
| Same-tier 20% goal | Reachable, slowly |
| Risk | The moat (verifiability) stays unsellable without the ops floor; window with no embedded incumbent (post-Kuzu) closes unexploited; we chase columnar engines on their home axis while neglecting ours |

### Option C — Both, sequenced (CHOSEN)
| Dimension | Assessment |
|---|---|
| Cost / time | Same total as B, but front-loads days-scale wins (metrics endpoint, evidence hygiene, re-benchmarks) |
| Same-tier 20% goal | Wave 0 makes claims honest; Wave 2 attacks the two blockers with **copied, licensed techniques** (faster than inventing) |
| Enterprise floor | Wave 1 clears the cheap 6 of 10 violations within weeks; the rest get an honest documented posture |
| Rationale | The moat is only monetizable if the ops floor exists; the blockers are only closable via techniques competitors already proved (MIT/Apache) — copying beats inventing on every one of them |

## 7. Refinement plan (gap register → waves)

**Wave 0 — Evidence hygiene & re-measurement (days; everything else depends on it)**
1. Run the pending empirical RSS sweep @500k/1M for the on-disk sidecar (command already in AUDIT--ONDISK-RERANK-RSS §3) — converts 4.00×/32.0× from structural to measured.
2. **New head-to-heads: P34 = HelixDB v3.0.7; P35 = LadybugDB 0.18.0** (same harness as P26/P30), plus re-run competitor ingest/RAM ratios against our post-MARK-XIII baseline.
3. Fix POSITIONING.md's seven overstatements (§4) + retract multi-vector-per-node; add scale/recall/payload conditions inline; update CLAUDE.md line count.
4. Publish the benchmark page (16 measured head-to-heads; REPORT §9.8 flagged this "very high" priority 11 days ago — still unshipped; HelixDB is publishing vendor benchmarks *now*).

**Wave 1 — Enterprise floor (weeks; server shell only, no engine changes)**
5. Prometheus `/metrics` endpoint (Qdrant/Weaviate parity; days of work, biggest below-bar ROI).
6. TLS via rustls opt-in in Axum + documented reverse-proxy posture; RBAC middleware layered over `api_key_guard` (design reference: Qdrant API-key model; implementation original).
7. REST front-end parity fixes: `spawn_blocking` in handlers, replace global `RwLock<Storage>` with `Arc<Storage>` (NAPI pattern), start autonomic snapshot loop (or scheduled compaction) in `src/main.rs`, add `execute_batch` REST route (napi-rest-parity), fix or document `/v1/query` bitemporal bypass.
8. Incremental backup design (WAL-segment shipping is the natural primitive — our WAL already checkpoints through the writer thread).

**Wave 2 — Same-tier blockers, via copy-what-works (weeks–quarter)**
9. **Bulk-load path** — copy the external-file-then-ingest pattern: build sorted/columnar runs outside the live WAL+DashMap path, ingest as one metadata operation. Sources: RocksDB `SstFileWriter`/`IngestExternalFile` (Apache-2.0/GPL-2.0 dual), Kuzu COPY-into-CSR (MIT, archived = zero roadmap risk). Target: within 2–3× of COPY-class engines (from 13–19×), measured by re-run of P26-class harness.
10. **Node arena interning** (mirror MARK XIII edge fix — precedent in-repo) + investigate eliminating hnsw_rs's duplicate in-RAM vector copy (fork or alternative crate). Target: RSS gap 7.1× → ≤3×.
11. **Deep-hop**: lean/bare-id traversal mode (kills the NeighborOutput cloning that dominates hop3/6 per P29) + frontier-batched multi-source BFS (copy DuckPGQ/VLDB-2025 set-based expansion pattern, MIT). Target: hop6 ≥20% faster than DuckDB (we are already +8% on P31's re-measured baseline; lean mode alone likely clears it).
12. **Filtered ANN**: audit `hybrid_search`'s filter behavior; if naive pre/post-filter, port pgvector's `iterative_scan` idea (PostgreSQL license; algorithmic port, no code needed).
13. **Native hybrid dense+sparse**: FTS5-style trigram BM25 + RRF fusion in-engine — generalize NotiKeeper's proven external design (our own consumer already validated it; RRF is public-domain algorithmics). Wire into both front-ends.

**Wave 3 — Structural (quarter+)**
14. **On-disk vector index** (larger-than-memory): Vamana/DiskANN-style flat graph (MIT, Microsoft) — reuse the already-shipped `SidecarReader` positioned-read + bounded-LRU primitive for vectors; Weaviate's own analysis confirms flat graphs suit disk paging better than HNSW hierarchy. Alternative: LanceDB IVF_PQ (Apache-2.0). This closes the only −100% structural axis.
15. **HA story**: promote CRDT/ed25519 sync into a documented, failover-tested replica mode (our differentiator doing double duty) — needs its own ADR + failover test suite.
16. Multi-tenancy (tenant = auth boundary + quota over collections) and compliance program — sequenced last; org-level, not code-level.

**Query-language track (HQL vs Cypher) — strengthen-the-moat work; slots into Waves 2–3 without displacing the blockers** *(added 2026-07-03 after follow-up analysis)*

Verdict: "sole custom QL in this category" is **false** (HelixDB ships custom compiled HelixQL; LadybugDB ships Cypher; GQL is now an ISO standard and Neo4j Cypher 25 gained a vector `SEARCH` clause, GA 2026.02). HQL's defensible surface is not syntax ownership but **semantics Cypher/GQL cannot express** — and those map exactly onto the two moat axes of §2.1:
`AS OF` bitemporal time-travel on every command (`src/query/hql.pest:22`), `SIMILAR TO … ALPHA` single-statement vector+graph blending, and `CONTEXT … TIER … BUDGET` token-budgeted retrieval (an agent-native primitive no engine has). Meanwhile the syntax tax is already shrinking: Cypher-style pattern `MATCH` shipped (PR #60, `hql.pest:47-76`) and WHERE/ORDER BY/LIMIT/RETURN shipped (PRs #43/#44) — HQL is converging on Cypher exactly where LLM familiarity pays.

Decision: **refine HQL (convergent-syntax strategy); do NOT build a full Cypher/GQL engine** (planner + write semantics + openCypher TCK ≈ multi-quarter, would consume Wave 1–2 capacity for a surface that surrenders the moat semantics).

17. **W2.6** — variable-length path patterns `-[*1..3]->` (the missing killer Cypher feature; patterns today are fixed linear hops, `hql.pest:65`), basic aggregation (`COUNT`), and `EXPLAIN` (echo the dispatch path — cheap since there is no planner).
18. **W2.7** — LLM-writes-HQL eval: the real users of this language are agents via MCP/SDKs; measure generation accuracy against the 34-test fuzz grammar, feed few-shot corrections into the MCP tool descriptions. The metric that matters is "% of LLM-generated queries that parse and mean what was intended", not grammar aesthetics.
19. **W3.4 (conditional, demand-driven)** — openCypher **read-subset adapter** (translate a defined MATCH/WHERE/RETURN subset onto the existing Storage dispatch) **only if** the post-Kuzu ecosystem capture (Graphiti/Cognee integration spike) proves Cypher is required at the driver level. Build the Graphiti driver first; if driver-level integration suffices, skip the adapter entirely.

## 8. Consequences

**Easier:** enterprise conversations stop dying at the ops-floor checklist; competitive claims become audit-proof (each carries conditions + a current measurement); the two moat axes get an honest, externally-verified "sole in market" statement; bulk-load and RAM work rides proven designs instead of research.
**Harder:** three benchmark harness re-runs become recurring obligations (LadybugDB monthly cadence will keep going); the REST refactor (lock model) risks subtle behavior changes — needs the REST contract tests; maintaining an on-disk index doubles the vector-index surface.
**Revisit:** single-indexing-thread ADR once bulk-load path lands (the ceiling moves); SurrealDB CRDT watch item (community discussion still open); whether to upstream or fork hnsw_rs.

## 9. Method & correction log (why these numbers can be trusted)

- 6 Sonnet-5 workers (architecture, perf-evidence, claims-validation, same-tier research, enterprise research, copy-worthy techniques) → 8 Opus gates (each claim re-verified by opening the cited file or fetching the URL) → Fable final gate.
- 159 claims: 137 CONFIRMED, 6 REFUTED (dropped/corrected), 19 UNVERIFIABLE (excluded from any load-bearing statement), corrections applied: Milvus RaBitQ "72%"→32×; Elastic BBQ bench re-attributed first-party-2025; Cognee issue actually closed; DuckDB 2.0 "Sept"→"fall 2026"; prrao87 Q1 figure re-attributed; Kuzu "no longer supporting" quote dropped (not in source).
- **Two degenerate agent outputs (placeholder "test" values) were caught by output validation and re-run.**
- **Final-gate correction (this document):** the workflow scorecard inverted the Kuzu/LadybugDB hop1 row (claimed we were 189× *slower*); P26/P30/P31 unambiguously measure us 189×/177× *faster*. Corrected in §2.1. The Opus scorecard gate verified 18 rows' arithmetic but did not spot-check that row — recorded here as a process lesson: **inversion checks belong in the gate prompt**.
- Standing limitations (unchanged by this ADR): competitor-side numbers are 2026-06-21 vintage; no p99/cold-start/crash-recovery/concurrent-QPS measurements exist; HelixDB unmeasured.

## 10. Action items

1. [ ] Wave 0.1 — run empirical RSS sweep @500k/1M (owner: dev-host; ~1 day)
2. [ ] Wave 0.2 — P34 HelixDB + P35 LadybugDB-0.18 head-to-heads + competitor ratio re-runs
3. [ ] Wave 0.3 — POSITIONING.md/CLAUDE.md corrections (7 overstatements + 1 retraction + line count)
4. [ ] Wave 0.4 — publish benchmark page
5. [ ] Wave 1.1 — /metrics endpoint; 1.2 TLS+RBAC; 1.3 REST parity refactor (spawn_blocking, lock model, autonomic loop, execute_batch route, /v1/query fix); 1.4 incremental backup design ADR
6. [ ] Wave 2.1 — bulk-load external-file path (RocksDB/Kuzu pattern); 2.2 node interning + hnsw_rs double-copy; 2.3 lean traversal + frontier BFS; 2.4 filtered-ANN audit→iterative scan; 2.5 native BM25+RRF; 2.6 HQL variable-length paths + COUNT + EXPLAIN; 2.7 LLM-writes-HQL eval
7. [ ] Wave 3.1 — on-disk Vamana index ADR + spike; 3.2 CRDT-based HA mode ADR; 3.3 multi-tenancy/compliance scoping; 3.4 (conditional) openCypher read-subset adapter — decide via Graphiti driver spike first
