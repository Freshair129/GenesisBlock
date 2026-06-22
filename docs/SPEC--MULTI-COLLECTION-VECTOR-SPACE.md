---
version: "0.1.0b"
created_at: "2026-06-21,ATHER"
status: "Proposed"
attributes:
  domain: "storage-engine"
  doc_type: "feature-spec"
  scope: "src/lib.rs"
  complexity: "C-3"
  target_path: "G:\\GenesisBlock_Dev\\GenesisBlock\\docs\\SPEC--MULTI-COLLECTION-VECTOR-SPACE.md"
---

# SPEC — Multi-Collection Vector Space

## 1. Motivation

Three problems, one root cause: the engine has **exactly one global vector space**
(`vector_arena`, `metadata_arena`, `hnsw_index`, scalar `vector_dim`) in
`src/lib.rs:336-339`.

1. **Cannot mix embedding models.** Local models have different dims and, more
   importantly, **incompatible vector spaces**: `jina-code-embeddings-1.5b` (dim
   1536, code) vs `bge-m3` (dim 1024, multilingual text). Distances across models
   are meaningless — they must live in separate indices even when dims match.
2. **Per-node memory waste from triple-stored embeddings.** Each node kept its
   vector 3×: `node.embedding: Vec<f64>` (12 KB/node at dim 1536, in `nodes`),
   `vector_arena: Vec<f32>` (6 KB), and the HNSW internal copy (6 KB). The f64
   node copy was pure waste — search runs on the f32 arena/HNSW.
   **[RESOLVED — see §5, P-B]** Measured at 5k nodes/dim 1536: net RSS dropped
   147 MB → 82 MB (~44%); extrapolated 32k: ~960 MB → ~525 MB.

> **Correction (measured 2026-06-21).** An earlier hypothesis blamed
> `init_hnsw()`'s hardcoded `max_elements = 1_000_000` (`src/lib.rs:416`).
> Reading hnsw_rs 0.3.4 (`PointIndexation::new`) shows `max_elements` is only a
> `Vec::with_capacity` *hint* for the layer pointer tables, not a hard cap and
> not vector-data pre-allocation — its real cost is ~8 MB, and inserts beyond it
> simply grow the Vec. The audit's **15.89 GB / 32k nodes** figure is an
> artifact of the old Mark VII engine; the current engine measures ~1 GB at
> 32k. `max_elements` tuning is therefore deprioritized (§4).

## 2. Goals / Non-Goals

**Goals**
- A node may hold at most one vector per *collection*; each collection owns its
  own model, dim, metric, arena, metadata, and HNSW index.
- Vector search is scoped to a single collection; query-dim is validated against
  the collection dim (closes the silent cross-space bug).
- HNSW capacity grows with data (amortized doubling), not a fixed 1M.
- Eliminate the redundant in-memory f64 embedding copy.
- Backward-compatible load: existing single-space data migrates to a `default`
  collection.

**Non-Goals**
- No cross-collection joined ranking in v1 (search one collection at a time).
- No change to the graph model (nodes/edges/traversal) beyond adding the
  collection dimension to vectors.
- Edge `from`/`to` typing is tracked separately (see §9 dependency).

## 3. Core types

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Metric { L2, Cosine }

