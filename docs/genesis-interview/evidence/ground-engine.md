# Engine Execution Paths — src/lib.rs (working tree, 6046 lines)

All line refs are `G:\GenesisBlock_Dev\GenesisBlock\src\lib.rs` unless noted. Working tree includes the uncommitted HQL-refinement changes (per-query `ef_search`/`oversample` are plumbed HQL→`HybridSearchInput`, SEARCH target now resolves to a query vector, K comes from the query).

## 1. hybrid_search / HybridSearchInput

**Input shape** — `HybridSearchInput` (255–276): `query_vector: Vec<f64>`, `k: u32`, `alpha: Option<f64>`, `lang: Option<String>`, `as_of: Option<String>`, `collection: Option<String>`, `ef_search: Option<u32>` (per-query HNSW override), `oversample: Option<u32>` (per-query rerank over-fetch multiplier).

**Pipeline** — `hybrid_search` (3516–3673), strictly sequential, single-threaded per call:
1. **Collection routing** (3517): `resolve_collection(&args.collection)` (1734–1742), `None` → `default_collection` ("default", always exists, created at open with `Metric::L2, Quant::None` — 2162–2180). **Hard dim validation** (3520–3527): query dim ≠ collection dim is an error, not a silent mismatch.
2. **Lang centroid shift** (3529–3537): if `args.lang` matches a `lang_centroids` entry, the centroid is *added* element-wise to the query (a language-bias hack, not a filter).
3. **Cosine normalization** (3539–3546): query unit-normalized iff `coll.metric == Metric::Cosine` (Cosine is implemented as L2-over-normalized-vectors; single `DistL2` index type — 492–494).
4. **ef resolution** (3549–3553): per-query `args.ef_search` → per-collection `coll.ef_search` (immutable, set at create) → engine-global `Storage.ef_search: AtomicUsize`, **default 100** (2215); `ef_construction` default **200** (2214). Settable via `set_index_params` (1717–1721), global across all collections.
5. **Fetch sizing** (3558–3567): `overfetch = args.oversample.unwrap_or(RERANK_OVERFETCH)` where **`RERANK_OVERFETCH = 8`** (661). `k2 = k*2` (hardcoded ×2, 3562). If the collection has an f32 rerank sidecar: `fetch = max(k*overfetch, k2)`; else `fetch = k*2`.
6. **Exact-brute-force escape hatch** (3576–3581): if a sidecar exists and `fetch >= len_rows`, skip HNSW entirely and enumerate all slots `(0..n)` with distance 0.0 — rerank then scores everything exactly (determinism fix for quantized ties on small collections).
7. **HNSW search** (3587–3595): `VecIndex::search_f32(query, fetch, ef, center, sq8)` (994–1031) under `coll.hnsw.read()`. Returns `(arena_id, distance_f32)`. BQ Hamming distance is normalized by dim to [0,1] (1016–1022) so `1 - distance` stays a similarity.
8. **f32-sidecar rerank** (3603–3618): for each candidate, replace quantized distance with **exact L2** (`exact_l2`, 668–677 — plain scalar iterator, no SIMD) against the on-disk sidecar row (`SidecarReader::row`, positioned read + LRU). Missing row keeps quantized distance (degrade, never drop). Re-sort ascending, **truncate to k*2** (3617).
9. **Metadata join + bitemporal post-filter** (3626–3656): map `arena_id → NodeMetadata.node_u32 → nodes[u32]` (DashMap). **`as_of` is a POST-filter** — `is_valid_as_of(valid_from, valid_to, as_of)` (3502–3514, plain RFC3339 *string comparison*: valid iff `valid_from <= as_of < valid_to`) drops candidates *after* HNSW, with **no top-up refetch** — a heavily-superseded corpus can return < k.
10. **Fusion/score** (3624, 3641–3643): `alpha = args.alpha.unwrap_or(0.0)`; `score = similarity*(1-alpha) + node.impact.unwrap_or(0.0)*alpha` where `similarity = 1.0 - distance`. That is the ENTIRE fusion: a single hardcoded linear blend of vector similarity with K-Impact. **No RRF, no lexical/trigram leg** — despite the name, `hybrid_search` is vector-only (+optional impact blend). The trigram index is used solely for fuzzy-id resolution (see §5).
11. **Sort desc (NaN-safe), dedupe by node id (keep best — one node can hold multiple arena slots after `add_vector` supersede), truncate to k** (3659–3671).

