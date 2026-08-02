---
proposed_id: AUDIT--P30-LADYBUGDB-HEAD-TO-HEAD
type: audit
status: historical
aliases:
  - AUDIT
  - P30
tier: process
cluster: implementation_flow
role: "Embedded↔embedded graph head-to-head: GenesisBlockDB vs LadybugDB (Kuzu fork)"
phase: 30
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P22-GRAPH-TRAVERSAL
  - AUDIT--P26-KUZU-HEAD-TO-HEAD
  - AUDIT--P28-DUCKDB-GRAPH-HEAD-TO-HEAD
  - adr/ADR--GENESISDB-MARKET-POSITIONING
---

# AUDIT — P30 LadybugDB Head-to-Head (the most on-niche comparator)

## 1. Why
**LadybugDB** is the community **fork of Kuzu** (continues Kuzu's last release
after its main development stopped): property-graph, Cypher, columnar + CSR
adjacency (forward+backward), HNSW vector index, full-text search, ACID, embedded.
That feature set — embedded graph **+ vector**, aimed at agentic memory / hybrid
RAG — is exactly GenesisBlockDB's niche, making LadybugDB the single most
directly-comparable competitor. P26 measured Kuzu (the parent); this measures the
**live fork** that an agent-memory builder would actually pick today.

## 2. Method
`benches/ladybug_bench.py` (`real_ladybug` 0.15.3, API kuzu-compatible) — a direct
port of `kuzu_bench.py` with a swapped import, so the method is identical to P26:
N nodes, **fanout-8** random edges (independent draw, identical stats), `COPY FROM`
CSV bulk load, **prepared** Cypher `MATCH (a)-[:LINK*1..d]->(b) RETURN b.gid LIMIT
1000`, depths {1,3,6}, 200 q/depth. GenesisBlockDB side = P22 `graph-bench`. Both
embedded, in-process, C: SSD.

## 3. Results (100k nodes / 800k edges)

| Metric | GenesisBlockDB | LadybugDB | Kuzu (P26, parent) |
|---|---|---|---|
| hop1 p50 | **21.6 µs** | 3,637 µs | 3,653 µs |
| hop3 p50 | **2,334 µs** | 15,592 µs | 15,705 µs |
| hop6 p50 | **4,403 µs** | 59,541 µs | 60,914 µs |
| Edge ingest | ~24 s (durable WAL) | **0.5 s** (COPY) | 0.4 s (COPY) |
| Memory (RSS Δ) | 1,057 MB | **96 MB** | 97 MB |

GenesisBlockDB vs LadybugDB: **hop1 ~168×, hop3 ~6.7×, hop6 ~13.5×** faster.
LadybugDB wins **ingest ~48×** and **memory ~11×**.

(10k: GenesisBlockDB hop1 23 µs vs LadybugDB 2,791 µs; Ladybug ingest 0.1 s, RSS 88 MB.)

## 4. Reading — LadybugDB ≈ Kuzu, and the win is clean

- **LadybugDB's numbers are statistically identical to Kuzu's** (hop1 3,637 vs
  3,653 µs; hop6 59.5 vs 60.9 ms; RSS 96 vs 97 MB). Expected: it forks Kuzu's last
  release and has not (yet) diverged on traversal performance. Measuring the fork
  confirms the P26 result transfers to the live competitor an agent-builder would
  use today.
- **The latency win is robust — no payload caveat in GenesisBlockDB's favor.**
  Unlike P29 (RocksDB returned bare ids while GenesisBlockDB materialized full
  node+path objects), here LadybugDB's Cypher *also* returns only `b.gid`, yet is
  6.7–168× slower. GenesisBlockDB wins despite doing **more** per result (cloning
  node + path). Its adjacency-list + direct-call `neighbors()` is built for
  low-latency point/k-hop; LadybugDB pays per-query operator setup + recursive-join
  cost (point latency is not its design target).
- **LadybugDB wins ingest & memory** via Kuzu's columnar + CSR store — same
  sweet-spot split as P26/P28, and again confirming **edge-id (UUID) interning is
  GenesisBlockDB's dominant RAM cost** (top lever: numeric/u64 ids).

**Takeaway:** against the most on-niche competitor (embedded graph+vector for
agent memory), GenesisBlockDB holds a decisive **point/k-hop latency lead
(~168× hop1)** while LadybugDB leads on bulk ingest and memory footprint.
Different sweet spots — GenesisBlockDB for low-latency agent memory, LadybugDB
(like Kuzu) for compact, bulk-loaded graph analytics.

## 5. Caveats
- LadybugDB uses prepared+execute (emits a deprecation notice in 0.15.3 favoring a
  single `execute()`); kept for method-parity with P26 Kuzu. The query plan is
  cached either way, so this does not inflate its latency.
- GenesisBlockDB edge-ingest is the durable-WAL figure (~24 s); Ladybug/Kuzu COPY
  is non-durable bulk load — comparable on time-to-queryable, not durability.
- Cypher `RETURN b.gid` returns ids; GenesisBlockDB returns node+path payloads —
  this asymmetry favors LadybugDB, yet GenesisBlockDB still wins (see §4).

## 6. Program status — competitor matrix complete (8 measured)
Vector: **Chroma** (P15/P21), **Qdrant** (P20), **LanceDB** (P27). Graph:
**Neo4j** (P23), **Kuzu** (P26), **DuckDB+graph** (P28), **RocksDB+graph** (P29),
**LadybugDB** (P30). Plus cost proofs (governance P24, K-Impact P25) and scale (P22).

Reproduce:
```
pip install real_ladybug numpy psutil
GB_LADYBUG_N=100000 python benches/ladybug_bench.py
GB_VBENCH=<dir> GB_GRAPH_N=100000 cargo run --release --bin graph-bench
```
