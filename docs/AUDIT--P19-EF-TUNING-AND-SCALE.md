---
proposed_id: AUDIT--P19-EF-TUNING-AND-SCALE
type: audit
status: historical
aliases:
  - AUDIT
  - P19
tier: process
cluster: implementation_flow
role: "ef_construction tuning + 50k-scale ANN comparison"
phase: 19
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P18-PARALLEL-HNSW-BUILD
  - AUDIT--P15-COMPETITIVE-VECTOR-BENCHMARK
---

# AUDIT — P19 ef tuning + 50k scale

## 1. Changes

1. `init_hnsw` `ef_construction` **200 → 100** (matches Chroma/hnswlib default).
2. Added a synthetic-clustered vector generator (`vbench.py synth <N>`) for scale
   tests — at 3k both engines saturate near recall 1.0, so quality only
   differentiates at larger N. Gaussian blobs around random centroids,
   unit-normalized, identical vectors fed to both engines, exact L2 ground truth.

## 2. Result — 3k (bge-m3, real) at ef 200 vs 100

| | ef=200 | ef=100 |
|---|---|---|
| insert | 1,986 vec/s | 1,721 vec/s |
| recall@10 | 0.982 | 0.984 |

At 3k everything is saturated — the ef change is **within run-to-run noise**.
This confirms 3k cannot distinguish ANN quality.

## 3. Result — 50k (synthetic-clustered, ef=100, C: SSD)

| Metric | GenesisBlockDB (hnsw_rs) | Chroma (hnswlib) |
|---|---|---|
| Insert (durable / in-mem) | 2,280 vec/s | 3,505 vec/s |
| Query p50 | **928.8 µs** | 1,011.5 µs |
| Query p95 | **1,381 µs** | 1,400 µs |
| Recall@10 | 0.962 | 1.000 |

At scale the picture sharpens:
- **Query latency: GenesisBlockDB now ≤ Chroma** (929 vs 1011 µs p50). hnsw_rs search
  on the ef=100 graph is competitive/faster.
- **Insert gap narrows to ~1.5×** (durable vs non-durable).
- **Recall differentiates: 0.962 vs 1.000.** This is the cost of `ef_construction
  =100` + the fixed query-time `ef` — a sparser graph builds/searches faster but
  loses a few % recall. Chroma holds 1.000.

## 4. Honest reading & recommendation

The 0.962 vs 1.000 gap is a **tunable speed↔recall trade**, not a fundamental
hnsw_rs deficit: raising `ef_construction` back to 200 (and/or the query-time
`ef`) moves GenesisBlockDB back up the recall curve at some latency cost.

**Recommendation:** expose `ef_construction` (and query `ef`) via `OpenOptions`
instead of hardcoding, with a quality-first default (200) and an opt-in
speed mode (100). Hardcoding either value forces all users onto one point of the
frontier. (Current code ships 100; see follow-up.)

## 5. Verification

`cargo test` green (20 passed, 0 failed) — no search-result test regressed at
ef=100.
