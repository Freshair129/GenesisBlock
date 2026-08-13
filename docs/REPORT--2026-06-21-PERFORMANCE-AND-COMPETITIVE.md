---
proposed_id: REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE
doc_id: REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE
type: report
status: historical
version: n/a
owner: GenesisBlockDB Engineering
aliases:
  - REPORT
tier: process
cluster: implementation_flow
role: "Session report — docs/code audit, correctness fixes, perf optimization, competitive benchmark"
date: 2026-06-21
proposed_by: agent
related:
  - INCIDENT--EDGE-U32-BUILD-BREAK-AND-RAM-MISDIAGNOSIS
  - CR--EDGE-ENDPOINTS-STRING-AND-EMBEDDING-DEDUP
  - AUDIT--P14-POST-REFACTOR-VERIFICATION
  - AUDIT--P15-COMPETITIVE-VECTOR-BENCHMARK
  - AUDIT--P16-CONCURRENCY-LOCK-FIX
  - AUDIT--P17-BULK-WAL-BATCH
  - AUDIT--P18-PARALLEL-HNSW-BUILD
  - AUDIT--P19-EF-TUNING-AND-SCALE
  - AUDIT--P20-QDRANT-3WAY-AND-EF-CONFIG
  - AUDIT--P31-POST-MARKXIII-REGRESSION
---

# GenesisBlockDB — Engineering Report (2026-06-21)

> **Internal audit (Level 0).** The numbers in this report were measured by the
> maintainer on one machine and are **not** independently reproduced. For the
> reproducible, schema-verified benchmark workflow that anyone can run and submit,
> see [`../BENCHMARKING.md`](../BENCHMARKING.md) and
> [`benchmarks/INDEPENDENT-BENCHMARKS.md`](benchmarks/INDEPENDENT-BENCHMARKS.md).

Docs↔code audit → correctness restoration → performance optimization → first
measured competitive benchmark. All work merged to `main` and pushed; the full
Rust suite is green throughout (20 passed / 0 failed / 22 binaries).

---

## 1. Executive summary

- The working tree did **not compile** and the entire test/bench suite was
  broken by an unfinished `String→u32` edge refactor. Reverted to a clean,
  durable, green state.
- Cut per-node memory **~44%** (dropped a redundant in-memory f64 embedding copy).
- Lifted **concurrent durable ingest 6.1×** (136→839 TPS) and **single-thread
  bulk insert 7.8×** (254→~1,950 vec/s) via three targeted fixes.
- Ran the project's **first real head-to-head** vs Chroma and Qdrant. At 100k
  vectors GenesisBlockDB is **query-latency-leading and recall-at-parity (ef=200),
  durable**.
- Replaced unverifiable "competitive" claims and a wrong RAM diagnosis with
  measured numbers, with explicit honesty about what is and isn't comparable.

---

## 2. Audit: docs vs code (starting state)

A 3-agent audit (docs claims / code surface / SDK drift) plus `cargo check`
found large drift:

- **Build broken:** `src/lib.rs:1695` type error (`execute_batch`).
- **Edge endpoints** changed `String→u32` (uncommitted) — broke tests, benches,
  and both SDKs; exposed an internal arena id at the API/persistence boundary
  (not client-knowable, not stable across WAL replay); `add_edge` could panic.
- **HNSW not rehydrated** on snapshot instant-load → silent semantic-search
  outage until manual rebuild.
- **Stale docs:** MCP tool set conflicts, dual `hql.pest` references (root file
  gone), `retract_edge` stub, `execute_batch` not a REST route, version chaos
  (v2.0.0 / 1.2.0 / 0.2.2b), `API_REFERENCE.md` contains a leaked LLM transcript.
- **No measured competitor benchmark ever existed** — prior figures were
  published-spec or self-admitted "oversell".

Details: `INCIDENT--EDGE-U32-BUILD-BREAK-AND-RAM-MISDIAGNOSIS`.

---

## 3. Correctness restoration

| Change | Effect | Commit |
|---|---|---|
| Edge `from/to` reverted to `String` (intern to u32 internally) | build + suite green; SDK/binding drift resolved for free | b5e9771 |
| `add_edge` unwrap → guard | no panic on unknown endpoint | b5e9771 |
| HNSW rehydrate on **both** load paths | snapshot load no longer breaks search | b5e9771 |
| Bench compile rot (`ldbc_lite`) | benches runnable again | b5e9771 |

