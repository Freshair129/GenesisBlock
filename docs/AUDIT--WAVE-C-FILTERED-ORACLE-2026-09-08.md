---
doc_id: AUDIT--WAVE-C-FILTERED-ORACLE-2026-09-08
owner: GenesisBlockDB Engineering
version: "0.1.0b"
created_at: "2026-09-08T18:00:00+07:00,ATHER"
last_update: "2026-09-08T18:15:00+07:00,ATHER,d8ef9af"
status: beta
superseded_by: null
attributes:
  domain: query-correctness
  scope: wave-c-r04-filtered-ann
  complexity: C-3
  risk: MEDIUM-HIGH
---

# Wave C filtered-ANN exact-oracle audit

## 1. Purpose

This audit closes the Wave C R-04 evidence gap: a filtered vector query must
return `k` eligible distinct nodes when enough live vectors remain, even when
nearby HNSW rows have been retracted. It also measures recall against a small
exact L2 oracle and records the latency cost of the refill budget.

## 2. Method

- **Command:** `cargo test --no-default-features --release --test wave_c_oracle_probe -- --nocapture`
- **Corpus:** 4,097 deterministic 8-dimensional vectors, query `[0; 8]`,
  `k=10`; the nearest rows are retired before each run.
- **Churn:** 0%, 10%, 50% and 90% of the nearest rows.
- **Configurations:** lossless `none`, `f16`, `sq8`, calibrated `sq8c`, `bq`,
  plus `sq8` and `bq` with an f32 rerank sidecar.
- **Measurements:** eligible rows, returned rows, recall@10 against the exact
  filtered L2 ordering, and five-query p50/p95 latency.
- **Run:** release profile on Windows; total probe duration 257.35 seconds.

## 3. Result

Every cell returned 10 eligible rows. This is the primary R-04 contract and
holds across all 28 churn/configuration cells.

| configuration | churn | eligible | returned | recall@10 | p50 µs | p95 µs |
|---|---:|---:|---:|---:|---:|---:|
| none | 0% | 4097 | 10 | 1.000 | 383 | 469 |
| none | 10% | 3688 | 10 | 1.000 | 29184 | 29427 |
| none | 50% | 2049 | 10 | 1.000 | 16543 | 17020 |
| none | 90% | 410 | 10 | 1.000 | 3528 | 3681 |
| f16 | 0% | 4097 | 10 | 1.000 | 188 | 242 |
| f16 | 10% | 3688 | 10 | 1.000 | 28914 | 29200 |
| f16 | 50% | 2049 | 10 | 1.000 | 16489 | 27598 |
| f16 | 90% | 410 | 10 | 1.000 | 3849 | 4031 |
| sq8 | 0% | 4097 | 10 | 0.300 | 185 | 242 |
| sq8 | 10% | 3688 | 10 | 1.000 | 29145 | 29344 |
| sq8 | 50% | 2049 | 10 | 1.000 | 16542 | 17025 |
| sq8 | 90% | 410 | 10 | 1.000 | 3565 | 4180 |
| sq8c | 0% | 4097 | 10 | 0.400 | 705 | 865 |
| sq8c | 10% | 3688 | 10 | 1.000 | 28883 | 29287 |
| sq8c | 50% | 2049 | 10 | 1.000 | 16275 | 16473 |
| sq8c | 90% | 410 | 10 | 1.000 | 3647 | 4090 |
| bq | 0% | 4097 | 10 | 0.100 | 184 | 240 |
| bq | 10% | 3688 | 10 | 0.000 | 124 | 190 |
| bq | 50% | 2049 | 10 | 1.000 | 16050 | 17333 |
| bq | 90% | 410 | 10 | 1.000 | 3730 | 3854 |
| sq8-rerank | 0% | 4097 | 10 | 1.000 | 576 | 776 |
| sq8-rerank | 10% | 3688 | 10 | 1.000 | 38443 | 39806 |
| sq8-rerank | 50% | 2049 | 10 | 1.000 | 26142 | 26937 |
| sq8-rerank | 90% | 410 | 10 | 1.000 | 14037 | 14109 |
| bq-rerank | 0% | 4097 | 10 | 0.000 | 733 | 952 |
| bq-rerank | 10% | 3688 | 10 | 0.000 | 1317 | 2535 |
| bq-rerank | 50% | 2049 | 10 | 0.000 | 387 | 956 |
| bq-rerank | 90% | 410 | 10 | 1.000 | 13440 | 13527 |

## 4. Findings and limits

The first release probe reproduced a correctness failure before the final
patch: a 4,097-slot lossless collection at 10% churn returned zero rows with
`ef_search=64`. The refill loop was increasing the requested result count while
leaving HNSW's exploration budget fixed. The Wave C fix now uses
`max(ef_search, requested_limit)` and invokes the exact oracle if all slots have
been explored without finding `k` eligible nodes.

The eligibility contract is green. Recall is a separate quality axis: this
synthetic corpus collapses many vectors into the same BQ code, so BQ and BQ
rerank can have low recall even though they return the requested number of live
rows. That result is evidence for a Wave D quality envelope, not a reason to
claim exact ranking for BQ.

## 5. Outcome

R-04's filtered-ANN shortfall gate is verified across the full churn matrix.
The remaining Wave C evidence gates are final cross-feature regression, mobile/
FFI host checks and documentation/version recording. Wave D should set explicit
recall and latency limits per quantizer rather than treating the eligibility
contract as a recall guarantee.

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-08 | beta | Record 28-cell filtered-ANN oracle and latency matrix | d8ef9af | ATHER |