**No attribute/property filter exists anywhere in this path** — neither pre- nor post-filter. WHERE filtering happens only in HQL post-processing (`apply_hql_clauses`, 2927–2989) *after* hybrid_search returns k rows, so `SEARCH ... WHERE` filters the already-truncated top-k (the classic filtered-ANN recall bug; relevant to G1/Qdrant parity — Qdrant does filter-aware HNSW).

**Index-lag interaction**: none in-query. HNSW inserts are async (§6); an unflushed vector is invisible to search. `flush_index()` (1899–1904) sends `IndexJob::Flush` and blocks on ack; `index_lag()` (1907–1909) reads `index_pending: AtomicUsize`.

**HQL dispatch** (execute_hql 3354–3500): `Search` maps to `hybrid_search` with `alpha: Some(0.0)` (3416); `Hybrid` passes the query's alpha (3476). Query vector: explicit `vector` wins; else target id (fuzzy-resolved if `~`) → `reconstruct_embedding` from the arena (3370–3397, 5424–5441). `get_ranked_context` (3675–3679) is `hybrid_search` with hardcoded `alpha = 0.4`.

## 2. neighbors / traversal

**Structures** (1556–1559): `nodes: DashMap<u32, NodeOutput>`, `edges: DashMap<u128, EdgeOutput>` keyed by `edge_key(id)` = first 16 bytes of SHA256(id) (1700–1707), `out_idx / in_idx: DashMap<u32, HashSet<u128>>` (node-u32 → edge-key set). No per-rel bucketing — the adjacency set mixes all relation types.

**`neighbors(seed, NeighborInput, is_inferred)`** (3681–3820) — iterative **BFS** with `VecDeque` + `visited: HashSet<u32>`:
- `NeighborInput` (229–237): `depth, rel, rels, direction, as_of, include_invalid, limit`.
- Depth: `args.depth.unwrap_or(1)` (3691). Nodes visited once globally (`visited`, 3727–3728, 3783) — it's a BFS *tree*, not all paths.
- **Rel filter** (3694–3710): `rels` (non-empty list) overrides `rel`; `None`/`"ANY"` = no filter. Applied per-edge *inside* the loop (3768) after collecting the full adjacency set — filtering is a scan, not an index lookup (**fan-out cost is O(total edges at node) regardless of rel**).
- **Direction** (3713–3719): "out" (default) / "in" / "both"; candidate eids unioned from out_idx/in_idx and deduped in a per-node `HashSet<u128>` (3737–3747).
- **Bitemporal** (3753–3767): per-edge `is_valid_as_of` (as_of projection); additionally with **no** as_of, an edge whose `valid_to <= now` (retracted) is hidden unless `include_invalid = true` (3761–3767, `now` snapshotted once as RFC3339 at 3725). Per-*node* `is_valid_as_of` too (3789–3795).
- Far endpoint by u32 identity (3777–3781): whichever of from/to does NOT intern to curr; **undirected endpoint pick** even for `dir=out` (works because eid came from out_idx). Path accumulated by **cloning the full `Vec<EdgeOutput>` per expansion** (3797–3798) — O(depth²) clones per result row.
- **Fan-out limits**: only `args.limit` — a *global* early-return once `results.len() >= limit` (3805–3809). No per-node fan-out cap, no frontier cap.
- `is_inferred` (TRAVERSE `INFERRED::rel`): ignores the depth bound (3732, 3810) — unbounded transitive closure over the visited set.
- **No variable-length `[*1..n]` support at storage level** — depth is the only knob (P1 of the HQL plan).