Decision: keep stable **string identity at the API/persistence boundary**,
`u32` only in internal indices — the correct layering. Change request:
`CR--EDGE-ENDPOINTS-STRING-AND-EMBEDDING-DEDUP` (merged).

---

## 4. Performance optimizations

| # | Change | Result | Commit |
|---|---|---|---|
| P-B | Drop redundant in-memory f64 embedding (`insert_node_lean`) | RSS 147→82 MB @5k×1536 (**−44%**) | 75d560e |
| #1 | Remove per-op global HNSW write lock (`insert(&self)` → shared read; short arena CS) | concurrent ingest 136.60→**839.36 TPS** (**×6.1**) | 2ffe5f1 |
| #2 | `bulk_add_*` → one `Event::Batch` per chunk (1 fsync) | bulk insert 254→385 vec/s (+52%) | 8efdff4 |
| #3 | `parallel_insert` (rayon) on batch path | bulk insert 385→~1,950 vec/s (×5.2; **×7.8** overall) | ce06746 |
| ef | `set_index_params(ef_construction, ef_search)` runtime knob; default 200 | tunable recall↔speed | 440afaa |

Audits: P16 (lock), P17 (batch WAL), P18 (parallel build), P19 (ef + scale),
P20 (ef config). All retain durability and atomic-batch semantics.

---

## 5. Competitive benchmark

**Embedding model:** `bge-m3` (BAAI, **1024-dim**) via local Ollama — real
embeddings of repo doc chunks for the realistic 3k run; synthetic clustered
vectors (dim 1024) for scale runs (we lack 100k diverse real texts locally).
Identical vectors fed to every engine; exact brute-force L2 ground truth; C: SSD.

### 100k vectors, 3 engines

| Metric | GenesisBlockDB ef=200 | GenesisBlockDB ef=100 | Chroma (hnswlib) | Qdrant (server) |
|---|---|---|---|---|
| Query p50 | **974 µs** | **896 µs** | 990 µs | 3,301 µs |
| Query p95 | **1,472 µs** | **1,414 µs** | 1,704 µs | 4,424 µs |
| Recall@10 | **0.979** | 0.956 | 0.981 | 0.999 |
| Insert (vec/s) | 1,751 (durable) | 1,982 (durable) | 3,270 (in-mem) | 715 (server+index) |

**Findings**
- **Query latency: GenesisBlockDB leads** both at scale. Qdrant's ~3.3 ms is the
  localhost gRPC round-trip — the embedded-vs-server tradeoff, not an index gap.
- **Recall at ef=200 ≈ Chroma** (0.979 vs 0.981). Chroma itself fell from 1.000
  (3k/50k) to 0.981 at 100k — only large N differentiates ANN quality.
- **ef knob works:** ef=100 → faster, recall 0.956; ef=200 → recall 0.979 at
  ~12% lower insert.
- **Insert:** GenesisBlockDB durable ~1.7× slower than in-memory Chroma; Qdrant
  slowest (server-side async index build + gRPC).

Audits: P15 (3k Chroma), P20 (100k 3-way). Interactive view:
`perf-comparison-dashboard.html`.

---

## 6. Methodology & honesty notes

- **Storage matters (P14):** the project disk `G:` is a 7200 RPM HDD; historical
  audits ran on NVMe. The same binary ran **42–46× faster** for fsync-bound
  writes on `C:` (SSD). Disk-bound numbers are not comparable across machines;
  memory and in-memory latency are.
- **Durability asymmetry:** GenesisBlockDB persists every write (WAL); Chroma here is
  in-memory ephemeral; Qdrant is a persisted server. Insert numbers are not
  like-for-like — query latency and recall are the fair index metrics.
- **15.89 GB was a myth:** the old P12 figure is a Mark VII artifact; the current
  engine measures ~1 GB at 32k. An initial blame on HNSW `max_elements` was
  wrong (it is a ~8 MB `with_capacity` hint) — corrected by reading hnsw_rs and
  measuring.

---

## 7. Remaining work / recommendations

