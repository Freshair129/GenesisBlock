---
doc_id: AUDIT--WAVE-D-QUALITY-2026-09-08
version: "0.1.0b"
created_at: "2026-09-08T20:10:00+07:00,ATHER,Wave-D"
last_update: "2026-09-08T20:10:00+07:00,ATHER,Wave-D"
status: beta
superseded_by: null
attributes:
  domain: query-budgets-quality
  scope: wave-d-r11
  artifact: docs/AUDIT--WAVE-D-QUALITY-2026-09-08.json
---

# Wave D per-index quality audit

This is a local evidence artifact for the approved Wave D quality gate. It is
not a production or release claim. The machine-readable rows are in
[`AUDIT--WAVE-D-QUALITY-2026-09-08.json`](AUDIT--WAVE-D-QUALITY-2026-09-08.json).

## Run envelope

- Harness: `wave-d-quality-gate`
- Dataset: deterministic synthetic unit vectors, `N=64`, `dim=8`, `Q=8`,
  `k=3`, seed `4242`
- Metric: cosine
- Index matrix: `none`, `f16`, `sq8`, `sq8+rerank`, `bq`, `bq+rerank`
- `ef_search`: `32`, `100`
- Live-visibility churn: `0%`, `10%`, `50%`, `90%`
- Each row records exact filtered recall, eligibility shortfall, result equality,
  p50/p95/p99 latency, flush lag, checkpoint/reopen time, and arena/sidecar
  bytes. The harness also reopens every database before recording residency.

Command used:

```powershell
$env:GB_WAVE_D_OUT = '.brain/audit/wave-d-run'
$env:GB_WAVE_D_N = '64'
$env:GB_WAVE_D_Q = '8'
$env:GB_WAVE_D_DIM = '8'
$env:GB_WAVE_D_K = '3'
cargo run --release --no-default-features --features bins --bin wave-d-quality-gate
```

## Findings

The harness emitted 48 configuration rows. Every row had zero eligibility
shortfall, so the Wave C filtered-ANN contract held under this churn matrix.
The quality envelope still has six failing rows; they are visible individually
in the JSON artifact and are not hidden by an aggregate median:

| Index configuration | Churn | Recall | Declared floor |
|---|---:|---:|---:|
| none / ef=100 | 90% | 0.7500 | 0.95 |
| f16 / ef=32 | 90% | 0.8333 | 0.90 |
| sq8 + rerank / ef=100 | 90% | 0.7500 | 0.90 |
| bq + rerank / ef=32 | 50% | 0.7500 | 0.80 |
| bq + rerank / ef=100 | 50% | 0.7500 | 0.80 |
| bq + rerank / ef=100 | 90% | 0.7500 | 0.80 |

These rows are a quality-envelope signal for high tombstone churn on a small
synthetic corpus. No HNSW default was tuned from this run. The artifact keeps
the observed recall, tail latency, checkpoint/reopen timings, and memory
measurements available for the next corpus-sized audit.

## Gate status

The artifact-level `pass` is `false` because six declared per-index floors did
not pass. Wave D therefore records retrieval evidence and remains closed for a
consumer-facing recall claim until a larger, representative corpus and a
reviewed envelope address those rows.

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-08 | beta | 48-row per-index quality envelope with exact filtered oracle and churn matrix | pending | ATHER |
