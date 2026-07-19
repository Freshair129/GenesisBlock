---
proposed_id: AUDIT--P23-NEO4J-HEAD-TO-HEAD
type: audit
status: historical
aliases:
  - AUDIT
  - P23
tier: process
cluster: implementation_flow
role: "Graph head-to-head: embedded GenesisBlockDB vs server Neo4j"
phase: 23
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P22-GRAPH-TRAVERSAL
  - adr/ADR--GENESISDB-MARKET-POSITIONING
---

# AUDIT — P23 Neo4j Head-to-Head (graph traversal)

## 1. Setup

Neo4j (`neo4j:latest`, Docker, `NEO4J_AUTH=none`, 4 GB heap) via the Python bolt
driver (`benches`/`neo4j_bench.py`) vs GenesisBlockDB embedded (P22, `graph-bench`).
Same topology parameters: N nodes, fanout-8 random directed edges, depths {1,3,6},
200 queries/depth, `LIMIT 1000`, C: SSD. Edges are independent draws of identical
statistics (not byte-identical sets).

**Caveat (the whole point):** GenesisBlockDB runs **in-process**; Neo4j is
**client-server** — each query pays a bolt round-trip + Cypher planning + JVM.
Memory is GenesisBlockDB RSS vs Neo4j JVM heap+store. This compares the engines *as
typically deployed* (embedded vs server), not two embedded libraries.

## 2. Results

| N | metric | GenesisBlockDB (embedded) | Neo4j (server) | GenesisBlockDB advantage |
|---|---|---|---|---|
| 10k | hop1 p50 | 23.1 µs | 4,273.8 µs | **185×** |
| 10k | hop3 p50 | 1.97 ms | 20.69 ms | 10.5× |
| 10k | hop6 p50 | 4.21 ms | 32.11 ms | 7.6× |
| 10k | memory | 146 MB | 786 MB | 5.4× lighter |
| 100k | hop1 p50 | 21.6 µs | 2,590.4 µs | **120×** |
| 100k | hop3 p50 | 2.33 ms | 19.74 ms | 8.5× |
| 100k | hop6 p50 | 4.40 ms | 31.64 ms | 7.2× |
| 100k | edge ingest | 24.4 s | 23.3 s | ~par |
| 100k | memory | 1,057 MB | 1,077 MB | ~par |

## 3. Reading

- **Traversal: GenesisBlockDB is 7–185× faster.** hop1 shows the largest gap because
  Neo4j's per-query cost there is dominated by bolt + Cypher planning + JVM, not
  graph work; GenesisBlockDB does a direct in-process index lookup (~20 µs).
  Deeper hops (7–10×) reflect engine + protocol together.
- **Ingest is ~par at 100k** (both ~23–24 s for 800k edges) — Neo4j's batched
  index-backed `MATCH…CREATE` is competitive with GenesisBlockDB's durable batch path.
- **Memory ~par at 100k** (~1.06 GB each); at 10k Neo4j's JVM baseline (~700 MB)
  makes it 5× heavier — GenesisBlockDB has no fixed runtime floor.
- **Honest framing:** much of the hop1 gap is the embedded-vs-server tax. An
  *embedded* Neo4j would narrow it. But the positioning thesis is exactly that an
  agent-memory engine should be embedded and not pay that tax — which the numbers
  make concrete against the best-known graph DB.

## 4. Positioning note

This supports the reframed comparator set (Kuzu, DuckDB+graph, RocksDB+graph
layer — embedded analytics/agent-memory), with Neo4j as the well-known reference.
A Kuzu head-to-head (embedded vs embedded) would be the fairest next datapoint.

## 5. Program status

P22–P25 complete (graph traversal, Neo4j head-to-head, governance cost, K-Impact
cost). The graph claim now has measured evidence across scale, a named-competitor
comparison, and cost proofs for governance and incremental K-Impact.

Reproduce:
```
docker run -d --name gb-neo4j -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=none neo4j:latest
GB_NEO_N=100000 python benches/neo4j_bench.py
GB_VBENCH=<dir> GB_GRAPH_N=100000 cargo run --release --bin graph-bench
```
