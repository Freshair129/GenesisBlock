---
proposed_id: ADR--GENESISDB-ADD-VECTOR
type: adr
status: accepted
aliases:
  - ADR
phase: 36
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-06-22T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-ADD-VECTOR

## Context

Multi-collection ([[ADR--GENESISDB-MULTI-COLLECTION]]) gave each collection its own
model/dim/metric/arena/HNSW, but a node could only ever receive **one** vector —
the `embedding` on its `Event::Node`, routed to `NodeInput.collection`. The
intended use case (P-D, deferred) is a single node carrying **several** vectors in
different spaces: e.g. a source-file node with a `jina-code` (1536-d) embedding in
collection `code` *and* a `bge-m3` (1024-d) embedding in collection `text`, so the
same node is retrievable by code similarity or by natural-language similarity.

There was no event or op to attach an additional vector to an existing node.

## Decision

Add a first-class **`Event::Vector`** and a `Storage::add_vector` op.

1. **`Event::Vector(VectorEvent)`** where
   `VectorEvent { node_id, collection: Option<String>, embedding: Vec<f64>, lang }`.
   It carries the f64 embedding for replay, exactly mirroring how `Event::Node`
   carries a node's primary embedding.
2. **`add_vector(node_id, collection, embedding)`**: requires the node to exist;
   validates the embedding dim against the collection; stages the vector into the
   collection's arena (durable, immediately in-memory) and enqueues the deferred
   HNSW insert (eventually searchable, [[ADR--GENESISDB-ASYNC-INDEXING]]); persists
   `Event::Vector`. A node holds at most one vector per collection — re-adding to a
   collection it already has supersedes the prior `node_to_arena` mapping, and the
   orphaned arena slot is reclaimed at the next compaction.
3. **`Event::Vector` handled in all four exhaustive `Event` matches:**
   - **WAL replay (open):** `replay_vector(.., index=false)` — stage only; the
     post-load `rehydrate_hnsw_index` builds every index once.
   - **`reconcile_state` (CRDT sync):** `replay_vector(.., index=true)` (stage +
     enqueue; auto-provisions the collection if the peer lacks it) + persist.
   - **`semantic_verify`:** `Ok(true)` — a vector attachment carries no
     governance/axiom implication.
   - **`submit_vote` (consensus apply):** persist only — vectors aren't axiom
     subjects, but a proposed one stays durable.
4. **NAPI** `addVector(nodeId, collection, embedding)` + **REST**
   `POST /v1/vector/add` `{ node_id, collection, embedding }`.

### Why no compaction / snapshot changes were needed
Compaction (`perform_index_compaction`) rebuilds each collection's arena from that
collection's **own metadata** (`meta_arena.iter()`), keeping every vector whose
`node_id` maps to a live node — not from `node.embedding`. Snapshots dump each
collection's arena (`vec_<name>.bin`). And `compact()` already strips embeddings
from the rewritten WAL (it reads the in-memory lean nodes), so vector durability
already rests on the arena, not the WAL, post-compaction. An `add_vector` vector
therefore gets **identical durability to a node's primary embedding** with zero
changes to those paths.

## Consequences

### Positive
- One node, many vectors: code+text (or any multi-model) retrieval over the same
  graph node — the core agentic-memory / hybrid-RAG use case.
- Durable and sync-aware: survives WAL replay and propagates over CRDT sync,
  reusing `replay_vector`'s existing index-vs-stage discipline.
- Minimal blast radius: one event variant, one op, four one-line match arms; no
  changes to compaction, snapshot, or the node schema.

### Negative / Trade-offs
- Re-adding to a collection a node already occupies leaves a dead arena slot until
  compaction (same pattern as node supersession).
- The node record does not enumerate its non-primary vectors; the per-collection
  arenas are the source of truth (consistent with the engine's existing model).

## Alternatives Considered
| Alternative | Reason Rejected |
|---|---|
| Multi-embedding field on `NodeInput`/`Event::Node` | Bloats every node event; doesn't fit attaching a vector *after* node creation; complicates supersession. |
| Reuse `Event::Node` with empty graph fields | Conflates node mutation with vector attachment; pollutes node history and governance checks. |
| Store all vectors of a node in one collection | Defeats per-model isolation — cross-model distances are meaningless ([[ADR--GENESISDB-MULTI-COLLECTION]] §1). |

## Verification
- `tests/add_vector_tests.rs` (5): two-collection attach + per-space search;
  missing-node error; dim-mismatch error; unknown-collection error; **WAL-replay
  durability** (reopen with no snapshot replays `Event::Vector` into the
  auto-provisioned collection, both vectors searchable).
- Full `cargo test` (30 binaries) + `npm test` (7) green; `index.d.ts`
  regenerated with `addVector`.

### Outcome (measured 2026-06-22)
Shipped. A node can hold one vector per collection; attach is durable (WAL
`Event::Vector`) and sync-propagating, with no compaction/snapshot changes.

---
### Related Links
- **Vector spaces:** [[ADR--GENESISDB-MULTI-COLLECTION]]
- **Async indexing:** [[ADR--GENESISDB-ASYNC-INDEXING]]
- **Spec:** SPEC--MULTI-COLLECTION-VECTOR-SPACE §6 (P-D)

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-22 | Proposed & accepted & shipped: `Event::Vector` + `add_vector` (NAPI `addVector`, REST `POST /v1/vector/add`); one node, one vector per collection; durable via WAL replay; no compaction/snapshot changes needed. |
