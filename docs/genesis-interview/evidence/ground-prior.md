# Grounding Report — Prior Art + Real Performance Numbers (GenesisBlockDB repo)

All paths relative to `G:\GenesisBlock_Dev\GenesisBlock`.

---

## 1. HQL refinement plan — task list and working-tree status

Source: `docs/PLAN--HQL-REFINEMENT.md` (22 tasks, P0→P1→P2 as sequential PRs, P3 = design-first ADR). Verified against the **uncommitted working tree** (`git diff`: src/query/hql.pest +16, src/query/ast.rs +245/−, src/lib.rs +133/−, tests +168).

| ID | What it fixes | Working-tree status |
|---|---|---|
| P0-T0 | Design gate: target semantics, colon-id policy, strict-number policy | Implicitly resolved by the implemented code (search-by-node chosen; strict errors chosen); no separate decision doc found |
| P0-T1 | SEARCH/hybrid discarded `_resolved` target → search-by-node (stored embedding when `SIMILAR TO` omitted; error if unresolvable/no embedding) | **DONE** — `similar_clause?` optional in `hql.pest`; `hql_query_vector()` + `resolved_target_id()` helpers in `src/lib.rs` (~3356-3400 in diff); `_resolved` deleted from both Search and Hybrid arms; uses `reconstruct_embedding`. Tests: `hql_search_without_literal_vector_uses_target_embedding`, `hql_search_without_vector_errors_for_missing_embedding` in `tests/hql.rs` |
| P0-T2 | Hybrid hardcoded `k: 10` → optional `K <n>` clause (default 10) | **DONE** — `(^"K" ~ k)?` on `hybrid` rule; `let mut k = 10` default in `parse_hybrid` (ast.rs); executor passes `k` |
| P0-T3 | `EF <n>` / `OVERSAMPLE <n>` unreachable (P32 recall fix + VQ rerank knob forced `None`) | **DONE** — `ef_spec`/`oversample_spec` atomic rules on both search + hybrid; flow into `HybridSearchInput.ef_search/.oversample` |
| P0-T4 | TRAVERSE hardcoded `direction:"out"`, single rel → `DIRECTION in\|out\|both` + `REL a\|b` union | **DONE** — `direction_spec`, `rel_type = rel_name ~ ("\|" ~ rel_name)*` in grammar; `rels`/`direction` on `Traverse` AST; executor maps to `NeighborInput` (`direction.or_else(\|\| Some("out"))`). Test: `hql_traverse_direction_and_multi_rel_are_exposed` |
| P0-T5 | Silent numeric defaults (bad K→5, DEPTH→1, ALPHA→0.5) → parse errors | **DONE** — `parse_u32`/`parse_f64` return `"HQL Parse Error: {field} value out of range"`; LIMIT saturate deliberately kept (`unwrap_or(usize::MAX)` at ast.rs:399, 881). Test: `strict_numeric_parse_errors_surface_in_ast` (K/DEPTH/ALPHA/BUDGET) |
| P0-T6 | Per-edge `Utc::now().to_rfc3339()` in `match_pattern` + `neighbors` hot loops → once per query | **DONE** — `let now = Utc::now().to_rfc3339();` hoisted in both (`src/lib.rs` diff at match_pattern ~3049 and neighbors ~3725) |
| P0-T7 | `{id:"…"}`-anchored pattern was O(N) full node scan → O(1) interned-id seed | **DONE** — anchor_id fast path via `get_u32` + `nodes.get`, falls back to scan otherwise |
| P0-T8 | Tests: new grammar + fuzz-corpus extension | **PARTIAL** — new positive/negative tests exist in `tests/hql.rs` + `tests/hql_filter_tests.rs`; but `tests/hql_fuzz_tests.rs` changed only 1 line (vector type compile fix, `Some(vec![…])`) — **fuzz mutation generators NOT extended with the new keywords** as the plan's L2 DoD requires |
| P0-T9 | Docs de-stale (ADR statuses, CLAUDE.md 5 forms, `user:5` example, index.d.ts/MCP docstrings) | **NOT DONE** — CLAUDE.md, ADRs, index.d.ts unmodified in git status |
| Colon-id (P0-T0 fork 2 / spec §2.6) | `user:5` unquoted qualified_id | **NOT DONE** — no `qualified_id` in the working-tree grammar |
| P1-T0..T6 | Var-length paths `-[:R*min..max]->`, frontier cap, lazy bindings, identity join, rel alternation in patterns, bench rows | **PENDING** (nothing in tree) |
| P2-T1..T4 | OR+parens WHERE, `RETURN count(*)`, label index, CONTEXT clauses | **PENDING**; execution strategy **superseded 2026-07-03** by ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE phase S2 (WHERE/OR + count compile to SQL over the SQLite projection; bespoke `label_idx` cancelled in favor of an indexed `node_labels` table; `score`/`depth` predicates stay native) |
| P3-T0 | `SEARCH TEXT` design ADR | **PENDING**; design fork already resolved toward option (b): SQLite FTS5 trigram BM25 + RRF (substrate S3), one lexical engine shared with Wave 2.5 |

