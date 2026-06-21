---
proposed_id: AUDIT--P28-DUCKDB-GRAPH-HEAD-TO-HEAD
type: audit
status: complete
aliases:
  - AUDIT
  - P28
tier: process
cluster: implementation_flow
role: "Embedded↔embedded graph head-to-head: GenesisBlockDB vs DuckDB (recursive CTE)"
phase: 28
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P22-GRAPH-TRAVERSAL
  - AUDIT--P26-KUZU-HEAD-TO-HEAD
  - adr/ADR--GENESISDB-MARKET-POSITIONING
---

# AUDIT — P28 DuckDB + graph Head-to-Head (recursive CTE)

## 1. Why
The positioning ADR names **DuckDB + graph layer** as a nearest embedded
comparator but it had never been measured. DuckDB is an embedded columnar
analytical engine; the idiomatic "graph" is an edges table traversed by a
**recursive CTE**. This measures that head-to-head on the same topology as the
Kuzu (P26) and GenesisBlockDB (P22) graph runs.

## 2. Method
`benches/duckdb_bench.py` (mirrors `kuzu_bench.py`): N nodes, **fanout-8** random
edges (independent draw, identical stats), depths {1,3,6}, 200 q/depth, LIMIT
1000. DuckDB 1.5.4 in-process; edges bulk-loaded via Arrow, **ART index on
`from_id`**; traversal = `WITH RECURSIVE reach(node,depth) … a-[*1..d]->b LIMIT
1000`, parametrized. GenesisBlockDB side = P22 `graph-bench` (`graph_results_{N}.json`).
Both embedded, C: SSD.

## 3. Results (100k nodes / 800k edges)

| Metric | GenesisBlockDB | DuckDB (recursive CTE) | Kuzu (P26) |
|---|---|---|---|
| hop1 p50 | **21.6 µs** | 1,169.8 µs | 3,653 µs |
| hop3 p50 | **2,334 µs** | 3,216 µs | 15,705 µs |
| hop6 p50 | 4,403 µs | 4,669 µs | 60,914 µs |
| Edge ingest | ~24 s (durable WAL) | **0.7 s** (+index) | 0.4 s (COPY) |
| Memory (RSS Δ) | 1,057 MB | **97 MB** | 97 MB |

(10k: GenesisBlockDB hop1 23 µs vs DuckDB 969 µs; DuckDB ingest 0.7 s, RSS 67 MB.)

## 4. Reading — gap narrows with depth (honest)

- **GenesisBlockDB wins point/shallow traversal decisively — hop1 ~54×** (21.6 µs vs
  1.17 ms). Adjacency-list + direct-call `neighbors()` is built for low-latency
  point lookups; DuckDB pays per-query plan + recursive-join setup even for one hop.
- **The gap shrinks as depth grows: hop3 ~1.4×, hop6 ~1.06× (effectively tied).**
  At depth 6 the workload becomes large-neighborhood expansion, where DuckDB's
  vectorized, set-based recursive join is in its element — it closes almost the
  entire gap. This is the most competitive deep-traversal comparator measured so far.
- **DuckDB beats Kuzu at every depth** (≈3× hop1, ≈5× hop3, ≈13× hop6): its
  recursive-CTE engine is more efficient here than Kuzu's recursive Cypher.
- **DuckDB wins ingest (~35×) and memory (~11×)** vs GenesisBlockDB — same columnar
  advantage as Kuzu, and it corroborates P22/P26: edge-id (UUID) interning is
  GenesisBlockDB's dominant RAM cost (top lever: numeric/u64 ids).

**Takeaway:** GenesisBlockDB is the right tool for **low-latency point/shallow agent-
memory graph queries** (µs-scale hop1, the dominant access pattern). For
**deep set-reachability over a static graph at low memory**, DuckDB's recursive
CTE is a strong embedded option and nearly ties at depth 6. Different sweet spots.

## 5. Caveats
- GenesisBlockDB edge-ingest time is the durable-WAL figure from P22/P26 (~24 s);
  DuckDB/Kuzu ingest is non-durable bulk load — not apples-to-apples on
  durability, only on time-to-queryable.
- Recursive CTE returns reachable nodes (LIMIT 1000), matching the
  variable-length `*1..d` semantics; both engines cap at 1000 results.
- DuckDB run in-memory (`duckdb.connect()`); on-disk mode would add persistence
  at some latency cost.

## 6. Program status
Graph comparators measured: **Neo4j** (server, P23), **Kuzu** (embedded, P26),
**DuckDB+graph** (embedded, P28). Vector: Chroma (P15/P21), Qdrant (P20), LanceDB
(P27). Still pending: **RocksDB+graph** (P29).

Reproduce:
```
pip install duckdb pyarrow psutil
GB_DUCK_N=100000 python benches/duckdb_bench.py
GB_VBENCH=<dir> GB_GRAPH_N=100000 cargo run --release --bin graph-bench
```