**`match_pattern`** (3037–3227, Cypher-subset MATCH): anchor = **full `nodes` DashMap scan** (3078–3091) unless the anchor pattern has `{id:"..."}` (then direct intern lookup, 3062–3076). Each hop expands the frontier via out_idx/in_idx with the same as_of + retraction checks (3120–3130), exact `rel_type` equality (3131–3135), `node_matches` label/prop equality (2994–3024). Rows are JSON `Map<String, Value>` with **full node/edge JSON serialized into every binding row** (3157–3163). No cycle prevention across hops (Cartesian expansion), no limit push-down.

**`query`** (3822–3839, `/v1/query`): raw full scan of `edges`, from/to equality only, **ignores bitemporal entirely**.

## 3. retrieve_context (GRL)

`retrieve_context(target_id, tier_str, budget, fuzzy)` (4257–4364):
- Tier logic: `ScalingTier::parse` (182–204) — `"H0".."H5"` → **hops = 0..5**, unknown string defaults H1. That is the entire tier logic: tier == BFS radius.
- BFS over out_idx AND in_idx (4292–4328) — **NO bitemporal check, NO rel filter, NO retraction filter, edges pushed unconditionally including duplicates** (an edge inside the neighborhood is pushed once per endpoint visit).
- Budget (4331–4352): `token_estimate = sum(props.to_string().len()) / 4`. If `estimate > budget`: **discard ALL nodes and edges** and return every `meta_nodes` SuperNode in the DB (not just relevant clusters) — plus a `println!` to stdout (4342). This is the "HGMem cluster retrieval" that the candidate work would re-key on vector similarity; today it is tier-blind, similarity-blind, and all-or-nothing.
- Output `ContextPackage` (208–214): `nodes, edges, super_nodes, token_estimate, reasoning_path` (a format string, 4359).
- SuperNode (467–475): `cluster_id, theme` (literally `"Theme-{id}"`, 3978), `member_count, impact, centroid: Vec<f64>, timestamp, drift`. Built by `update_meta_graph` (3930–4014) from `NodeMetadata.cluster_id` (**initialized to arena_id at stage — 1453 — i.e. singleton clusters until `detect_communities` (3841+) runs label-propagation over the default collection**). MetaEdge (479–483): cluster-pair weight counts.

## 4. Bitemporal model

- `NodeOutput` (121–138): `valid_from: String`, `valid_to: Option<String>`, `clock: LogicalClock{time: u32, peer_id}`, plus `caused_by`, `expires_at` (TTL), `collection`. **No `recorded_at`/tx-time on nodes and no `superseded_by` on nodes** — lineage is `caused_by`.
- `EdgeOutput` (156–169): `valid_from, valid_to: Option<String>, recorded_at: String, superseded_by: Option<String>, caused_by, impact, clock` — edges are the genuinely bitemporal entity (valid-time + tx-time). `superseded_by` is on the struct but nothing in lib.rs sets it on the write paths shown (add_edge sets `None`, 2765).
- `supersede_node` (2777–2812): closes old version (`valid_to = now`), persists it, clones to a new version (`valid_from = now, valid_to = None`, new clock, optional new props/caused_by), **overwrites the same `nodes[u32]` slot** — the old version survives ONLY in the WAL, so `as_of` time-travel on *nodes* can only see the current in-memory version's window; historical node versions are not queryable from RAM.
- `retract_edge` (5583–5596): bitemporal soft delete — sets `valid_to = at.unwrap_or(now)`, advances clock (CRDT LWW), edge stays in `edges` map; visible via `as_of` before retraction or `include_invalid`.
- **as_of projection** = `is_valid_as_of` (3502–3514): lexicographic RFC3339 string compare, `valid_from <= as_of < valid_to`. Applied as a post-filter in hybrid_search (3633), per-edge/per-node in neighbors (3754, 3789) and match_pattern (3066, 3120, 3151).
- **No interval/BETWEEN/OVERLAPS support anywhere at storage level** — single-instant projection only. No temporal index; every temporal check is a per-row string compare during scan/traverse.