Docs-side working tree also adds a **track boundary (2026-07-05)** note to both PLAN and SPEC-V2: P0 stays native/independently shippable; SQLite may reshape only P2/P3.

Baseline warning from `docs/REPORT--HQL-V1-VS-V2.md` §4.2: **the v1 `hql-query-stress` + `graph-bench` baseline must be captured on pre-P0 `main` before P0 merges** — since P0 is already implemented uncommitted, this baseline capture is still possible (changes not merged) but is a live process risk.

## 2. SPEC--HQL-V2 target surface (key committed productions)

Source: `docs/SPEC--HQL-V2.md` (status: target). Compatibility contract: every valid v1 query parses identically; the **only breaking change** is strict numeric parse errors; no planner, no EXPLAIN, no on-disk format change.

- §2.1 SEARCH/hybrid (search-by-node, design-gated):
  ```
  SEARCH [~]<target> [SIMILAR TO [ <vector> ]] K <k> [EF <n>] [OVERSAMPLE <n>]
         [IN <collection>] [LANGUAGE "…"] [AS OF "…"] [<clauses>]
  MATCH  [~]<target> [SIMILAR TO [ <vector> ]] ALPHA <a> [K <k>] [EF <n>] [OVERSAMPLE <n>] …
  ```
  Vector present → byte-identical v1; omitted → node's stored embedding; unresolvable → error `"HQL: target '<t>' does not resolve to a node and no vector was given"` (never silent-empty).
- §2.4 TRAVERSE: `TRAVERSE FROM [~]<seed> DEPTH <d> REL <rel>[|<rel>…] | INFER(<rel>) [DIRECTION in|out|both] [AS OF "…"] [<clauses>]`
- §3.1 Var-length: `-[r:REL*<min>..<max>]->` e.g. `-[:LINK*1..6]->` — binds terminal node only, no path variable; direction/rel filter at every step; `*1..1` MUST be row-identical to single hop; explicitly makes the P26/P30 competitor-bench query expressible: `MATCH (a {id:"g0"})-[:LINK*1..6]->(b) RETURN b.id LIMIT 1000`.
- §3.2 Frontier cap: per-round **row** cap (recommended 100k), **hard error** with remedy text, not silent truncation.
- §3.3 Repeated variable = identity join (real cycles); §3.4 edge-only alternation `-[:R1|R2]->`.
- §4.1 WHERE: `expr := term (OR term)* ; term := factor (AND factor)* ; factor := pred | "(" expr ")"` — AND binds tighter; null ⇒ leaf false.
- §4.2 `RETURN count(*)` → `[{"count": n}]` post-WHERE; combining with ORDER BY/fields = parse error.
- §5 `TEXT` is **reserved**; semantics only via ADR (must reconcile Wave 2.5; no-capability text query = named error, never silent empty).
- §7 Non-goals on record: no planner/EXPLAIN, no branching patterns/OPTIONAL MATCH/path vars/shortest-path, no aggregation beyond count(*), no prop-value indexes, **no in-engine embedding model (mobile weight budget)**.

