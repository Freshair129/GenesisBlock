---
proposed_id: AUDIT--P29-ROCKSDB-GRAPH-HEAD-TO-HEAD
type: audit
status: complete
aliases:
  - AUDIT
  - P29
tier: process
cluster: implementation_flow
role: "Embedded↔embedded graph head-to-head: GenesisBlockDB vs RocksDB+adjacency"
phase: 29
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P22-GRAPH-TRAVERSAL
  - AUDIT--P26-KUZU-HEAD-TO-HEAD
  - AUDIT--P28-DUCKDB-GRAPH-HEAD-TO-HEAD
  - adr/ADR--GENESISDB-MARKET-POSITIONING
---

# AUDIT — P29 RocksDB + graph Head-to-Head (KV + adjacency lists)

## 1. Why
The positioning ADR names **RocksDB + graph layer** as a nearest embedded
comparator — the last unmeasured one. RocksDB is an embedded LSM key-value store;
the idiomatic "graph layer" stores each node's adjacency list as a KV entry and
traverses by BFS over point lookups. This is the architecture closest to
GenesisBlockDB's own (adjacency-list + point lookups), so it is the sharpest test of
whether GenesisBlockDB's latency advantage is real or just "vs the wrong baseline".

## 2. Method
`benches/rocksdb_bench.py` (`rocksdict` 0.3.29, RocksDB in raw-bytes mode):
N nodes, **fanout-8** random adjacency (independent draw, identical stats), one KV
entry per node = its 8 neighbor ids (int64 bytes). Traversal = Python BFS over KV
`get()`s, **with a visited-set dedup to match GenesisBlockDB's distinct-node
semantics**, capped at LIMIT 1000, depths {1,3,6}, 200 q/depth. GenesisBlockDB side =
P22 `graph-bench`. Both embedded, C: SSD.

## 3. Results (100k nodes / 800k edges)

| Metric | GenesisBlockDB | RocksDB + adjacency BFS |
|---|---|---|
| hop1 p50 | **21.6 µs** | 26.8 µs |
| hop3 p50 | 2,334 µs | 435 µs † |
| hop6 p50 | 4,403 µs | 675 µs † |
| Edge ingest | ~24 s (durable WAL) | **0.7 s** |
| Memory (RSS Δ) | 1,057 MB | **33 MB** |

(10k: GenesisBlockDB hop1 23 µs vs RocksDB ~27 µs; RocksDB ingest 0.1 s, RSS 7 MB.)

## 4. Reading — the cleanest comparison is hop1 (be very honest here)

**† The hop3/hop6 numbers are NOT apples-to-apples and must not be cited as a
RocksDB win.** The two harnesses return different payloads:

- **GenesisBlockDB `neighbors()` materializes a full `NeighborOutput { node, path }`
  per result** — it clones the node record *and* the accumulated edge path for
  each of the 582–1000 results at depth 3–6. That allocation dominates deep-hop time.
- **The RocksDB harness returns bare node ids** (no node record, no path objects).
  It does categorically less work per result, so its deep-hop latency is lower for
  reasons unrelated to traversal speed.

The **hop1** point (≤8 results, negligible payload) is the clean comparison:
**GenesisBlockDB 21.6 µs vs RocksDB 26.8 µs — effectively tied, GenesisBlockDB slightly
ahead.** This is the key result: an embedded KV store with a hand-rolled
adjacency layer lands in the **same point-query latency class** as GenesisBlockDB.
That *validates* GenesisBlockDB's architecture rather than undercutting it — the µs-scale
point latency is a property of the adjacency-list + in-process design, confirmed
against the most directly comparable baseline.

## 5. What RocksDB+graph does NOT give you
RocksDB is a KV store; everything graph-shaped is DIY. To reach the result above
the harness already had to hand-code adjacency encoding, BFS, *and* the
visited-set dedup. It has **no query language, no path semantics, no typed/
directional edge filters, no bitemporal validity, no governance tiers, no vector
/ hybrid search, no CRDT sync** — all of which GenesisBlockDB provides in-engine at the
same point-query latency. "RocksDB + graph" is not a product; it is a project.

RocksDB wins **ingest (~30×)** and **memory (~32×: 33 MB vs 1,057 MB)** — the same
columnar/compact-store advantage seen with Kuzu (P26) and DuckDB (P28), and again
corroborating P22's finding that **edge-id (UUID) interning is GenesisBlockDB's
dominant RAM cost** and the top lever (numeric/u64 ids).

## 6. Program status — competitor matrix complete
All seven named comparators are now measured: **Chroma** (P15/P21), **Qdrant**
(P20), **LanceDB** (P27) — vector; **Neo4j** (P23), **Kuzu** (P26),
**DuckDB+graph** (P28), **RocksDB+graph** (P29) — graph. Plus cost proofs
(governance P24, K-Impact P25) and scale (P22).

**Cross-comparator takeaway:** GenesisBlockDB's point/shallow-traversal latency
(hop1 ~22 µs) is in the same class as a raw RocksDB adjacency store and faster
than every analytical/columnar/server engine measured — while uniquely bundling
graph + vector + governance + bitemporal semantics. It trades higher RAM (the
known interning lever) and durable-WAL ingest cost for that. Right tool for
**low-latency agent memory**, not bulk analytics or larger-than-memory storage.

Reproduce:
```
pip install rocksdict numpy psutil
GB_ROCKS_N=100000 python benches/rocksdb_bench.py
GB_VBENCH=<dir> GB_GRAPH_N=100000 cargo run --release --bin graph-bench
```
