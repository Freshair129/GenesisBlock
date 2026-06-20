---
proposed_id: AUDIT--P16-CONCURRENCY-LOCK-FIX
type: audit
status: complete
aliases:
  - AUDIT
  - P16
tier: process
cluster: implementation_flow
role: "Concurrency fix — remove per-op global HNSW write lock"
phase: 16
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P13-GROUP-COMMIT-REPORT
  - AUDIT--P14-POST-REFACTOR-VERIFICATION
---

# AUDIT — P16 Concurrency Lock Fix

## 1. Problem (P13's open bottleneck)

P13 concluded that durable write throughput was capped by "every add_node/add_edge
locking the global RwLock of the HNSW index." `shadow_sync_stress` (12 writer
threads, 10k notes × 1536-dim) measured **136.60 TPS on SSD** — 12 cores behaving
like ~1 because every insert serialized.

Root cause in `add_vector_internal` (`src/lib.rs`):
1. `self.hnsw_index.write()` was taken **per insert** — a global exclusive lock.
2. `metadata_arena.write()` + `vector_arena.write()` guards were held for the
   **entire function**, including the slow (~ms) HNSW insert.

## 2. Fix

Key fact: hnsw_rs `Hnsw::insert(&self, ...)` takes `&self` — it is internally
synchronized and supports concurrent inserts. The outer `RwLock<Option<Hnsw>>`
`.write()` was redundant over-locking.

- HNSW insert now runs under a **shared `.read()`** lock (write used only for a
  one-time, double-checked lazy init).
- The arena `.write()` guards are scoped to **just the Vec pushes** (~µs) and
  released **before** the HNSW insert.

~2-line semantic change + a scope tightening; no API/behaviour change.

## 3. Result — `shadow_sync_stress`, 12 writers, 10k × 1536, C: SSD

| Metric | Before | After | Δ |
|---|---|---|---|
| Ingest throughput | 136.60 TPS | **839.36 TPS** | **×6.1** |
| Ingest time | 73.21 s | **11.91 s** | −84% |
| P95 query under load | 1.13 ms | 6.31 ms | still < 10 ms target |
| Durability (WAL replay) | ✅ | ✅ | unchanged |

P95 rose because 12 writers now genuinely run in parallel (real contention
inside a 6× shorter window) — still inside the < 10 ms target. Snapshot save +
instant load + `Note-9999` WAL-replay verification still pass.

## 4. Verification

`cargo test`: green (20 passed, 0 failed, 21 binaries) — the locking change is
concurrency-only; single-threaded behaviour is identical.

## 5. Remaining headroom (not in this change)

- **Batch WAL path** (`execute_batch` → one `Event::Batch` → one fsync) for bulk
  ingest and the per-op fsync round-trip.
- **`parallel_insert` / `parallel_search`** (hnsw_rs, rayon) for bulk build/query.
- **Deferred/async indexing** to take HNSW off the write hot path entirely.
- Arena `RwLock<Vec>` is now the next (much smaller) serialization point; shard
  if profiling demands.
