---
proposed_id: AUDIT--P21-RECALL-LATENCY-FRONTIER
type: audit
status: complete
aliases:
  - AUDIT
  - P21
tier: process
cluster: implementation_flow
role: "Recall–latency frontier (ef_search sweep) vs Chroma & Qdrant"
phase: 21
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P20-QDRANT-3WAY-AND-EF-CONFIG
---

# AUDIT — P21 Recall–Latency Frontier

## 1. Why

Single ef points (P19/P20) can't show an engine's true position. A
recall-vs-latency curve does: build the index once, sweep query-time `ef_search`,
and plot every (latency, recall) point against competitors' points.

## 2. Method

`vbench_genesis` gained a sweep mode (`GB_EF_SWEEP`): build once at
`ef_construction=200`, then for each `ef_search` run the 200-query set and record
latency + top-k; `vbench.py frontier` computes recall per point vs exact L2
ground truth. 100k synthetic vectors, dim 1024, C: SSD. Chroma & Qdrant appear
as single reference points (their 100k results).

## 3. Result (100k, ef_construction=200)

| ef_search | p50 (µs) | p95 (µs) | recall@10 |
|---|---|---|---|
| 16  | 559.8  | 879.5  | 0.859 |
| 32  | 653.6  | 1050.7 | 0.913 |
| 64  | 812.2  | 1394.6 | 0.964 |
| 128 | 1097.4 | 2145.5 | 0.984 |
| 256 | 1255.0 | 1874.6 | 0.988 |
| 512 | 2119.4 | 3069.8 | 0.990 |

Reference points: **Chroma** 990 µs / 0.981 · **Qdrant** 3301 µs / 0.999.

## 4. Reading

- **GenesisDB's frontier passes through Chroma's point.** ef_search=128 →
  recall 0.984 (> Chroma's 0.981) at ~1.1 ms; ef_search=64 → 0.964 at 0.81 ms
  (faster than Chroma). They occupy essentially the same recall↔latency frontier.
- **Qdrant** trades latency for recall: 0.999 but 3.3 ms (localhost gRPC). On the
  curve it's the high-recall/high-latency corner; GenesisDB reaches 0.990 at
  2.1 ms (faster) for marginally less recall.
- `ef_search` is a **live knob** (`set_index_params`) — a deployment selects any
  point on this curve without rebuilding.

## 5. Artifact

Interactive scatter (log-x latency, y recall) in `perf-comparison-dashboard.html`.

Reproduce:
```
GB_VBENCH=<dir> GB_EF=200 GB_EF_SWEEP=16,32,64,128,256,512 cargo run --release --bin vbench-genesis
python benches/vbench.py frontier
```
