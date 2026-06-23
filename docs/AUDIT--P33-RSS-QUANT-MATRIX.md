---
proposed_id: AUDIT--P33-RSS-QUANT-MATRIX
type: audit
status: partial
aliases:
  - AUDIT
  - P33
tier: process
cluster: implementation_flow
role: "RSS + on-disk footprint x quant x scale matrix at 100k-1M — engine RAM/disk ceiling evidence + WAL-blowup finding (MARK XV P1)"
phase: 33
audited_at: 2026-06-23
proposed_by: agent
related:
  - AUDIT--P32-RECALL-500K-FRONTIER
  - AUDIT--P31-POST-MARKXIII-REGRESSION
  - adr/ADR--GENESISDB-VECTOR-QUANTIZATION
  - adr/ADR--GENESISDB-NODE-ID-INTERNING
---

# AUDIT — P33 RSS × Quant × Scale Matrix

## 1. Why

[P32](AUDIT--P32-RECALL-500K-FRONTIER.md) measured the recall half of the P1 scale
ceiling. This is the **RAM half**: resident set size (RSS) of the engine across
`{none, sq8, bq} × {rerank 0/1} × {100k, 250k, 500k}` nodes, to (a) quantify the
node-id interning + quantization RAM gains the prior audits listed as pending, and
(b) test the ROADMAP P4 claim that BQ gives "~32×" RAM savings.

## 2. Method

- **Corpus:** synthetic-clustered, dim 1024 (same generator as P32, seed 42).
  - 100k–500k: prefixes of the 500k set `gb_vbench_500k/` via the `GB_LIMIT` knob.
  - 1M: a regenerated `gb_vbench_1m/` (4.096 GB, streamed gen, no ground truth —
    recall not scored here).
