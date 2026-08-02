---
proposed_id: AUDIT--P26-KUZU-HEAD-TO-HEAD
type: audit
status: historical
aliases:
  - AUDIT
  - P26
tier: process
cluster: implementation_flow
role: "Embedded↔embedded graph head-to-head: GenesisBlockDB vs Kuzu"
phase: 26
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P22-GRAPH-TRAVERSAL
  - AUDIT--P23-NEO4J-HEAD-TO-HEAD
  - adr/ADR--GENESISDB-MARKET-POSITIONING
---

# AUDIT — P26 Kuzu Head-to-Head (the fairest comparator)

## 1. Why
Per the reframed positioning, the fairest competitor is an **embedded** graph
engine. Kuzu (kuzu 0.11.3, C++ core, in-process) is exactly that — no
server/network tax, unlike Neo4j (P23). Same topology params as P22/P23.

## 2. Method
Kuzu via Python (`benches/kuzu_bench.py`): `COPY FROM` CSV bulk load (Kuzu's
idiomatic path), **prepared** Cypher for traversal (fair vs a compiled call),
depths {1,3,6}, 200 q/depth, LIMIT 1000. GenesisBlockDB = P22 `graph-bench`. Both
embedded, C: SSD, fanout-8 random graph (independent draws, identical stats).

## 3. Results (100k nodes / 800k edges)

| Metric | GenesisBlockDB | Kuzu | Winner |
|---|---|---|---|
| hop1 p50 | 22 µs | 3,653 µs | **GenesisBlockDB ~166×** |
| hop3 p50 | 2.33 ms | 15.71 ms | **GenesisBlockDB ~6.7×** |
| hop6 p50 | 4.40 ms | 60.91 ms | **GenesisBlockDB ~14×** |
| Edge ingest | 24.4 s (durable WAL) | 0.4 s (COPY) | **Kuzu ~60×** |
| Memory (RSS Δ) | 1,057 MB | 97 MB | **Kuzu ~11×** |

(10k: GenesisBlockDB hop1 23 µs vs Kuzu 2,424 µs; Kuzu ingest 0.1 s, RSS 88 MB.)

## 4. Reading — different sweet spots (honest)

- **GenesisBlockDB wins point/local traversal latency** by 7–166×. Its adjacency-list
  + direct-function `neighbors()` design is built for low-latency point/k-hop
  queries — the agent-memory access pattern.
- **Kuzu wins ingest (≈60×) and memory (≈11×).** It is a **columnar analytical
  graph engine**: `COPY` bulk-loads fast and its compressed columnar store is far
  leaner. Point-query latency is not its design target (per-query operator setup
  dominates a 1-hop), and depth-6 recursive joins are expensive.
- Both are embedded — this is a like-for-like architecture comparison.

**Takeaway:** GenesisBlockDB is the right tool for **low-latency agent memory**
(µs point/local graph queries + vectors + governance), Kuzu for **bulk graph
analytics**. Not "faster at everything" — faster where it's designed to be.

## 5. Corroborates a known optimization target
Kuzu's 11× lower memory directly validates AUDIT--P22's finding: GenesisBlockDB's
**edge-id (UUID) interning** is the dominant RAM cost and the top lever
(switch to numeric/u64 ids) to push graph scale past 1M / 32 GB.

## 6. Program status
Named-competitor matrix (measured): **Chroma & Qdrant** (vector, P15/P20/P21),
**LanceDB** (embedded vector, P27), **Neo4j** (server graph, P23), **Kuzu**
(embedded graph, P26). Plus cost proofs (governance P24, K-Impact P25) and scale
(P22). Still pending: **DuckDB+graph** (P28), **RocksDB+graph** (P29).

Reproduce:
```
pip install kuzu psutil
GB_KUZU_N=100000 python benches/kuzu_bench.py
GB_VBENCH=<dir> GB_GRAPH_N=100000 cargo run --release --bin graph-bench
```
