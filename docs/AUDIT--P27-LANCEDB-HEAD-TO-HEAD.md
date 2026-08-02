---
proposed_id: AUDIT--P27-LANCEDB-HEAD-TO-HEAD
type: audit
status: historical
aliases:
  - AUDIT
  - P27
tier: process
cluster: implementation_flow
role: "Embedded↔embedded vector head-to-head: GenesisBlockDB vs LanceDB (vs Chroma)"
phase: 27
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P15-COMPETITIVE-VECTOR-BENCHMARK
  - AUDIT--P21-RECALL-LATENCY-FRONTIER
  - AUDIT--P26-KUZU-HEAD-TO-HEAD
  - adr/ADR--GENESISDB-MARKET-POSITIONING
---

# AUDIT — P27 LanceDB Head-to-Head (embedded vector comparator)

## 1. Why
The positioning ADR names **LanceDB** as a nearest comparator (embedded vector,
Rust core — like GenesisBlockDB) but it had never been measured; the report only
claimed it as a named peer. This closes that gap with a fair, like-for-like
embedded-vector head-to-head, alongside Chroma (also embedded) on the same corpus.

## 2. Method
Single corpus, single ground truth, fed identically to all engines
(`benches/vbench.py`): synthetic-clustered vectors, **n=100,000, dim=1024,
k=10, L2**, 200 query vectors, exact L2 ground truth. SSD (C:).

- **GenesisBlockDB** (`vbench-genesis`, hnsw_rs): ef_construction=200, ef_search=100,
  memory-resident; insert = durable batched WAL fsync.
- **Chroma** (hnswlib): in-memory ephemeral, space=l2.
- **LanceDB** 0.33.0 (`IVF_HNSW_FLAT`, single IVF partition ≈ pure HNSW):
  m=16, ef_construction=200, **query ef=100 (matched to GenesisBlockDB ef_search)**,
  on-disk Lance columnar store. 20 warmup queries before timing so we measure
  steady-state (page-cached) latency — the same warm state the memory-resident
  engines enjoy by construction (without warmup LanceDB read the index from disk
  per query and recall sat at 0.80 with the default low ef).

## 3. Results (n=100,000, dim 1024, L2, same vectors)

| Metric | GenesisBlockDB (hnsw_rs) | Chroma (hnswlib) | LanceDB (IVF_HNSW_FLAT) |
|---|---|---|---|
| query p50 | **935.6 µs** | 1,166.5 µs | 8,392.1 µs |
| query p95 | **1,369.5 µs** | 2,320.9 µs | 9,988.1 µs |
| recall@10 | 0.948 | 0.997 | 0.998 |
| insert (vec/s) | 1,718 (durable WAL) | 3,584 (in-mem) | 2,750 (on-disk) |
| RSS | 1,575 MB | — | — |
| durability | durable WAL fsync | in-memory ephemeral | on-disk persisted |

At a **matched recall band (~0.95–1.0)**: GenesisBlockDB point-query p50 is **~9×
lower than LanceDB** (936 µs vs 8.4 ms) and ~1.25× lower than Chroma. With
query ef=100, LanceDB's recall (0.998) is on par with Chroma and above
GenesisBlockDB's ef_search=100 point (0.948) — the recall gap is a tunable ef choice,
not a quality ceiling (see P21 frontier).

## 4. Reading — different sweet spots (honest)

- **GenesisBlockDB wins point-query latency (~9×).** hnsw_rs lives in RAM and the
  query path is a direct in-process call — built for low-latency agent-memory
  retrieval.
- **LanceDB trades latency for on-disk scale & cost.** Its columnar Lance store
  is designed for larger-than-memory datasets and cheap storage, not lowest p50;
  the per-query disk/columnar path costs ~8 ms even warm and HNSW-indexed. Same
  structural tradeoff as Kuzu in P26 (columnar/disk vs memory-resident).
- **Chroma** is the closest latency peer (also in-memory HNSW), slightly slower
  p50/p95 than GenesisBlockDB at higher recall.
- All three are embedded / in-process — a like-for-like architecture comparison.

**Takeaway:** for **low-latency, memory-resident agent memory**, GenesisBlockDB leads
on point-query latency; LanceDB is the pick when the vector set must live on disk
/ exceed RAM at low cost. Not "faster at everything" — faster where designed to be.

## 5. Caveats
- Qdrant numbers from earlier sessions are **not** part of this run (server not
  re-run on this corpus) and are excluded here; P20 holds the Qdrant head-to-head.
- LanceDB insert includes table build + HNSW index build (total time-to-queryable),
  comparable to Chroma building HNSW incrementally on add.
- RSS captured for GenesisBlockDB only (in-process); Chroma/LanceDB RSS not isolated
  this run — memory head-to-head is future work.

## 6. Program status
Vector comparators measured: **Chroma** (P15/P21), **Qdrant** (P20), **LanceDB**
(P27, embedded). Graph: **Neo4j** (P23), **Kuzu** (P26). Still pending:
**DuckDB+graph** (P28), **RocksDB+graph** (P29).

Reproduce:
```
pip install lancedb pyarrow
GB_VBENCH=<dir> python benches/vbench.py synth 100000
GB_VBENCH=<dir> cargo run --release --bin vbench-genesis
GB_VBENCH=<dir> python benches/vbench.py chroma
GB_VBENCH=<dir> python benches/vbench.py lance
GB_VBENCH=<dir> python benches/vbench.py finalize
```