Directly relevant to the mission's proposed `HYBRID … TRAVERSE … AS OF … RANK BY rrf(…)` construct: v2 as specced has **no RANK BY / fusion clause and no cross-command pipeline** — SEARCH/TRAVERSE/MATCH stay separate commands; RRF fusion exists only as the S3/Wave-2.5 lexical-hybrid plan. The proposed construct is new surface beyond SPEC--HQL-V2.

## 3. SQLite substrate plan (S0–S3)

Sources: `docs/SPEC--SQLITE-SUBSTRATE-S0-S1.md` (normative, 2026-07-05) + phase pointers in `docs/PLAN--HQL-REFINEMENT.md` P2/P3 banners. (The parent ADR is at `docs/adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md`, committed per memory 526b7ac.)

**Authority model:** signed WAL remains the **only** durability authority; SQLite is a derived, engine-owned projection — deletable/corruptible and rebuildable from authoritative state; no caller writes SQLite directly.

**What stays native (unchanged):** mutation intent/signatures/clocks/replay order; graph adjacency + bitemporal edge traversal; vector collections/HNSW/quantization/rerank sidecar.
**What moves to SQLite:** node `props` payload persistence (S1: SQLite becomes the runtime source for props reads); `node_labels` normalized rows (schema in S0, execution use later); `projection_state` replay watermark.

Stages:
- **S0 (foundation, C-3, HIGH risk):** embed `rusqlite`; schema v1 (`props`, `node_labels`, `projection_state`); idempotent replay + gap healing + full rebuild; snapshot coherence; 4 named crash windows proven (crash after WAL commit pre-SQLite-apply; mid-SQLite txn; SQLite missing; SQLite behind watermark). No runtime reads from SQLite yet; public surface frozen.
- **S1 (props migration, C-3):** props off the resident node map; traversal must not need resident props. **Gates:** RSS reduction re-measured on the P31 harness AND no material traversal regression (no hardcoded thresholds — compared to repo baselines).
- **S2 (per PLAN P2 banner):** HQL WHERE/OR and `count(*)` compile to SQL over the projection (pushed-down indexed filtering); label anchor via indexed `node_labels`; `score`/`depth` predicates stay in a small native evaluator. Language contract unchanged; execution strategy changes.
- **S3 (per PLAN P3 banner):** in-engine lexical search on SQLite **FTS5 trigram BM25 + RRF** — one lexical engine serving both HQL `SEARCH TEXT` and Wave 2.5 native dense+sparse hybrid (NotiKeeper-validated pattern).

**Meaning for query execution:** hop path and vector path stay in-memory native (the µs-class hop1 numbers below are what the substrate must not regress); predicate filtering/aggregation/lexical scoring become SQL-backed. HQL P0 is contractually independent of all of it.

## 4. Real measured performance numbers

**Credibility per `docs/benchmarks/INDEPENDENT-BENCHMARKS.md`: everything below is Level 0 (maintainer-run internal audit) except the `benchmark/` suite which is Level 1 (reproducible commands + schema-verified result.json + verify_report.py trust gate). Zero Level-2 external reproductions exist. No number in the repo is third-party-independent.** Within Level 0, the engine emits only observable metrics (latency/recall/disk); env/RAM are captured externally by `assemble_result.py` — but the operator is still the maintainer. Hardware throughout: Windows 10 Pro, i7-8700K, 32 GB RAM, C: SATA SSD (bench dir via `GB_VBENCH`), warm/in-process unless noted.

### Vector search (own engine)