/// One isolated vector space. All vectors in a collection come from ONE model.
pub struct VectorCollection {
    pub name: String,
    pub model: String,        // provenance, e.g. "jina-code-embeddings-1.5b"
    pub dim: u16,             // 1536 / 1024 / 768 ...
    pub metric: Metric,
    pub arena: RwLock<Vec<f32>>,              // dense, row-major, stride = dim
    pub metadata: RwLock<Vec<VectorMeta>>,    // arena_id -> {node_id, offset}
    pub hnsw: RwLock<Option<Hnsw<'static, f32, DistL2>>>,
    pub node_to_arena: DashMap<u32, u32>,     // node u32 -> arena_id (this collection)
    pub count: AtomicUsize,                   // live vectors
    pub capacity: AtomicUsize,                // current HNSW reservation
}
```

`Storage` change:
```rust
// REMOVE: vector_arena, metadata_arena, hnsw_index, vector_dim, u32_to_arena_id
// ADD:
pub collections: DashMap<String, Arc<VectorCollection>>,
pub default_collection: String,   // "default"
```

`NodeMetadata.cluster_id` / community fields stay in node-graph land; vector
metadata moves into `VectorMeta { arena_id, node_id, embedding_offset, lang }`.

## 4. Dynamic HNSW capacity (DEPRIORITIZED — ~8 MB, not the RAM fix)

> Superseded by the §1 correction: `max_elements` is a `with_capacity` hint
> (~8 MB at 1M), not the RAM driver. Keep only as minor hygiene; the real RAM
> win was the embedding dedup in §5. Left here for reference.

Replace the hardcoded constructor with size-aware construction + amortized
doubling. Do **not** rely on hnsw_rs auto-growth; rebuild from the arena (we own
the source vectors) when capacity is hit.

```rust
const HNSW_M: usize = 16;
const HNSW_MAX_LAYER: usize = 16;
const HNSW_EF_CONSTRUCTION: usize = 200;
const HNSW_MIN_CAP: usize = 1024;

fn build_hnsw(cap: usize) -> Hnsw<'static, f32, DistL2> {
    Hnsw::new(HNSW_M, cap.max(HNSW_MIN_CAP), HNSW_MAX_LAYER, HNSW_EF_CONSTRUCTION, DistL2 {})
}

// on insert into a collection:
//   if count + 1 > capacity { rebuild HNSW at capacity*2 by re-inserting arena }
//   else hnsw.insert((&emb, arena_id))
// on snapshot rehydrate:
//   capacity = (count).next_power_of_two().max(HNSW_MIN_CAP); build_hnsw(capacity)
```

Amortized O(1) per insert (geometric rebuilds), peak reservation ≤ 2× actual.

## 5. Embedding dedup — IMPLEMENTED (P-B, commit 75d560e)

- `Storage::insert_node_lean()` strips `embedding` before storing into the
  in-memory `nodes` map. Every insert site routes through it (add_node,
  supersede_node, WAL replay, execute_batch, reconcile_state, axiom bootstrap,
  snapshot load). The arena is the single source of truth for f32 vectors.
- Durability preserved: WAL `Event::Node` still carries the full f64 embedding
  for replay → arena rebuild; snapshots persist the arena in `vector.bin` and
  rehydrate HNSW on load (both load paths, fixed alongside).
- **Measured:** 5k nodes/dim 1536, net RSS 147 MB → 82 MB (~44%). Extrapolated
  32k: ~960 MB → ~525 MB.
- Behavior change: node read responses (neighbors/context/get) no longer echo
  raw embeddings — intentional and a better default for graph results.

This phase is done; §3/§6/§7 (per-collection spaces) remain the open work.

## 6. API surface

- `NodeInput` gains `collection: Option<String>` (default `default`) and keeps
  `embedding: Option<Vec<f64>>`. The vector, if present, is routed to
  `collection`.
- New explicit vector op (REST + NAPI):
  `add_vector(node_id: String, collection: String, embedding: Vec<f64>)` — lets a
  node carry a code-embedding *and* a text-embedding in different collections.
  ✅ **DONE** (2026-06-22, [[ADR--GENESISDB-ADD-VECTOR]]): `Event::Vector` +
  `add_vector`; NAPI `addVector`, REST `POST /v1/vector/add`. Durable via WAL
  replay; no compaction/snapshot changes needed (arenas are the source of truth).
- New admin ops: `create_collection(name, model, dim, metric)`,
  `list_collections() -> Vec<CollectionInfo>`.
- `HybridSearchInput` gains `collection: Option<String>`.
- HQL: optional `IN <collection>` clause on `SEARCH`/`MATCH`:
  `SEARCH target SIMILAR TO [..] K 5 IN "code"`. Grammar adds
  `in_clause = { ^"IN" ~ string_lit }`; default collection if omitted.

**Dim validation:** every search validates `query.len() == collection.dim`,
returning a typed error instead of silently producing garbage neighbors.

## 7. Persistence & migration

Per-collection files under the DB path:
- `vec_<name>.bin`     — raw f32 LE arena
- `meta_<name>.bin`    — bincode `Vec<VectorMeta>`
- `index_<name>.hnsw.{graph,data}` — hnsw_rs dump (optional; rehydrate from arena
  is cheap)

`state.json` gains a `collections` manifest:
```json
{ "collections": [ {"name":"code","model":"jina-code-embeddings-1.5b","dim":1536,"metric":"L2"},
                   {"name":"text","model":"bge-m3","dim":1024,"metric":"Cosine"} ] }