- **Engine:** `benches/vbench_genesis.rs`, extended for this audit
  ([runbook](../benches/scripts/rss_probe.md), runner
  [`benches/scripts/rss_matrix.ps1`](../benches/scripts/rss_matrix.ps1)). Four harness
  defects were fixed first (see §5): the corpus is now **streamed from disk** (not held
  resident — essential at 1M, where the old 8 GB upfront load OOM'd a 32 GB box),
  `flush_index()` drains the async HNSW backlog before measuring, quantized configs
  route through `create_collection(quant, rerank)`, and disk is measured **excluding
  the WAL** to isolate the persisted snapshot.
- **Config:** `efc = 200`, `ef_search` = default, `k = 10`, `q = 200`. RSS via `sysinfo`
  process memory after ingest + flush; disk via `gdb` dir size. Build: main `1c030b9`
  + the harness extension (release). Box: 32 GB RAM, ~34 GB free disk at start.
- **Scope caveat:** `GB_LIMIT` < n is valid for **RSS/latency/disk** only; recall is not
  scored here (the full-corpus ground truth only matches at `GB_LIMIT == n`).

## 3. Result

Raw: `gb_vbench_500k/rss_matrix.txt` (100k–500k), `gb_vbench_1m/rss_disk_matrix_1m.txt`
(1M RSS+disk), `disk_snapshot_100k.txt` (snapshot vs WAL split).

### RSS (MB) by scale

| quant | rerank | 100k | 250k | 500k | **1M** | ×f32 RAM @1M |
|-------|:------:|-----:|-----:|-----:|-------:|:------------:|
| none  | –      | 1158 | 2887 | 5745 | 11408  | 1.00×        |
| sq8   | no     |  589 | 1448 | 2823 |  5534  | **2.06× smaller** |
| sq8   | yes    |  984 | 2429 | 4753 |  9440  | 1.21× smaller |
| bq    | no     |  466 | 1084 | 1981 |  3898  | **2.93× smaller** |
| bq    | yes    |  838 | 2041 | 3976 |  7773  | 1.47× smaller |

### Latency + ingest (@500k / @1M)

| quant | rerank | p50 µs (500k/1M) | p95 µs (500k/1M) | insert/s (500k/1M) |
|-------|:------:|:----------------:|:----------------:|:------------------:|
| none  | –      | 1104 / 1146      | 2102 / 2310      | 2236 / 1897        |
| sq8   | no     |  983 / 1188      | 2065 / 2500      | 2306 / 2154        |
| sq8   | yes    | 1228 / 1381      | 2060 / 2469      | 2316 / 1977        |
| bq    | no     | **416 / 482**    | **747 / 970**    | **2966 / 2624**    |
| bq    | yes    |  511 / 598       |  834 / 1197      | 2935 / 2647        |

### On-disk footprint (@100k, snapshot isolated from WAL)

| quant | rerank | snapshot (MB) | vector-arena (calc) | arena ×f32 | WAL (MB) |
|-------|:------:|--------------:|--------------------:|:----------:|---------:|
| none  | –      | 419.3         | 409.6               | 1.00×      | 2092     |
| sq8   | no     | 126.1         | 102.4               | **4.0×**   | 2092     |
| bq    | no     |  40.7         |  12.8               | **32×**    | 2092     |
| sq8   | yes    | 516.7         | 102.4 + 409.6 f32 sidecar | —    | 2092     |
| bq    | yes    | 431.3         |  12.8 + 409.6 f32 sidecar | —    | 2092     |

The WAL (`genesis-graph.wal`) is **quant-independent** (2092 MB for every config) and
scales linearly — measured **~20.4 GB at 1M**, which drove free disk to **2.1 GB** and
starved the snapshot save.

## 4. Findings

- **Corrected f32 baseline: 5.75 GB @ 500k**, not the 7.69 GB of the first probe run.
  The ~1.94 GB delta was the harness's own resident `corpus.f32` buffer (§5); freeing
  it before the RSS read removes the confound. *Every prior RSS figure from this bin
  was inflated by ~2 GB.*
- **The per-vector quant ratios (4× SQ8, ~32× BQ) do NOT hold end-to-end.** Measured
  total-RSS savings are only **2.04× (SQ8)** and **2.90× (BQ)**. Quantization shrinks
  only the vector arena (+ hnsw_rs's internal copy); a **~1.8 GB non-vector floor**
  (HNSW graph links + node/edge metadata + interning maps + page cache) is untouched
  and dominates once the vectors shrink. **The graph, not the vectors, is now the RAM
  frontier** — directly relevant to ROADMAP P4 (the "~32×" note describes arena bytes,
  not process RSS).
- **BQ is also the latency + ingest winner:** ~2.6× faster p50 (416 vs 1104 µs),
  ~2.8× lower p95, ~33% faster ingest — Hamming popcount on u64 words beats f32/u8 L2.
- **Rerank costs ≈ a full f32 vector arena.** The f32 sidecar adds ~1.9 GB @500k
  (sq8: 2823→4753; bq: 1981→3976), roughly halving BQ's RAM edge (2.90× → 1.45×).
  Rerank is recall insurance paid in RAM.
- **The 1M run confirms the extrapolation precisely.** Predicted (from 500k) none ≈ 11.2,
  sq8 ≈ 5.6, bq ≈ 3.9 GB; **measured 11.4 / 5.5 / 3.9 GB.** Quant ratios are stable across
  scale (500k: 2.04×/2.90× → 1M: 2.06×/2.93×). The non-vector floor scales ~linearly
  (~1.8 GB @500k → ~3.6 GB @1M ≈ 3.6 KB/node). Against the
  [P31](AUDIT--P31-POST-MARKXIII-REGRESSION.md) 12.6 GB @1M *pre*-interning, f32 now =
  **11.4 GB ≈ 9.5% lower** — modest at 1024-dim because vectors+graph dominate the
  node-id bookkeeping interning shrank. **Quant is a far larger RAM lever than interning
  at high dim. And f32 @1M *fits* a 32 GB box** (no OOM once the corpus streams).
- **On disk, the per-vector 4×/32× claim IS exact — for the arena bin.** The persisted
  `vec_<name>.bin` is 409.6 → 102.4 (**4.0×**, SQ8) → 12.8 MB (**32×**, BQ) at 100k, exactly
  as `ADR--GENESISDB-VECTOR-QUANTIZATION` specifies. Total *snapshot* ratios are lower
  (3.3× SQ8, 10.3× BQ) because node metadata + `state.json` don't shrink.
- **Rerank costs disk too — a quant+rerank snapshot is *larger* than plain f32.** The
  f32 sidecar (`fvec_<name>.bin`, ~409.6 MB @100k) is added on top of the quantized arena,
  so sq8+rerank (516.7 MB) and bq+rerank (431.3 MB) both exceed f32-none (419.3 MB). Rerank
  trades disk *and* RAM for recall; it does not save space.
- **⚠ The WAL is the real disk ceiling, and it is unbounded.** `genesis-graph.wal` is
  quant-independent (stores raw input embeddings) and **never compacted** in this path:
  ~2.1 GB @100k, **~20.4 GB @1M** — 5× the f32 snapshot, 51× the BQ snapshot. At 1M it drove
  free disk to **2.1 GB** and starved the snapshot save (no `vec_*.bin` written). *Quant's
  disk savings are invisible until the WAL is compacted/rotated.* This is a production
  durability/ops concern, not a quant issue.

## 5. Harness defects fixed (so the numbers are trustworthy)

`benches/vbench_genesis.rs` before this audit:
1. **Loaded the entire `corpus.f32` into RAM up front** (`fs::read` → 4 GB `Vec<u8>`,
   then `.collect()` → 4 GB `Vec<f32>` = ~8 GB at 1M) and held it through the RSS read.
   This both **OOM'd 1M** on a 32 GB box and **inflated every RSS number** by the corpus
   size (~2 GB @500k). Now the corpus is **streamed from disk** one 10k-row chunk at a
   time → 1M fits, and the RSS read is clean by construction.
2. **Never called `flush_index()`** → the async HNSW backlog could be unindexed at
   measure time, *undercounting* the graph (and risking missed-recent-insert queries).
   Now flushed before RSS + queries.
3. **Drove only the legacy f32 single-space path** (`collection: None`) → could not
   measure quant/rerank at all. Now routes through `create_collection(quant, rerank)`
   when `GB_QUANT != none`; added `GB_QUANT`/`GB_RERANK`/`GB_LIMIT` env knobs.
4. **Disk measurement counted the WAL**, which dwarfs and masks the quant snapshot.
   The runner now sums the snapshot (`.bin` + `state.json`) and the WAL separately.

## 6. Open / Follow-ups

- **2M scale** — not run; needs an ~8 GB corpus and, more importantly, the WAL would
  hit ~40 GB and exceed free disk on this box. Blocked on WAL compaction (below) or a
  bigger disk. 1M is the verified ceiling here.
- **⚠ WAL compaction / rotation** — the headline ops finding. At 1M the uncompacted WAL
  is ~20 GB and starves the snapshot save. This belongs on the roadmap as a production
  hardening item (MARK XV P3), ahead of any further vector/on-disk work.
- **Recall is NOT measured here.** BQ's RAM/latency/disk wins are meaningless until its
  recall is measured on real embeddings — same-sign/different-magnitude vectors
  collapse under BQ without rerank. This is the remaining P1 half
  ([recall harness runbook](../benches/scripts/recall_harness.md)).
- **The non-vector RAM floor (~3.6 GB @1M) is the next RAM target** — graph-link and
  metadata compaction would beat further vector quantization. MARK XV P4 reframed
  accordingly.
