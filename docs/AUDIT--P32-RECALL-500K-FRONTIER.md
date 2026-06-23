---
proposed_id: AUDIT--P32-RECALL-500K-FRONTIER
type: audit
status: complete
aliases:
  - AUDIT
  - P32
tier: process
cluster: implementation_flow
role: "Recall@500k vs ef_search frontier — scale-ceiling evidence for per-collection ef_search"
phase: 32
audited_at: 2026-06-23
proposed_by: agent
related:
  - AUDIT--P31-POST-MARKXIII-REGRESSION
  - adr/ADR--GENESISDB-ASYNC-INDEXING
  - adr/ADR--GENESISDB-MULTI-COLLECTION
---

# AUDIT — P32 Recall@500k vs `ef_search` Frontier

## 1. Why

MARK XIV P1 ("Scale Ceiling") flagged that the engine's single **global** `ef_search`
default, tuned at 100k vectors (recall@10 ≈ 0.982), appeared to fall to ≈ 0.89 at
500k. A single global value cannot serve both scales. This audit measures the
recall–latency frontier at 500k to quantify the gap and decide the fix.

## 2. Method

- **Corpus:** a separate 500k synthetic-clustered set generated at
  `gb_vbench_500k/` (`gen.py` mirrors `benches/vbench.py do_synth`, rng seed 42;
  ~2.05 GB `corpus.f32` + exact-L2 ground truth). The existing 100k set is
  untouched. Runbook: [`benches/scripts/recall_harness.md`](../benches/scripts/recall_harness.md).
- **Engine:** `vbench-genesis` over the legacy single-space `Storage::open(vector_dim)`
  path (no quantization) — measures **baseline-engine recall vs `ef_search`**, not
  a quantized configuration.
- **Sweep:** `GB_EF_SWEEP="50,100,200,400,800"`, `efc = 200`, `k = 10`, `q = 200`,
  `dim = 1024`, `n = 500_000`.
- **Scoring:** standalone `gb_vbench_500k/score.py` (mirrors `recall_at_k`) →
  `frontier_results.json`.

## 3. Result

n = 500k, dim 1024, k = 10, q = 200, efc = 200:

| `ef_search` | recall@10 | p50 (µs) | p95 (µs) |
|------------:|----------:|---------:|---------:|
| 50          | 0.7895    | 920      | 1549     |
| 100         | 0.8350    | 1169     | 2229     |
| **200**     | **0.8870**| 1458     | 2991     |
| 400         | 0.9405    | 1892     | 4148     |
| **800**     | **0.9730**| 4528     | 7065     |

(Raw numbers: `gb_vbench_500k/frontier_results.json`.)

## 4. Findings

- **The 500k regression reproduces at the global default.** `ef = 200` yields
  recall@10 = **0.887** at 500k vs **0.982** at 100k for the same default — a ~0.095
  absolute drop purely from scale.
- **Recall is fully recoverable by raising `ef_search`.** `ef = 400 → 0.940`,
  `ef = 800 → 0.973`. The index is not lossy at scale; the default is simply too low.
- **But recovery costs latency.** Holding recall ≥ 0.97 needs `ef ≈ 800`, which is
  **~3.1× the p50** of `ef = 200` (1458 → 4528 µs) and ~2.4× p95.
- **Therefore a single global `ef` cannot serve both scales** — 100k wants a low ef
  (fast, already high recall), 500k wants a high ef (slower, to claw recall back).

## 5. Decision / Outcome

This is direct evidence for **per-query / per-collection `ef_search`** so callers
trade recall vs latency by workload instead of one global compromise:

- **Per-query `ef_search`** shipped earlier (`HybridSearchInput.ef_search`).
- **Per-collection default `ef_search`** shipped in MARK XIV PR #20
  (`create_collection(..., ef_search)`; resolution: per-query → per-collection →
  global). A 500k collection can default to `ef = 800` while a 100k collection keeps
  the fast default — no global compromise.

## 6. Open / Follow-ups

- **RSS at 500k–2M** (the other half of the P1 ceiling) is **not** measured here.
  Runbook prepared: [`benches/scripts/rss_probe.md`](../benches/scripts/rss_probe.md).
- **SQ8 / BQ recall on real embeddings** (PR #21 added rerank): measure recall
  recovery on a real corpus, not just this synthetic baseline. See the recall-harness
  runbook.
- Numbers here are the **baseline engine** (no quant); a quantized + rerank frontier
  is a separate sweep.