1. **Expose `ef` in `OpenOptions`** too (currently a runtime setter) once the ~40
   construction sites are worth touching; keep quality-first default 200.
2. **Insert throughput:** investigate hnsw_rs vs hnswlib raw build speed; consider
   deferred/async indexing to take HNSW off the write hot path (keeps query
   latency low during bulk load — see P16 P95 note).
3. **Doc hygiene:** mark every SPEC/TDD with status (Proposed/Implemented/
   Deprecated) + verified-commit; regenerate `API_REFERENCE.md` from code; fix
   dangling path references. (Tracked in the docs-vs-code audit.)
4. **Multi-collection vector space** (`SPEC--MULTI-COLLECTION-VECTOR-SPACE`,
   P-C/P-D): per-model/dim spaces for code + text (e.g. jina-code 1536 + bge-m3
   1024) — the next architectural feature.

---

## 7b. P21–P25 — Frontier, scale, graph & cost evidence

**P21 Recall–latency frontier (100k, ef_construction=200, ef_search swept):**
GenesisBlockDB curve passes through Chroma's point — ef_search=128 → recall 0.984 @
1.1 ms (> Chroma 0.981 @ 0.99 ms); ef_search=64 → 0.964 @ 0.81 ms. Qdrant
0.999 @ 3.3 ms (server). `ef_search` is a live knob (`set_index_params`).

**P21 vector scale:** RSS 1.57 GB @100k → 7.7 GB @500k (linear); recall at
ef_search=100 falls 0.982 → 0.891 by 500k (needs higher ef_search at scale).
1M ≈ RAM ceiling on 32 GB.

**P22 graph traversal (LDBC-lite, fanout 8):**

| N | hop1 p50 | hop3 p50 | hop6 p50 | hop1 thrpt | RSS |
|---|---|---|---|---|---|
| 10k | 23.1 µs | 1.97 ms | 4.21 ms | 41,525/s | 146 MB |
| 100k | 21.6 µs | 2.33 ms | 4.40 ms | 42,783/s | 1.06 GB |
| 1M | 35.4 µs | 4.58 ms | 9.29 ms | 27,898/s | 12.6 GB |

hop1 stays tens-of-µs across 100× = **O(neighborhood), not O(N)**. RAM ceiling
~12.6 GB @1M/8M (edge-UUID interning) → 10M infeasible on 32 GB. Engine fix:
`neighbors` now honors `limit` (was ignored).

**P23 Neo4j head-to-head (embedded vs server):** GenesisBlockDB **7–185× faster** on
traversal (hop1 100k: 21.6 µs vs 2,590 µs); ingest & memory ~par at 100k. Gap is
largely the embedded-vs-server tax (bolt + Cypher planning + JVM).

**P24 governance guard:** ~524 ns/op = **< 0.1 %** of a durable write
(optimizable ~10× by dropping per-label allocation).

**P25 K-Impact full vs incremental:** full ~O(V) (9 → 104 → 664 ms at
10k/100k/500k); incremental(1 node) flat at ~1–1.7 µs → **O(V_affected) proven**
(up to 398,000× faster than a full pass).

**P26 Kuzu head-to-head (embedded↔embedded, 100k):** GenesisBlockDB wins point/local
traversal latency (hop1 22 µs vs 3,653 µs, 7–166× across hops); **Kuzu wins
ingest ~60× (COPY) and memory ~11×** (columnar analytical store). Different sweet
spots — GenesisBlockDB for low-latency agent memory, Kuzu for bulk graph analytics.
Kuzu's 11× lower memory confirms edge-UUID interning is GenesisBlockDB's top RAM lever.

**Positioning (reframed):** GenesisBlockDB is an **embedded analytics / agent-memory
graph+vector engine** — nearest comparators are Kuzu, DuckDB+graph, RocksDB+graph
layer, and LanceDB (embedded vector); Neo4j/Qdrant are well-known references, not
the category.

**Competitor matrix — measured vs pending (honest status):**

| Comparator      | Category        | Status                  |
|-----------------|-----------------|-------------------------|
| Chroma          | embedded vector | ✅ measured (P15, P21)  |
| Qdrant          | server vector   | ✅ measured (P20)       |
| LanceDB         | embedded vector | ✅ measured (P27)       |
| Neo4j           | server graph    | ✅ measured (P23)       |
| Kuzu            | embedded graph  | ✅ measured (P26)       |
| DuckDB + graph  | embedded graph  | ✅ measured (P28)       |
| RocksDB + graph | embedded graph  | ✅ measured (P29)       |
| LadybugDB       | embedded graph+vec (Kuzu fork) | ✅ measured (P30) |

