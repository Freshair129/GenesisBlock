---
proposed_id: ADR--GENESISDB-ASYNC-INDEXING
type: adr
status: accepted
aliases:
  - ADR
phase: 34
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-06-22T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-ASYNC-INDEXING

## Context

[[RCA--ASYNC-HNSW-INDEXING]] (confirmed) measured query **P95 rising
1.13 → 6.31 ms** under concurrent ingest (`shadow-sync-stress`). Root cause:
HNSW insertion ran **on the write hot path**, synchronously, contending with
queries for the same per-collection index. `hnsw_rs` `insert`/`search` are
internally synchronized, so a writer mid-insert directly stalls a concurrent
reader; under 12 parallel writers the `parallel_insert` work also saturated the
cores readers need. The vector's durability never depended on the HNSW insert —
`stage()` appends to the arena synchronously and the WAL `Event::Node` carries
the embedding; the index is a derived structure rebuilt by `rehydrate`.

## Decision

Move live HNSW insertion **off the caller thread** onto a dedicated per-Storage
**indexing thread** fed by a bounded channel (mirrors the WAL-writer thread).

1. **`IndexJob`** = `One { coll, arena_id, emb, ef_c }` |
   `Batch { coll, items, ef_c }` | `Flush(ack)`. The indexing thread drains jobs
   and calls `coll.hnsw.insert` / `parallel_insert`. Bounded (`bounded(4096)`)
   for **backpressure** — a sustained bulk load blocks the writer rather than
   growing an unbounded queue of vector copies.
2. **Write path stages, then enqueues.** `add_node`/`execute_batch` call
   `coll.prep` + `coll.stage` (arena append — durable, immediately in-memory)
   and `enqueue_one`/`enqueue_batch`; they return **without** building the graph.
   Vectors are **eventually searchable** (Qdrant/Chroma semantics).
3. **Startup WAL replay stays synchronous.** `replay_vector(.., index=false)`
   stages only; the post-load `rehydrate_hnsw_index` builds every index once —
   enqueuing during replay would double-insert. Runtime CRDT sync
   (`reconcile_state`) uses `index=true` (stage + enqueue), since no rehydrate
   follows it.
4. **`flush_index()` / `index_lag()`.** `flush_index` enqueues a `Flush` job and
   blocks on its ack — FIFO guarantees all prior inserts are done. Called before
   any op that **reassigns arena ids** (`perform_index_compaction`,
   `rebuild_index_parallel`) so a queued insert never targets a stale id.
   `index_lag` reports staged-but-unindexed vectors.

## Consequences

### Positive
- Query P95 under concurrent ingest **6.31 → 0.60 ms** (≈10×; below the 1.13 ms
  idle baseline) — writers no longer hold the index or burn reader cores.
- Bulk indexing keeps `parallel_insert` (rayon) — it just runs on the indexing
  thread, so the caller is unblocked while throughput is preserved.
- Durability unchanged: WAL ack still precedes return; only HNSW *visibility* is
  deferred, and the arena (its source of truth) is written synchronously.

### Negative / Trade-offs
- **Eventually searchable**: a vector is not findable until the queue drains.
  Tests/operations that need read-your-write call `flush_index()`. The bounded
  queue applies backpressure (writer blocks) under sustained overload — a
  deliberate cap, not silent unbounded growth.
- One extra long-lived thread per `Storage` (alongside the WAL writer + gossip).
- Compaction/rebuild must `flush_index()` first (encoded) to avoid stale-arena-id
  inserts.

## Alternatives Considered
| Alternative | Reason Rejected |
|---|---|
| Keep synchronous insert | The measured P95 regression is the whole problem. |
| Per-collection indexing thread | N threads for N collections; one thread keyed by the job's `Arc<VectorCollection>` is simpler and the index is the bottleneck, not thread count. |
| Unbounded queue | Transient RAM blows up under bulk load (a second copy of every pending vector); bounded gives backpressure. |
| Make `search` flush first | Defeats the purpose — every query would block on pending writes, recreating the contention. |

## Verification
- `tests/async_indexing_tests.rs` (4): searchable-after-flush; `flush_index`
  drives `index_lag` to 0; bulk-then-flush indexes all (probed by exact-match
  top-1, since HNSW is approximate); crash/reopen (pure WAL replay) rebuilds the
  full index synchronously at open. Full `cargo test` (45) + `npm test` (7) green.
- `shadow-sync-stress` (12 writers / 4 readers, 10k × 1536-dim): **P95 0.60 ms**
  under load (target < 10 ms met); WAL replay durability verified.

### Outcome (measured 2026-06-22)
Shipped. Reader **P95 under concurrent ingest 6.31 → 0.60 ms** (≈10×). Ingest
217 TPS / 45.9 s for 10k×1536-dim on HDD (fsync-bound, not the lever). Durability
(WAL replay) intact.

---
### Related Links
- **Root Cause:** [[RCA--ASYNC-HNSW-INDEXING]]
- **Vector spaces:** [[ADR--GENESISDB-MULTI-COLLECTION]]
- **Probe:** `benches/shadow_sync_stress.rs`

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-22 | Proposed & accepted & shipped: deferred HNSW indexing on a bounded-queue thread; eventually-searchable + flush_index/index_lag; P95 6.31→0.60 ms. |