```

**Migration:** on load, if legacy `vector.bin`/`meta.bin` exist and no manifest is
present, wrap them as collection `default` with `dim = stored vector_dim`. No
user action required; old DBs keep working.

## 8. Phased implementation

1. ~~**P-A (RAM):** dynamic `max_elements`.~~ **DROPPED** — measured ~8 MB
   impact (§1 correction, §4).
2. **P-B (dedup):** ✅ **DONE** (commit 75d560e) — `insert_node_lean`, ~44% RSS.
3. **P-C (collections):** ✅ **DONE** (2026-06-22, [[ADR--GENESISDB-MULTI-COLLECTION]]).
   `VectorCollection` + `collections` map replace the global space; `default`
   always exists; routing by `NodeInput.collection`; per-collection HNSW/arena;
   dim validation on insert + search; per-collection snapshot files + manifest;
   legacy `vector.bin`/`meta.bin` migrates to `default`; WAL-replay auto-provisions
   missing collections from the embedding dim. Community/meta/gaps run over the
   `default` collection; compaction loops all collections. Cosine = normalize-then-L2.
4. **P-D (surface):** ✅ **DONE** — `NodeInput.collection`,
   `NodeOutput.collection`, `HybridSearchInput.collection`; NAPI
   `create_collection`/`list_collections` (+ `CollectionInfo`); REST
   `POST /v1/collection/create`, `GET /v1/collections`; `index.d.ts` regenerated.
   Same-node-multi-vector `add_vector` op ✅ **DONE** (2026-06-22,
   [[ADR--GENESISDB-ADD-VECTOR]]): `Event::Vector` variant + `add_vector` (NAPI
   `addVector`, REST `POST /v1/vector/add`). HQL `IN <collection>` clause is
   tracked separately (its own branch/PR).

P-B shipped independently and de-risked the rest. **P-C/P-D shipped 2026-06-22.**

## 9. Impact (measured, not estimated)

| | Before | After P-B |
|---|---|---|
| In-memory copies of each vector | 3 (f64 node + f32 arena + f32 HNSW) | 2 (f32 arena + f32 HNSW) |
| Net RSS, 5k nodes / dim 1536 | 147 MB | 82 MB (**-44%**) |
| Extrapolated, 32k nodes | ~960 MB | ~525 MB |

The old audit's 15.89 GB was a Mark VII artifact, not current behavior. A
two-collection local stack (code 1536 + text 1024) under P-C is expected to stay
well within the `< 2 GB` budget.

## 10. Dependencies / open questions

- **Edge `from`/`to` type** — RESOLVED: kept as `String` at the API/persistence
  boundary, interned to `u32` only internally (commit b5e9771). P-C can build on
  that stable surface.
- Metric per collection implies the HNSW distance type may differ (L2 vs Cosine).
  hnsw_rs is generic over the distance; v1 may normalize-then-L2 for cosine to
  keep a single `DistL2` index type. Decide before P-C.
- Recommended default local stack: `code → jina-code-embeddings-1.5b (1536)`,
  `text → bge-m3 (1024)`, rerank with `bge-reranker-v2-m3`.