| Metric | Value | Conditions | Source |
|---|---|---|---|
| Query p50 / p95 | 974 / 1,472 µs (ef=200); 896 / 1,414 µs (ef=100) | 100k synthetic clustered, dim 1024, L2, k=10, embedded warm | AUDIT--P20 |
| Recall@10 | 0.979 (ef=200), 0.956 (ef=100) | same | AUDIT--P20 |
| ef frontier @100k | ef16: 560 µs/0.859 · ef64: 812 µs/0.964 · ef128: 1097 µs/0.984 · ef256: 1255 µs/0.988 · ef512: 2119 µs/0.990 | 100k, dim 1024, efc=200 | AUDIT--P21 |
| ef frontier @500k | ef50: 920 µs/0.7895 · **ef200: 1458 µs/0.887** · ef400: 1892 µs/0.9405 · **ef800: 4528 µs/0.973** (p95 7065) | 500k synthetic, dim 1024, k=10, q=200, no quant | AUDIT--P32 — the scale-recall collapse that motivates the per-query EF knob HQL P0-T3 now exposes |
| Insert throughput | 1,751–1,982 vec/s durable @100k (P20); 254 vec/s per-op-fsync @3k (P15); 2,236/s @500k, 1,897/s @1M f32 (P33) | durable WAL | P20/P15/P33 |
| Quant latency (p50, 500k/1M) | f32 1104/1146 µs · SQ8 983/1188 · **BQ 416/482** · BQ+rerank 511/598 | synthetic dim 1024, default ef | AUDIT--P33 |
| RSS @1M dim-1024 | f32 11,408 MB · SQ8 5,534 (2.06×) · BQ 3,898 (2.93×) · +rerank sidecar adds ~1.9 GB@500k (resident-era numbers) | ~1.8 GB@500k / ~3.6 GB@1M non-vector floor untouched — "graph is the RAM frontier" | AUDIT--P33 |
| Recall on REAL embeddings (bge-m3, n=3000, k=10, ef=200) | f32 **0.9875** · SQ8 0.9485 · **SQ8+rerank 0.9875 (=f32)** · **BQ alone 0.6845 (unusable)** · **BQ+rerank 0.9655 @ 317 µs (~5× faster than f32's 1671 µs)** · BQ centering: no lift (0.6810), no regression w/ rerank (0.9640) | real repo-docs corpus, exact-L2 GT | AUDIT--P33 §3.4 + P1a addendum |
| Off-RAM rerank sidecar | resident/vec restored: SQ8+rerank 1.33×→**4.00×**, BQ+rerank 1.88×→**32.0×**; LRU cap ≈24 MiB@1536-dim O(1) in N; recall byte-identical | structural proof; empirical 500k RSS sweep **pending** | AUDIT--ONDISK-RERANK-RSS |

### Graph traversal / ingest (own engine, current baseline = P31)

| Metric | Value | Conditions | Source |
|---|---|---|---|
| hop1 / hop3 / hop6 p50 | **22.6 µs / 2,529 µs / 4,902 µs** | 100k nodes, fanout-8 (800k edges), 200 q/depth, LIMIT 1000, returns full node+path payload, `graph-bench` | AUDIT--P31 (post-MARK XIII; ±5–11% run variance; hop6 single runs untrustworthy per §4) |
| hop1 throughput | 42,327/s | same | AUDIT--P31 |
| RSS @100k/800k | **686 MB** (was 1,057 pre-interning, −35%) | same | AUDIT--P31 |
| Edge ingest 800k | **7.8 s** durable WAL (was 24.4 s, 3.1×) | same | AUDIT--P31 |
| HQL-layer query | avg **10.49 µs**/query | `hql-query-stress` 1k×1536, 100 iters (small corpus — not comparable to the 100k graph numbers) | AUDIT--P14 |
| ldbc_lite (criterion) | 1-hop 8.37 / 2-hop 78.28 / 3-hop 527.85 µs median | 1k nodes, fanout 5 | AUDIT--P14 |
| Concurrent load | 118.34 TPS, p95 432 µs, peak RSS 213 MB | shadow-sync-stress 10k×1536, 12w/4r, **G: HDD** | AUDIT--P14 |
| 12h soak | 4.72M nodes, ingest flat 360–420 ms/500-node cycle, query sub-ms, disk bounded 4.1 GB (WAL compaction working), peak RAM ~15–17 GB | dim=4 toy vectors (recall ~0.6 expected at dim 4 — footprint test, not recall test) | AUDIT--SOAK-TEST-12H (Level 1 harness exists in `benchmark/`) |

## 5. Prior competitive benchmarking and outcomes

All maintainer-run (Level 0), same-machine, methodology + reproduce commands in each audit; harnesses in `benches/*.py` + `graph-bench`/`vbench-genesis`.

| Competitor | Doc | Setup fairness | Outcome (headline) |
|---|---|---|---|
| **Chroma** (hnswlib, embedded) | P15 (3k real bge-m3), P20/P21 (100k synth) | like-for-like embedded; Chroma non-durable | @100k GB p50 896–974 µs vs 990; p95 lower; recall 0.979 vs 0.981 (parity); frontier passes through Chroma's point (P21). Insert: Chroma ~1.7× faster (non-durable) |
| **Qdrant** (Docker server, gRPC) | P20 | GB embedded vs Qdrant server — **latency includes localhost gRPC round-trip** | Qdrant p50 3,301 µs / recall 0.999; GB 974 µs / 0.979. GB "wins" p50 by ~3.4× but this is the embedded-vs-server tax, explicitly flagged. **No embedded-fair Qdrant comparison exists, and no filtered-search (vector+filter) comparison at all — G1 has no prior evidence base in-repo** |
| **LanceDB** 0.33 (embedded, IVF_HNSW_FLAT) | P27 | like-for-like embedded; warm, matched ef=100 | GB p50 935.6 µs vs 8,392 µs (~9×); recall 0.948 vs 0.998 (ef choice, see P21 frontier) |
| **Neo4j** (Docker server) | P23 | embedded vs server (bolt+Cypher-plan+JVM tax stated as "the whole point") | hop1 **120–185×** faster, hop3 8.5–10.5×, hop6 7.2–7.6×; ingest ~par @100k; memory ~par @100k |
| **Kuzu** 0.11.3 (embedded, columnar) | P26, re-run P31 | like-for-like embedded, prepared Cypher `*1..d` | GB hop1 189× (P31), hop3 6.8×, hop6 23.3× (hop6 ratio inflated by *their* run variance — audit forbids claiming it); **Kuzu wins ingest ~60× (COPY, 0.4–0.6 s) and RSS ~7.1× (97 vs 686 MB)** |
| **LadybugDB** 0.15.3 (live Kuzu fork — the "most on-niche" comparator) | P30, re-run P31 | like-for-like; LadybugDB returns bare `b.gid`, GB returns node+path — asymmetry favors them, GB still wins | hop1 177× (4,002 µs vs 22.6), hop3 7.9×, hop6 23.6× (variance caveat); LadybugDB wins ingest ~48× (0.5 s) and RSS ~7× (96 MB) |
| **DuckDB+graph** | P28/P31 | embedded SQL recursive-CTE graph | GB hop1 52× (1,170 µs); hop3 1.41×, hop6 1.09× — **near-parity at deep hops** |
| **RocksDB+adjacency** | P29/P31 | closest architecture; hop3/6 return bare ids (**flagged not apples-to-apples**) | hop1 effectively tied (GB 21.6–22.6 vs RDB 17.4–26.8 µs across runs — same class, variance-flipped); RDB wins ingest ~7× and RSS ~20× (33 MB). Validates GB's µs class; audit stresses RocksDB+graph "is not a product; it is a project" |
| **HelixDB** v3.0.7 | PLAN--P34 (scope only, **not run**) | would be P23-shaped: server/Docker-only, closed engine image; vendor's own bench (v2.1.0, ARM, concurrent-load) ruled unusable quantitatively | no numbers; would be the first independent HelixDB v3 measurement |

**Cross-cutting honest findings the mission should inherit:**
- GB's decisive wins are **point/k-hop latency** (hop1 µs-class) and **hybrid capability density**; the standing losses are **bulk ingest (durable WAL vs COPY, ~48–60×)** and **RSS vs columnar stores (~7×, non-vector floor ~3.6 KB/node)** — the S1 props migration targets exactly this.
- **No cross-dimension (G3-shape) benchmark has ever been run** — nothing measures a single query spanning vector+hop+AS-OF+fusion vs app-composition; deep-hop numbers also carry a payload-materialization confound (P29) that P1-T3 lazy bindings targets.
- Per REPORT §4.4, after P1 the P26/P30-shape query becomes expressible in HQL itself, enabling the first apples-to-apples HQL-vs-Cypher head-to-head instead of benching `neighbors()` as a proxy.
- Bench discipline is codified: ≥2 runs, p50/p95, exact commands + commit in PR body, ±10–11% hop-class envelope, C: SSD, `--features bins` (REPORT §4; P31 §4 variance lesson: Kuzu hop6 doubled between runs on identical code).