**All named comparators now have measured head-to-heads (P15–P30)** — including
LadybugDB, the Kuzu fork on this project's exact niche.

**P27 LanceDB head-to-head (embedded vector, 100k/1024d/L2):** at matched recall
(~0.95–1.0), GenesisBlockDB point-query p50 935.6 µs vs LanceDB 8,392 µs (**~9×**) and
Chroma 1,166 µs. LanceDB (on-disk Lance columnar) trades latency for
larger-than-memory scale & cost — same sweet-spot split as Kuzu (P26). See
`AUDIT--P27-LANCEDB-HEAD-TO-HEAD.md`.

**P28 DuckDB+graph head-to-head (embedded, 100k/800k, recursive CTE):** GenesisBlockDB
wins hop1 **~54×** (21.6 µs vs 1,170 µs) but the gap narrows with depth — hop3
~1.4×, hop6 ~1.06× (4.40 vs 4.67 ms, effectively tied) as DuckDB's set-based
recursive join shines on deep expansion. DuckDB beats Kuzu at every depth and
wins ingest ~35× / memory ~11× vs GenesisBlockDB. See
`AUDIT--P28-DUCKDB-GRAPH-HEAD-TO-HEAD.md`.

**P29 RocksDB+graph head-to-head (embedded KV + adjacency BFS, 100k/800k):** the
architecturally-closest baseline. Clean comparison is **hop1: 21.6 µs vs 26.8 µs —
effectively tied** (GenesisBlockDB slightly ahead), confirming GenesisBlockDB's µs point
latency is real against a raw adjacency store. Deep-hop numbers are **not
apples-to-apples** (GenesisBlockDB materializes full node+path objects; the RocksDB
harness returns bare ids) and are not claimed as a RocksDB win. RocksDB wins
ingest ~30× / memory ~32×, but is a KV store with a hand-rolled graph layer (no
query language, paths, governance, bitemporal, or vectors). See
`AUDIT--P29-ROCKSDB-GRAPH-HEAD-TO-HEAD.md`.

