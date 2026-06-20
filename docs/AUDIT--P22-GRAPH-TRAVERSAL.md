---
proposed_id: AUDIT--P22-GRAPH-TRAVERSAL
type: audit
status: complete
aliases:
  - AUDIT
  - P22
tier: process
cluster: implementation_flow
role: "Graph traversal benchmark (LDBC-lite) 10k/100k/1M"
phase: 22
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P21-RECALL-LATENCY-FRONTIER
  - DESIGN--HNSW-HYBRID-INDEX
---

# AUDIT — P22 Graph Traversal Benchmark (LDBC-lite)

## 1. Why

The vector path is now well-evidenced; the **graph** path had no heavy benchmark
even though the whitepaper claims graph-performance superiority. This is the
first measured k-hop traversal benchmark across scale. Positioning note: the
honest comparator class is **embedded analytics / agent-memory graph engines
(Kuzu, DuckDB+graph, RocksDB+graph layer)**, not distributed enterprise graph DBs.

## 2. Method

`benches/graph_bench.rs` (`[[bin]] graph-bench`): directed random graph, N nodes,
`fanout=8` out-edges each (≈8·N edges), no embeddings. Ingest via batched
bulk paths (durable WAL). Then 200 traversals/seed-depth at depths {1,3,6} via
`Storage::neighbors`, recording p50/p95/p99 latency + throughput + visited count.
Bounded by `limit=1000` per traversal. C: SSD.

**Engine fix shipped with this benchmark:** `neighbors` previously **ignored
`NeighborInput.limit`** — depth-6 on a hub graph would traverse the whole
component (OOM risk). Now honored (early-return at the cap).

## 3. Results

| N (nodes) | edges | hop1 p50 | hop3 p50 | hop6 p50 | hop1 thrpt | RSS | edge ingest |
|---|---|---|---|---|---|---|---|
| 10,000 | 80k | 23.1 µs | 1.97 ms | 4.21 ms | 41,525 /s | 146 MB | 2.2 s |
| 100,000 | 800k | 21.6 µs | 2.33 ms | 4.40 ms | 42,783 /s | 1.06 GB | 24.4 s |
| 1,000,000 | 8M | 35.4 µs | 4.58 ms | 9.29 ms | 27,898 /s | 12.6 GB | 281 s |

(p95/p99 and per-depth detail in `graph_results_<N>.json`. hop6 hits the
limit=1000 cap; hop3 visits ≈580 nodes = 8+64+512 as expected.)

## 4. Reading

- **hop1 is O(neighborhood), not O(N).** Across a 100× node increase (10k→1M)
  1-hop latency stays in the tens of µs (23→22→35 µs) at ~28–43k traversals/s.
  The slight rise at 1M is cache/memory pressure (12.6 GB working set), not
  algorithmic — the core property of an index-backed graph engine holds.
- **Multi-hop stays bounded** (ms-scale); ~2× slower at 1M from colder, larger
  structures, but local queries don't blow up with graph size.
- **RAM is the real ceiling.** 1M nodes / 8M edges = 12.6 GB (~1.5 KB per
  node+edge), dominated by interning 8M edge-UUID strings (×2 maps) +
  `EdgeOutput` + `out_idx`/`in_idx`. **10M nodes / 80M edges ≈ 120 GB →
  infeasible on this 32 GB host; 1M is the practical ceiling here.**
- **Ingest:** ~28k durable edges/s at 8M; edge-UUID interning is the cost.

## 5. Follow-ups surfaced

- **Edge-id interning is the top RAM lever** for graph scale: don't intern edge
  UUIDs into the string maps (use a numeric/u64 id, or skip the reverse map).
  Likely the single biggest cut toward larger graphs.
- `neighbors` also ignores `args.direction` (always out-edges) and `args.rels`
  — fix before claiming directional/typed traversal performance.

## 6. Status vs program

P22 done. Remaining for full graph credibility: **P23** Neo4j (and ideally Kuzu)
head-to-head on the same graph; **P24** governance guard on/off cost; **P25**
K-Impact full vs incremental recompute (prove O(V_affected + E_affected)).

Reproduce:
```
GB_VBENCH=<dir> GB_GRAPH_N=1000000 GB_GRAPH_FANOUT=8 cargo run --release --bin graph-bench
```
