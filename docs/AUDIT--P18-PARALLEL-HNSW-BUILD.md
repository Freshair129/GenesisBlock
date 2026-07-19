---
proposed_id: AUDIT--P18-PARALLEL-HNSW-BUILD
type: audit
status: historical
aliases:
  - AUDIT
  - P18
tier: process
cluster: implementation_flow
role: "Bulk HNSW build via rayon parallel_insert"
phase: 18
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P15-COMPETITIVE-VECTOR-BENCHMARK
  - AUDIT--P17-BULK-WAL-BATCH
---

# AUDIT — P18 Parallel HNSW Build

## 1. Problem

After P17 batched the WAL, single-threaded HNSW build dominated bulk insert
(~2.6 ms/insert). The batch index phase still inserted vectors into the HNSW
graph **one at a time** on a single core.

## 2. Fix

hnsw_rs exposes `Hnsw::parallel_insert(&[(&Vec<T>, usize)])` (rayon, multi-core).
`add_vector_internal` was split into `stage_vector` (arena/metadata push, no
index) + the HNSW insert. The batch path (`execute_batch`, reached by
`bulk_add_nodes`) now stages every vector, then builds the graph with **one
`parallel_insert`** across all cores. Single `add_node` still uses the
sequential `insert`.

## 3. Result — vbench, 3,000 × 1024-dim, C: SSD (same vectors, bge-m3)

| insert path | vec/s | time | recall@10 |
|---|---|---|---|
| Per-op `add_node` (P15) | 254 | 11.82 s | 0.987 |
| Batch WAL (P17) | 385 | 7.80 s | 0.986 |
| **Batch WAL + parallel_insert (P18)** | **1,986** | **1.51 s** | 0.982 |

vs **Chroma (hnswlib): 4,074 vec/s** (in-memory, non-durable).

- **×7.8 overall** (254 → 1,986); ×5.2 from parallel_insert alone.
- Gap to Chroma closed from **16× → ~2×** — and GenesisBlockDB's 1,986 is **durable**
  (WAL-persisted) while Chroma's 4,074 is in-memory ephemeral.
- Recall 0.986 → 0.982: parallel build yields a marginally different graph;
  negligible. Query latency unchanged (1,668 µs p50).

## 4. Honest reading

The remaining ~2× is raw index-build speed (hnsw_rs vs hnswlib's hand-tuned
SIMD) plus the f64→f32 input conversion and durability overhead. Halving
`ef_construction` (200→100, Chroma's default) would narrow it further at a small
recall cost — deferred (kept 200 to isolate the parallel_insert effect).

A durable engine landing within ~2× of a non-durable C++ ANN on bulk vector
load is a strong, honest result.

## 5. Verification

`cargo test` green (execute_batch / hybrid_search paths exercised). Durability
and atomic batch semantics unchanged.
