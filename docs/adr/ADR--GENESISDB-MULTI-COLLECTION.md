---
proposed_id: ADR--GENESISDB-MULTI-COLLECTION
type: adr
status: accepted
aliases:
  - ADR
phase: 33
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-06-22T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-MULTI-COLLECTION

## Context

[[SPEC--MULTI-COLLECTION-VECTOR-SPACE]] §1: the engine had **exactly one global
vector space** (`vector_arena` / `metadata_arena` / `hnsw_index` / scalar
`vector_dim` / `u32_to_arena_id` in `src/lib.rs`). This blocks the core
local-agent use case — mixing embedding models. `jina-code-embeddings-1.5b`
(dim 1536, code) and `bge-m3` (dim 1024, text) have **incompatible** spaces;
distances across models are meaningless, so they must live in separate indices
even when dims happen to match. The single space also silently accepted
wrong-dim queries (garbage neighbors, no error). P-A (dynamic capacity) was
dropped (~8 MB, not the lever) and P-B (embedding dedup) shipped earlier; P-C
(collections) + P-D (surface) were the open work.

## Decision

Replace the global space with **per-collection isolated vector spaces**.

1. **`VectorCollection`** (`name`, `model`, `dim`, `metric`, `arena`,
   `metadata`, `hnsw`, `node_to_arena`, `count`). `Storage` drops
   `vector_arena`/`metadata_arena`/`hnsw_index`/`vector_dim`/`u32_to_arena_id`
   and gains `collections: DashMap<String, Arc<VectorCollection>>` +
   `default_collection`. The `default` collection is always created at open
   (dim = `OpenOptions.vector_dim`), so single-space behavior is preserved.
2. **Metric.** `Metric::{L2, Cosine}`. To keep one `DistL2` HNSW index type,
   Cosine collections store **L2-normalized** vectors and normalize the query
   too (normalize-then-L2 ≡ cosine ranking; SPEC §10). No per-metric index type.
3. **Routing + validation.** `NodeInput.collection` routes a node's embedding to
   a collection (default if unset); `NodeOutput.collection` records it so WAL
   replay rebuilds the right space. `add_node` / `execute_batch` **dim-validate
   before persist** (all-or-nothing); `hybrid_search` resolves the collection,
   validates query dim, and searches only that space — a typed error replaces
   the old silent cross-space bug.
4. **Reuse `NodeMetadata`** as the per-collection metadata element (not a new
   `VectorMeta`), so community detection / meta-graph / structural-gaps keep
   their field accesses; those autonomic features run over the **default**
   collection. Compaction loops every collection independently.
5. **Persistence (snapshot-first).** `save_state` writes `vec_<name>.bin` +
   `meta_<name>.bin` per collection and a `collections` manifest in
   `state.json`; HNSW is **not** dumped (it rehydrates from each arena on load).
   `try_load_state` reads the manifest; the atomic swap enumerates temp files
   (dynamic per-collection names) instead of a fixed list.
6. **Migration.** No manifest + a legacy `vector.bin`/`meta.bin` pair → wrap into
   `default` (dim from the old `state.json` vector_dim). Old DBs open unchanged.
7. **WAL-replay recovery.** `create_collection` is in-memory (durable only via
   the snapshot manifest). On pure WAL replay — or a CRDT-sync node referencing a
   collection we lack — `replay_vector` **auto-provisions** the collection from
   the embedding's dim (L2, model `recovered`); a later `save_state` records the
   true model/metric. The live add path stays strict.
8. **Surface (P-D).** NAPI `create_collection` / `list_collections` (+
   `CollectionInfo`); REST `POST /v1/collection/create`, `GET /v1/collections`;
   `index.d.ts` regenerated. **HQL `IN <collection>` is deferred** (grammar
   source ambiguity, CLAUDE.md) and so is the same-node-multi-vector
   `add_vector` op (would need a new `Event::Vector` variant rippling through 4
   exhaustive Event matches + CRDT/governance) — both noted as follow-ups.

## Consequences

### Positive
- Code and text embeddings coexist in isolated spaces; cross-space search is
  impossible. Wrong-dim insert/query is a typed error, not garbage.
- Backward compatible: legacy single-space DBs migrate to `default` transparently;
  the default-collection path is byte-for-byte the old behavior.
- Per-collection snapshot files + manifest; WAL-replay recovers custom collections.

### Negative / Trade-offs
- Community detection / meta-graph / structural-gaps operate on the **default**
  collection only (v1). Nodes whose vectors live solely in other collections
  aren't clustered — acceptable; documented.
- Cosine is normalize-then-L2, not a native cosine index (equivalent ranking;
  avoids a second index type).
- `create_collection` durability is snapshot-driven; pure WAL replay recovers
  collections with correct dim but default model/metric until the next snapshot.
- Same-node-multi-vector (`add_vector`) and HQL `IN` deferred.

## Alternatives Considered
| Alternative | Reason Rejected |
|---|---|
| New `VectorMeta` type (per SPEC §3) | Would rewrite every community/meta/gaps field access for no functional gain; reusing `NodeMetadata` is surgical. |
| Per-metric HNSW index type (true cosine) | Two index types to thread everywhere; normalize-then-L2 gives identical ranking with one type. |
| `Event::Vector` + `add_vector` now | New Event variant ripples through 4 exhaustive matches + CRDT/governance/compaction; high risk for a secondary capability. Deferred. |
| Persist HNSW dumps per collection | Rehydrate-from-arena is cheap and avoids stale/oversized index files; arena is the source of truth. |

## Verification
- `tests/multi_collection_tests.rs` (6): isolation across dims, dim-mismatch
  insert+query typed errors, unknown-collection error, default routing +
  `list_collections` counts, snapshot round-trip (model/metric/dim/count +
  searchable), WAL-replay recovery of a custom collection.
- Full `cargo test` (41 integration tests) + `npm test` (7 NAPI/MCP) green.
- REST `POST /v1/collection/create` + `GET /v1/collections` wired in `src/main.rs`.

### Outcome (2026-06-22)
Shipped P-C + P-D (minus HQL `IN` and `add_vector`, deferred). Single global
vector space replaced by `collections` map; default-collection back-compat and
legacy migration verified; correctness-gated (no perf claim — RAM is unchanged
vs the post-P-B baseline since the same arenas are just partitioned by name).

---
### Related Links
- **Spec:** [[SPEC--MULTI-COLLECTION-VECTOR-SPACE]]
- **Embedding dedup (P-B):** prior commit `75d560e`
- **Edge RAM levers:** [[ADR--GENESISDB-EDGE-ID-INTERNING]], [[ADR--GENESISDB-EDGE-NUMERIC-KEYS]]

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-22 | Proposed & accepted: per-collection vector spaces (P-C/P-D); normalize-then-L2 cosine; snapshot manifest + legacy migration + WAL-replay recovery; HQL `IN` and `add_vector` deferred. |