## 5. trigram_index (lexical)

- Structure (1575–1578): `trigram_index: DashMap<String, RoaringBitmap>` — token → bitmap of node u32s. **Nodes only; edges skip it. Only consumer is `find_fuzzy_id`** — it is NOT a lexical search leg for hybrid_search.
- Tokenizer `tokenize_id` (1647–1672): despite the name, tokens = every single lowercase char + (if the id contains combining marks, e.g. Thai vowels/tone marks) the mark-stripped chars + all lowercase **char bigrams** (windows(2)). Mark-stripping via `unicode_general_category` (Nonspacing/Spacing/Enclosing marks) is the Thai handling.
- `find_fuzzy_id` (2336–2381): exact intern hit → done; else union candidate bitmaps for every query token, score each candidate with **`strsim::jaro_winkler`** against the raw id, take the max. Thresholds: `> 0.85` → accept; else `> 0.20` → *also accept* (the "relaxed for Thai" fallback — effectively any non-trivial similarity wins; the "Neural Fuzzy vector fallback" comment at 2374 is dead — no vector search happens). Indexed at intern time in `get_or_intern_id` (1683–1688); rebuilt on snapshot load (4888–4890). **No scoring/BM25, no posting frequencies, no lexical result list** — output is a single best id.

## 6. Vector collections / HNSW / quantization

- `VectorCollection` (1242–1293): `name, model, dim: u16, metric, quant, arena: RwLock<ArenaStore>, metadata: RwLock<Vec<NodeMetadata>>, hnsw: RwLock<Option<VecIndex>>, node_to_arena: DashMap<u32,u32>, count, ef_search: Option<u32>, f32_sidecar: Option<RwLock<SidecarReader>>, bq_center: RwLock<Option<Vec<f32>>>, sq8_calibrate: bool, sq8_scale: RwLock<Option<(f32,f32)>>`.
- `ArenaStore` (699+): `F32(Vec<f32>) | U8 (SQ8, affine scale carried in-variant) | Binary | F16`. `NodeMetadata` (418–432): `arena_id, node_u32, timestamp, vector_dim, embedding_offset, gks_attributes: Vec<u8>, lang, cluster_id`.
- **HNSW params** (`VecIndex::build`, 907–915): `Hnsw::new(16, cap, 16, ef_c, dist)` — **M = 16, max_layer = 16 hardcoded**; `ef_construction` default **200**, `ef_search` default **100** (2214–2215); `HNSW_MIN_CAP = 1024` (1388). F16 has no native distance so its HNSW indexes f32 (897–903). BQ uses custom `DistBinaryHamming` popcount (683–693) because anndists' `DistHamming<u64>` counts word inequality.
- **Distance is scalar, not SIMD** in the engine's own code: `exact_l2` (668–677) and `DistBinaryHamming` are plain loops. `hnsw_rs = "0.3.4"` in Cargo.toml:66 with **no feature flags** — anndists' `simdeez` SIMD feature is NOT enabled; DistL2 runs the scalar path.
- **Async indexing**: bounded `crossbeam` channel **cap 4096** (2103) → one dedicated thread per Storage (2106–2160). `IndexJob::{One, Batch, Flush(ack)}` (1521–1534). Batches < **`PARALLEL_INSERT_MIN = 1024`** insert sequentially (parallel_insert leaves nodes unreachable on small graphs — RCA note 2133–2143); ≥1024 use rayon `parallel_insert`. `stage()` (1421–1460) appends arena + metadata + sidecar row under `meta → arena → sidecar` lock order, so vectors are durable/reconstructable immediately but only *eventually searchable*.
- **Quantization state**: SQ8 fixed affine **(scale, bias) = (127.5, 127.5)** (582–585) assuming [-1,1]; calibrated `sq8c` (quantile range at compaction) opt-in. BQ = sign bit per dim packed to u64 words; optional per-dim mean centering (`bq_center`, computed at compaction from sidecar f32, persisted `bqmean_<name>.bin`). **Rerank sidecar is OFF-RAM (post-P0)**: `SidecarReader` (1052–1058) does positioned reads (`seek_read`/`read_at`, never mmap) on `fvec_<name>.bin`, row at byte offset `arena_id*dim*4`, fronted by a hand-rolled LRU of **`SIDECAR_CACHE_ROWS = 4096`** rows (~16 MiB max at dim 1024) (1034–1038). `CollectionInfo.sidecar_resident_bytes` exists to prove residency ≈ 0 (299–311).

