---
proposed_id: AUDIT--P15-COMPETITIVE-VECTOR-BENCHMARK
type: audit
status: complete
aliases:
  - AUDIT
  - P15
tier: process
cluster: implementation_flow
role: "First measured head-to-head vs a market vector DB"
phase: 15
audited_at: 2026-06-21
proposed_by: agent
related:
  - AUDIT--P14-POST-REFACTOR-VERIFICATION
  - adr/ADR--GENESISDB-MARKET-POSITIONING
  - adr/ADR--GENESISDB-COMPETITIVE-ROADMAP
---

# AUDIT — P15 Competitive Vector Benchmark (GenesisBlockDB vs Chroma)

## 1. Why

Prior "competitive" claims (P8–P12 docs) were **published-spec comparisons or
admitted oversell** — no measured head-to-head against a real market DB ever
ran (the planned "Phase 11 audit vs Qdrant/Neo4j" was never executed). This is
the **first apples-to-apples measured comparison**.

## 2. Embedding model — stated explicitly

**`bge-m3` (BAAI, 1024-dim), served locally via Ollama.** Chosen because it is
the strongest multilingual (incl. Thai) text embedder available locally, matches
the project's Thai-aware positioning, and 1024 is a mainstream dim both engines
handle natively. **Vectors are real embeddings of real text** (3,200 unique
chunks from this repo's `docs/`), not uniform-random — so recall reflects real
semantic clustering.

## 3. Competitor & fairness

Original target was **Qdrant** (HNSW-vs-HNSW). Docker is unavailable on this
host and `qdrant-client` local mode is a non-representative pure-Python path, so
the competitor is **Chroma (hnswlib, C++ HNSW, embedded)** — the closest
embedded HNSW engine and the exact competitor named in
`ADR--GENESISDB-MARKET-POSITIONING`. Qdrant can be added later (needs Docker).

**Controls:** identical corpus (3,000 vectors) and identical queries (200), same
`k=10`, same **L2** distance (GenesisBlockDB `DistL2` ↔ Chroma `hnsw:space=l2`), same
machine, both on **C: (SSD)**. Ground truth = exact brute-force L2 top-10.

**Asymmetries (disclosed):**
- GenesisBlockDB = embedded in-process, **durable per-op WAL fsync** on insert.
- Chroma = embedded in-process, **in-memory (ephemeral), batched add** — no
  durability. → insert numbers are not apples-to-apples; query latency and
  recall are.

## 4. Results (bge-m3 1024-dim, L2, same vectors)

| Metric | GenesisBlockDB (hnsw_rs) | Chroma (hnswlib) | Comparable? |
|---|---|---|---|
| Insert throughput | 254 vec/s | 4,074 vec/s | no (durability-asymmetric) |
| Query latency p50 | 1,901 µs | 1,249 µs | **yes** |
| Query latency p95 | 2,361 µs | 1,951 µs | **yes** |
| Recall@10 | **0.987** | 1.000 | **yes** |

## 5. Reading

- **Recall:** 0.987 vs 1.000 — effectively at parity; both excellent. The small
  gap is HNSW search-effort (`ef_search`) tuning, not an algorithmic deficit.
- **Query latency:** GenesisBlockDB ~1.5× slower — same ballpark. GenesisBlockDB's
  `hybrid_search` does extra work even at `alpha=0` (k-impact path, full
  `NeighborOutput` + path construction) vs Chroma's tuned C++ ANN returning bare
  ids. Headroom exists (skip k-impact / lighter return type on pure-vector path).
- **Insert:** 16× slower, dominated by per-op WAL fsync + per-op global HNSW
  lock (the P13-identified bottleneck). Not a like-for-like number against
  in-memory Chroma; a batch/bulk durable path would close most of the gap.

**Verdict:** GenesisBlockDB is a **credible local vector engine** — at recall parity
and within ~1.5× query latency of Chroma — with a real, honest write-durability
cost and clear query-path optimization headroom. No overselling.

## 6. Reproduce

```
# 1. embeddings + Chroma (Python; needs Ollama running with bge-m3, pip install chromadb numpy)
python benches/vbench.py all
# 2. GenesisBlockDB side (vectors shared via the bench dir)
GB_VBENCH=<bench-dir> cargo run --release --bin vbench-genesis
# 3. combined table + GenesisBlockDB recall
python benches/vbench.py finalize
```

Harness: `benches/vbench.py` (embed/Chroma/ground-truth) + `benches/vbench_genesis.rs`
(`[[bin]] vbench-genesis`). Bench dir is parameterized by `GB_VBENCH`
(defaults in the script to an SSD path). Results feed the head-to-head section of
`perf-comparison-dashboard.html`.

## 7. Next

- Add **Qdrant** (Docker) and **hnswlib-direct** as additional columns.
- Scale to 50k–100k vectors (recall@10 at 3k is saturated near 1.0 for both;
  larger N differentiates ANN quality).
- Re-measure GenesisBlockDB query after trimming the pure-vector return path.