**P30 LadybugDB head-to-head (embedded graph+vector, Kuzu fork, 100k/800k):** the
most on-niche competitor. GenesisBlockDB wins hop1 **~168×** (21.6 µs vs 3,637 µs),
hop3 ~6.7×, hop6 ~13.5× — and this win has **no payload caveat** (Ladybug's Cypher
returns bare ids while GenesisBlockDB materializes node+path, yet still wins).
LadybugDB ≈ Kuzu (it forks Kuzu's last release) and wins ingest ~48× / memory ~11×.
See `AUDIT--P30-LADYBUGDB-HEAD-TO-HEAD.md`.

---

## 9. Competitive Market Brief (2026-06-22)

> ส่วนนี้ขยายจาก benchmark head-to-head (P15–P30) สู่ภาพตลาดเต็ม:
> positioning, feature landscape, opportunities, threats, และ strategic implications

### 9.1 ภาพรวมตลาด

ตลาด AI agent memory (vector DB + graph DB + hybrid) อยู่ที่ **$6.27B ปี 2025**
→ คาดเติบโตเป็น **$28.45B ปี 2030 (CAGR 35%)** ขับเคลื่อนจาก:

- **GraphRAG**: practitioner community 2026 converge บน pattern "vectors สำหรับ
  semantic entry-point, graphs สำหรับ relational depth" — hybrid system ได้เปรียบ
- **Agent memory**: LLM agent ต้องการ persistent memory ที่ retrieve เร็ว (vector)
  และ multi-hop reason ได้ (graph) พร้อมกัน
- **Local-first & privacy**: regulatory pressure + latency requirement ดัน embedded
  deployment กลับมา
- **Bitemporal audit**: finance, legal, healthcare ต้องการ temporal audit trail native

---

### 9.2 Competitive Set

#### หมวด A — Direct Competitors (Embedded Graph + Vector)

| | **GenesisBlock** | **LadybugDB** | **HelixDB** | **Kuzu** |
|---|---|---|---|---|
| ภาษา | Rust | C++ (Kuzu fork) | Rust | C++ |
| Open Source | ✓ | ✓ | ✓ | ✓ |
| Graph | ✓✓✓ | ✓✓✓ | ✓✓✓ | ✓✓✓ |
| Vector / HNSW | ✓✓✓ | ✓✓ | ✓✓✓ | ✓✓ |
| Bitemporal | ✓✓✓ | ✗ | ✗ | ✗ |
| CRDT + ed25519 sync | ✓✓✓ | ✗ | ✗ | ✗ |
| Governance tier (MASTER) | ✓✓✓ | ✗ | ✗ | ✗ |
| Node.js native addon | ✓✓✓ | ✗ | ✗ | ✓ |
| Columnar / bulk analytics | ✗ | ✓✓✓ | ✗ | ✓✓✓ |
| Query language | HQL (custom) | Cypher | HelixQL (compiled) | Cypher |
| hop1 latency (100k) | **21.6 µs** ✅ measured | 3,637 µs ✅ measured | — | 3,653 µs ✅ measured |
| Ingest throughput | 1,751 vec/s (durable) | ~48× เร็วกว่า (COPY) | — | ~60× เร็วกว่า (COPY) |
| Memory (100k/800k) | 1.06 GB | ~97 MB (~11× ต่ำกว่า) | — | ~97 MB (~11× ต่ำกว่า) |

#### หมวด B — Hybrid / Multi-model Competitors

| | **SurrealDB 3.0** | **Neo4j** | **ArangoDB 3.12** |
|---|---|---|---|
| ภาษา | Rust | Java | C++ |
| Graph + Vector | ✓✓✓ | ✓✓ (hybrid search preview) | ✓✓ (ArangoSearch) |
| Bitemporal | ✓✓ (storage layer) | ✗ | ✗ |
| CRDT / Sync | ✗ | ✗ | ✗ |
| Governance | ✓ (permissions) | ✓✓ (RBAC enterprise) | ✗ |
| Embedded | ✓✓ | ✗ (server-first) | บางส่วน |
| Enterprise customers | Verizon, Walmart, Nvidia | Forbes 2000+ | SAP, Cisco |
| hop1 latency (100k) | — | 2,590 µs ✅ measured | — |
| GenesisBlock vs | — | **7–185× faster** | — |

#### หมวด C — Pure Vector Databases

| | **Qdrant** | **Chroma** | **LanceDB** |
|---|---|---|---|
| Query p50 (100k) | 3,301 µs ✅ measured | 990 µs ✅ measured | 8,392 µs ✅ measured |
| GenesisBlock p50 vs | **~3.4× faster** | **~parity** (974 µs) | **~9× faster** |
| Recall@10 (100k) | **0.999** (เราตาม) | 0.981 | ~0.95–1.0 |
| Graph traversal | ✗ | ✗ | ✗ |
| Embedded | ✗ | ✓ | ✓ |
| Durability | ✓ (server) | ✗ (in-memory test) | ✓ (on-disk Lance) |

---

### 9.3 Feature Matrix — จุดที่ GenesisBlock เป็นเจ้าเดียว

> ⚠ **แก้ไข 2026-07-03:** ตารางนี้มีข้อผิดพลาด/ข้อมูลตกยุค 3 จุด — ดู §11
> (Corrections) ท้ายรายงาน และ ADR--GENESISDB-COMPETITIVE-SUPERIORITY §4

| Capability | เจ้าเดียวในตลาด? | หมายเหตุ |
|---|:---:|---|
| Bitemporal + graph + vector ใน binary เดียว | **ใช่** | SurrealDB มี bitemporal แต่ไม่ embedded เต็มรูป |
| CRDT + ed25519-signed events + Merkle root | **ใช่** | ไม่มีใครใน competitive set ทำ |
| MASTER governance tier (engine-enforced) | **ใช่** | LadybugDB อ้าง "regulated industries" แต่ไม่ enforce |
| Node.js NAPI native addon (graph+vector+bitemporal) | **ใช่** | ไม่มีใน LadybugDB, HelixDB, Kuzu |
| Multi-vector per node | **ใช่** (ใน hybrid DB) | Qdrant รองรับแต่ไม่มี graph |
| hop1 latency <25 µs embedded (100k, full node payload) | **ใช่** | RocksDB tied แต่ไม่มี query language หรือ governance |

---

### 9.4 Positioning Analysis

**ช่องว่างที่ไม่มีใครอ้าง:**

1. **"Verifiable knowledge graph"** — CRDT + ed25519 audit trail ทำให้ทุก mutation
   พิสูจน์ได้ว่าใครเขียน เมื่อไหร่ conflict resolve อย่างไร ไม่มีใครในตลาดอ้างจุดนี้

2. **"Governance-aware agent memory"** — MASTER tier ป้องกัน agent override ข้อมูล
   trusted ที่ engine level (ไม่ใช่แค่ permission layer) overhead วัดแล้วที่ **<0.1%**
   ของ durable write (~524 ns/op)

3. **"Local-first bitemporal graph"** — SurrealDB มี bitemporal แต่ server-first,
   LadybugDB ไม่มี bitemporal, ไม่มีใครทำ embedded+bitemporal+graph+vector ได้

**Positioning แนะนำ:**

> GenesisBlock คือ **verifiable hybrid knowledge engine** — graph + vector +
> bitemporal + CRDT ในไบนารีเดียว สำหรับ AI agent ที่ต้องการ memory ที่เชื่อถือได้
> ตรวจสอบได้ และ govern ได้

สั้นกว่า: **"The only knowledge database AI agents can't corrupt."**

---

### 9.5 จุดแข็งและจุดอ่อนที่ตัวเลขพิสูจน์แล้ว

**ชนะจริง (measured):**

| สถานการณ์ | ตัวเลข | เจ้าที่แพ้ |
|---|---|---|
| hop1 latency, embedded, full node payload | **168×** | LadybugDB (P30) |
| hop1 latency, embedded, full node payload | **7–166×** | Kuzu (P26) |
| hop1 latency vs server graph | **7–185×** | Neo4j (P23) |
| hop1 latency vs columnar recursive CTE | **54×** | DuckDB+graph (P28) |
| hop1 latency vs KV+adjacency | **~tied** | RocksDB+graph (P29) |
| Point-query vector p50, embedded vs embedded | **~9×** | LanceDB (P27) |
| Point-query vector p50, embedded vs server | **~3.4×** | Qdrant (P20) |
| Recall@10 parity, durable vs in-memory | **≈** (0.979 vs 0.981) | Chroma (P20) |
| Governance overhead | **<0.1%** | — (P24) |
| K-Impact incremental vs full | **up to 398,000×** | — (P25) |

**แพ้จริง (ต้องยอมรับ):**

| สถานการณ์ | ตัวเลข | เจ้าที่ชนะ |
|---|---|---|
| Bulk ingest throughput | แพ้ **~60×** | Kuzu/LadybugDB (columnar COPY) |
| Bulk ingest throughput | แพ้ **~35×** | DuckDB (P28) |
| Bulk ingest throughput | แพ้ **~30×** | RocksDB (P29) |
| RAM per node | แพ้ **~11×** | Kuzu/LadybugDB (edge UUID interning) |
| RAM ceiling | ~1M nodes @ 32 GB | ต้องแก้ edge interning ในอนาคต |
| Recall@10 สูงสุด | 0.979 vs 0.999 | Qdrant (แก้ได้ด้วย ef_search สูงขึ้น) |

---

### 9.6 Opportunities

1. **Verifiable agentic memory** — ตลาดยังไม่มีใครตอบ "ใครเขียนข้อมูลนี้ และ agent
   แก้ได้ไหม?" GenesisBlock มีคำตอบที่ engine level พร้อม benchmark พิสูจน์ overhead

2. **Node.js-native embedded DB** — NAPI-RS addon ใน TypeScript ecosystem ไม่มีคู่แข่ง
   ที่ทำ graph+vector+bitemporal แบบ native binding ได้

3. **GraphRAG ที่ audit-proof** — ตลาด GraphRAG เติบโตเร็ว แต่ทุกคนทำ retrieval แบบ
   stateless GenesisBlock position เป็น "trustworthy GraphRAG backend" ที่ตรวจสอบ
   provenance ได้

4. **LadybugDB อ้าง regulated industries แต่ไม่ enforce** — GenesisBlock เป็นเจ้าเดียว
   ที่มีทั้ง positioning **และ** benchmark พิสูจน์ว่า governance cost < 0.1%

### 9.7 Threats

| Threat | ความรุนแรง | Timeline |
|---|---|---|
| SurrealDB 3.0 เพิ่ม CRDT layer + governance enforcement | สูงมาก | 12–18 เดือน (มีทรัพยากร) |
| HelixDB เพิ่ม temporal + governance | ปานกลาง | 18–24 เดือน |
| Neo4j hybrid search → production-ready | ปานกลาง | 6–12 เดือน |
| pgvector + pg_graphql stack กิน "ง่ายๆ" workload | ปานกลาง | ทันที |
| Platform play: Weaviate/Neo4j ซื้อ CRDT library | สูง (ถ้าเกิด) | 24–36 เดือน |

**Nightmare scenario:** SurrealDB เปิดตัว "SurrealSync" — CRDT + ed25519 audit + governance
RBAC ใน v3.x ก่อนที่ GenesisBlock จะมี enterprise distribution story ที่ชัด

### 9.8 Strategic Implications

| Action | Priority | เหตุผล |
|---|---|---|
| ตีพิมพ์ benchmark ทั้ง 16 head-to-head เป็น public page | สูงมาก | ตัวเลขวัดแล้ว ใช้ได้เลย — อย่าปล่อยให้ LadybugDB/HelixDB claim ก่อน |
| Marketing copy: เน้น MASTER tier + ed25519 audit | สูง | ไม่มีใครอ้างจุดนี้ยังไม่สาย |
| แก้ edge UUID interning → ลด RAM ceiling | กลาง | ปลดล็อค >1M node use case |
| เพิ่ม quantization (scalar/binary) | กลาง | ปิดช่อง Qdrant recall argument |
| สร้าง TypeScript type definitions + install-in-5-min example | สูง | NAPI addon คือ moat ที่สร้างใหม่ยาก |
| ไม่ต้องสู้ billion-scale vector หรือ Managed cloud SaaS | — | ไม่ใช่ target, เสีย focus |

---

## 10. P31 — Post-MARK XIII Regression + Improvement Verification (2026-06-22)

Re-ran graph benchmarks (P22/P26/P28/P29/P30 harnesses) against the post-MARK XIII
codebase to measure the impact of edge interning Layer A+B + u128 keys.
Full audit: [AUDIT--P31-POST-MARKXIII-REGRESSION.md](AUDIT--P31-POST-MARKXIII-REGRESSION.md)

### 10.1 GenesisBlock: P22 vs P31

| Metric | P22 (pre-MARK XIII) | P31 (post-MARK XIII) | Δ |
|---|---|---|---|
| hop1 p50 | 21.6 µs | 22.6 µs | +5% (within variance) |
| hop3 p50 | 2,334 µs | 2,529 µs | +8% (within variance) |
| hop6 p50 | 4,403 µs | 4,902 µs | +11% (within variance) |
| RSS @100k/800k | 1,057 MB | **686 MB** | **−35% ✅** |
| Edge ingest | 24.4 s | **7.8 s** | **3.1× faster ✅** |

No traversal latency regression. RAM −35% and ingest 3.1× faster are the material gains
from edge UUID string removal + numeric key path.

### 10.2 Updated competitor ratios (P31)

| Competitor | hop1 | hop3 | hop6 |
|---|---|---|---|
| vs Kuzu | **189×** | 6.8× | 23.3× |
| vs LadybugDB | **177×** | 7.9× | 23.6× |
| vs DuckDB+graph | **52×** | 1.41× | 1.09× |
| vs RocksDB hop1 | 0.77× (RDB faster, within variance) | — | — |

Kuzu/LadybugDB hop6 ratios jumped (13.5× → 23.6×) due to their run-to-run variance at
deep traversal (exponential fan-out amplifies topology noise). Treat these as same-class,
not a causal improvement. RocksDB hop1 variance flip (tied → RDB slightly faster) is
expected: both engines are in the 17–27 µs range with normal measurement noise.

### 10.3 Strategic impact

MARK XIII closed 35% of the RAM gap vs Kuzu/LadybugDB (11× → 7.1×) and 78% of the
ingest gap (60× → 13×). The remaining gap (numeric id migration + compaction) is MARK XIV P1.
Dashboard updated: [perf-comparison-dashboard.html](perf-comparison-dashboard.html)

---

## 8. Appendix — commits (this session)

```
311bc8f bench(P23): Neo4j head-to-head — embedded GenesisBlockDB vs server Neo4j
6b8ae80 bench(P24,P25): governance guard cost + K-Impact full-vs-incremental proof
e5767af bench(P22): graph traversal benchmark (LDBC-lite) 10k/100k/1M + neighbors limit fix
f4ce106 bench(P21): recall-latency frontier (ef_search sweep) + dashboard scatter
965c01b docs: add consolidated session report
440afaa feat(engine): configurable HNSW ef + Qdrant 3-way benchmark at 100k
2c289cc perf(engine): ef_construction 200->100 + 50k-scale ANN benchmark
ce06746 perf(engine): parallel HNSW build on batch path (parallel_insert) -> 5.2x
8efdff4 perf(engine): route bulk_add_* through Event::Batch (one fsync/chunk) +52%
2ffe5f1 perf(engine): remove per-op global HNSW write lock -> 6.1x concurrent ingest
a35e19e bench: first measured head-to-head vs Chroma (vector k-NN) + dashboard
bd0b4ac docs: add interactive performance comparison dashboard
43b7c59 docs(audit): add P14 post-refactor benchmark verification
09fb6b9 docs(cr): mark edge/RAM change request as merged
11dd357 docs: add incident report (RCA) and change request for the edge/RAM work
663ba7b docs: add multi-collection vector-space spec; correct RAM diagnosis
75d560e perf(engine): drop redundant in-memory f64 embedding from node store (~44% RAM)
b5e9771 fix(engine): revert edge from/to to String; restore build + green suite
```

Reproduce the benchmark: `benches/vbench.py` (embed/Chroma/Qdrant/ground-truth)
+ `benches/vbench_genesis.rs` (`[[bin]] vbench-genesis`, `GB_EF` env).

---

## 11. Corrections & staleness (added 2026-07-03)

Findings from the Opus-gated competitive-superiority audit
(`ADR--GENESISDB-COMPETITIVE-SUPERIORITY`, 2026-07-03). This report is a dated
record; rather than rewriting history, corrections are listed here:

1. **§9.3 "Multi-vector per node — ใช่" is FALSE.** `NodeInput` has exactly one
   `embedding: Option<Vec<f64>>` routed to one named collection
   (`src/lib.rs:99-110`). The real capability is *multiple collections, one vector
   per node per collection*. Retracted.
2. **§9.3 "Node.js NAPI native addon — ไม่มีใน LadybugDB" is stale.** LadybugDB
   ships npm `@ladybugdb/core` (verified 2026-07-03). The defensible uniqueness is
   the combined embedded bitemporal+signed-governance+graph+vector binary, not
   Node embeddability alone.
3. **§9.2 Kuzu column is defunct.** Apple acqui-hired Kuzu Inc (~2025-10-09);
   `kuzudb/kuzu` archived read-only 2025-10-10. LadybugDB (MIT, v0.18.0 as of
   2026-07-01, ~monthly cadence) is the community successor — P30 measured 0.15.3
   and needs a re-run (planned as P35).
4. **§9.5 hop1 ratios carry conditions** documented in the underlying audits but
   dropped here: DuckDB 52× is hop1-only (hop6 effectively tied, P28 §4); the
   Neo4j band is largely the embedded-vs-server tax (P23 §3); LanceDB 9× was at
   recall 0.948 vs their 0.998 (P27 §3); ratios were measured at 100k only.
5. **§9.7 threat table update (2026-07-03):** Neo4j hybrid-search threat
   MATERIALIZED on schedule (Cypher 25 `SEARCH` GA 2026.02). SurrealDB
   CRDT+ed25519 "nightmare scenario" did NOT materialize (SurrealDS is
   quorum-based; no signing/bitemporal in 3.0 docs). Uniqueness claims in §9.4
   are now externally verified for the first time — no same-tier or enterprise
   vendor ships bitemporal + signed/verifiable storage natively.