## 7. compute_impact (K-Impact — being cut)

- `compute_impact` (2641–2661): `dd = min(in_degree/10, 1.0)`; tier score from labels (MASTER 1.0 / SPEC 0.8 / ADR 0.6 / USER 0.3); `sc = calculate_sc` (2626–2639, props.stability string → 1.0/0.8/0.4/0.1). Final: **`dd*0.5 + as*0.3 + sc*0.2`**. New nodes seed `impact = Some(0.7)` (2728); unknown node → 0.7 (2644).
- **Hook sites** (what removal frees): `refresh_impacts` called on every `add_edge` (2772), every `retract_edge` (5593), and batch-committed edges (2583) — a write-path recompute of the target node on every edge mutation (and `refresh_impacts(None)` recomputes ALL nodes). Read-side: hybrid_search alpha blend (3643, default 0.0 already), `get_ranked_context` hardcoded alpha 0.4 (3677), `semantic_verify` consensus gate `impact > 0.8` on MASTER conflicts (2399), SuperNode `impact` aggregation (3944, 3980), `calculate_structural_gaps` (5641+), and `NodeOutput.impact` field + `EdgeInput/EdgeOutput.impact`. Cutting it removes a per-edge-write O(1)–O(N) recompute and the last engine-imposed ranking policy in the vector path.

## 8. Secondary indexes on nodes/edges

**There are none.** Full inventory of index structures: `id_to_u32` (exact node-id intern, 1567), `out_idx`/`in_idx` (adjacency), `trigram_index` (fuzzy id only), per-collection HNSW + `node_to_arena`. **No label index** (match_pattern with a `:Label` anchor and no id does a full `nodes` scan, 3078; label predicates in WHERE scan the already-materialized result set, 2880–2891). **No property index** (all `{k:v}` and WHERE prop comparisons are per-row JSON `props.get`), **no rel-type index** (rel filters scan the mixed adjacency set), **no temporal index** (as_of is string-compare per row). The HQL P2 "label-index" item in docs/PLAN--HQL-REFINEMENT.md is unimplemented in the working tree.

## Cross-cutting facts relevant to the redesign

- The proposed `HYBRID ... TRAVERSE ... AS OF ... RANK BY rrf(...)` construct has **no existing fusion machinery to reuse**: today's only "fusion" is the alpha linear blend at 3643. RRF, recency, hops, epistemic signals would all be new operators; `NeighborOutput.score` (250) is the single score slot.
- Existing composition point: `execute_hql` (3354) is a flat match with direct dispatch; `apply_hql_clauses` (2927) is the shared WHERE/ORDER BY/LIMIT/RETURN post-processor over `Vec<NeighborOutput>` — a natural seam for a fixed-shape pipeline (search → traverse → asof → rank) since both SEARCH and TRAVERSE already emit that row type.
- G3 latency note: a chained vector→graph→as_of query today would pay hybrid_search's post-filter truncation, then neighbors' per-result BFS with full-path cloning — all in-process, zero round-trips, which is the moat's structural advantage; the weak spots vs app-composition are the missing filter-aware ANN (G1) and rel-filter scans / row-JSON materialization in match_pattern (G2).