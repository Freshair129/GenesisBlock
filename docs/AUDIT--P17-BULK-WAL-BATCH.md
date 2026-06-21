---
proposed_id: AUDIT--P17-BULK-WAL-BATCH
type: audit
status: complete
aliases:
  - AUDIT
  - P17
tier: process
cluster: implementation_flow
role: "Bulk ingest — single Event::Batch per chunk (one fsync)"
phase: 17
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P15-COMPETITIVE-VECTOR-BENCHMARK
  - AUDIT--P16-CONCURRENCY-LOCK-FIX
  - AUDIT--P13-GROUP-COMMIT-REPORT
---

# AUDIT — P17 Bulk WAL Batch

## 1. Problem

`bulk_add_nodes` / `bulk_add_edges` looped `add_node` / `add_edge`, so every item
paid its own WAL `sync_all()` round-trip. The competitive harness (P15) used the
same per-item path: GenesisBlockDB inserted 3,000 vectors at **254 vec/s** vs Chroma's
batched **4,074 vec/s**.

## 2. Fix

The engine already had `execute_batch` (build all events → persist ONE
`Event::Batch` → one fsync → index). The bulk APIs now route through it in
chunks of 1,024 (one fsync per chunk, bounding the size of a single serialized
batch). `vbench_genesis` insert switched from per-item `add_node` to
`bulk_add_nodes`, matching Chroma's batched `add`.

## 3. Result — vbench, 3,000 × 1024-dim, C: SSD (same vectors)

| Metric | Per-op (P15) | Batch (P17) | Chroma |
|---|---|---|---|
| Insert | 254 vec/s (11.82 s) | **385 vec/s (7.80 s)** | 4,074 vec/s |
| Query p50 | 1,901 µs | 1,698 µs | 1,249 µs |
| Recall@10 | 0.987 | 0.986 | 1.000 |

Per-op cost 3.94 ms → 2.60 ms: batching removed the ~1.3 ms/op WAL fsync
round-trip (**+52%** insert).

## 4. Honest reading — what's left

Batching the WAL did **not** close the gap to Chroma, because once fsync is
amortized the **single-threaded HNSW build dominates** (~2.6 ms/insert at
hnsw_rs `ef_construction=200`, dim 1024). Chroma's hnswlib builds at ~0.25
ms/insert. The remaining ~10× is index-build speed, not durability and not the
WAL.

Closing it (future **Fix #3**):
- `Hnsw::parallel_insert` (rayon) on the bulk path → multi-core build.
- Tune `ef_construction` toward Chroma's default (100) — currently 200 doubles
  build work for marginal recall.
- These are bulk-only; per-record durable insert stays as-is.

Note: this is single-threaded bulk. **Concurrent** durable ingest is already
strong after P16 (839 TPS, 12 writers).

## 5. Verification

`cargo test` green (batch path is exercised by `batch_atomicity_tests`).
Durability unchanged (atomic all-or-nothing `Event::Batch`).
