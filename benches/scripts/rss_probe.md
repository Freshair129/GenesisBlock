# RSS probe (resident memory at 500k–2M nodes)

Runbook for quantifying the **Node RAM ceiling** — the other half of MARK XIV P1.
Not run in CI (needs minutes–hours and several GB of RAM). Run locally to produce the
numbers the audit still lists as pending.

> Pass `--features bins` on every command (the bins are off by default). On
> Windows/PowerShell use `$env:NAME = "..."` and ignore the `Load Node-API … failed`
> stderr lines.

## What we measure

Resident set size (RSS) of the engine after ingesting N nodes, swept across scales,
for the relevant memory configurations:

- baseline `Quant::None` (f32 arena)
- `Quant::ScalarU8` (SQ8 — ~4× smaller vector RAM)
- `Quant::Binary` (BQ — ~32× smaller vector RAM)
- SQ8/BQ **+ rerank** (PR #21) — adds an f32 sidecar, so RSS rises back toward the
  f32 baseline; this probe is what quantifies the rerank RAM cost.

The engine already exposes RSS via `/v1/status` (`memory_usage_mb`) and the NAPI
status surface, so the probe just ingests then reads it.

## Procedure

1. Ingest N synthetic vectors into a collection of the chosen `quant` (+ `rerank`),
   flushing the index so everything is resident:

```bash
export GB_VBENCH=/path/to/gb_vbench_500k    # reuse the recall corpus
export GB_RSS_SWEEP="500000,1000000,2000000" # scales to probe
export GB_QUANT="none"                       # none | sq8 | bq
export GB_RERANK="0"                         # 1 to add the f32 sidecar
cargo run --release --features bins --bin vbench-genesis   # ingest + flush_index
```

2. Read RSS after `flush_index()` (steady state):

```bash
curl -s localhost:3000/v1/status | jq '.memory_usage_mb'
# or, headless, print process RSS via sysinfo at the end of the ingest bin
```

3. Repeat per `GB_QUANT` × `GB_RERANK` × scale and tabulate
   `{ n, quant, rerank, rss_mb }`.

## Expected shape (hypothesis to confirm)

- `none` RSS grows ~linearly with N × dim × 4 bytes (+ HNSW graph + metadata).
- `sq8` ≈ ¼ of the vector arena; `bq` ≈ 1/32.
- `+rerank` ≈ quantized arena **plus** a full f32 sidecar (≈ the `none` vector cost
  on top of the quantized index) — the probe pins this trade exactly.

## Notes

- `vbench-genesis` currently drives the legacy single-space path; a collection-aware
  ingest (to exercise `quant`/`rerank`) may need a small driver tweak — track as a
  follow-up alongside the recall-on-real-data harness.
- Compare against the P31 baseline (12.6 GB RSS at 1M nodes pre-interning) to show the
  A1/A2/A3 interning + quantization gains.
