---
proposed_id: REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE
type: report
status: complete
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
---

# GenesisDB — Engineering Report (2026-06-21)

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
  vectors GenesisDB is **query-latency-leading and recall-at-parity (ef=200),
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

| Metric | GenesisDB ef=200 | GenesisDB ef=100 | Chroma (hnswlib) | Qdrant (server) |
|---|---|---|---|---|
| Query p50 | **974 µs** | **896 µs** | 990 µs | 3,301 µs |
| Query p95 | **1,472 µs** | **1,414 µs** | 1,704 µs | 4,424 µs |
| Recall@10 | **0.979** | 0.956 | 0.981 | 0.999 |
| Insert (vec/s) | 1,751 (durable) | 1,982 (durable) | 3,270 (in-mem) | 715 (server+index) |

**Findings**
- **Query latency: GenesisDB leads** both at scale. Qdrant's ~3.3 ms is the
  localhost gRPC round-trip — the embedded-vs-server tradeoff, not an index gap.
- **Recall at ef=200 ≈ Chroma** (0.979 vs 0.981). Chroma itself fell from 1.000
  (3k/50k) to 0.981 at 100k — only large N differentiates ANN quality.
- **ef knob works:** ef=100 → faster, recall 0.956; ef=200 → recall 0.979 at
  ~12% lower insert.
- **Insert:** GenesisDB durable ~1.7× slower than in-memory Chroma; Qdrant
  slowest (server-side async index build + gRPC).

Audits: P15 (3k Chroma), P20 (100k 3-way). Interactive view:
`perf-comparison-dashboard.html`.

---

## 6. Methodology & honesty notes

- **Storage matters (P14):** the project disk `G:` is a 7200 RPM HDD; historical
  audits ran on NVMe. The same binary ran **42–46× faster** for fsync-bound
  writes on `C:` (SSD). Disk-bound numbers are not comparable across machines;
  memory and in-memory latency are.
- **Durability asymmetry:** GenesisDB persists every write (WAL); Chroma here is
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
GenesisDB curve passes through Chroma's point — ef_search=128 → recall 0.984 @
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

**P23 Neo4j head-to-head (embedded vs server):** GenesisDB **7–185× faster** on
traversal (hop1 100k: 21.6 µs vs 2,590 µs); ingest & memory ~par at 100k. Gap is
largely the embedded-vs-server tax (bolt + Cypher planning + JVM).

**P24 governance guard:** ~524 ns/op = **< 0.1 %** of a durable write
(optimizable ~10× by dropping per-label allocation).

**P25 K-Impact full vs incremental:** full ~O(V) (9 → 104 → 664 ms at
10k/100k/500k); incremental(1 node) flat at ~1–1.7 µs → **O(V_affected) proven**
(up to 398,000× faster than a full pass).

**Positioning (reframed):** GenesisDB is an **embedded analytics / agent-memory
graph+vector engine** — nearest comparators are Kuzu, DuckDB+graph, RocksDB+graph
layer; Neo4j/Qdrant are well-known references, not the category. Next fairest
datapoint: a Kuzu (embedded↔embedded) head-to-head.

---

## 8. Appendix — commits (this session)

```
311bc8f bench(P23): Neo4j head-to-head — embedded GenesisDB vs server Neo4j
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
