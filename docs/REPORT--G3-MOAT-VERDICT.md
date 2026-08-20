---
status: current
---

# REPORT — G3 Moat Bench Verdict (WP-3.2)

> **Status:** measured · **Gate consumed by:** WP-3.3 (USER decision gate)
> **Spec:** `BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md` §3 · **Baseline choice:** interview
> ROUND2 (`genesis-interview/ROUND2.md`) — the single-SQLite-file DIY assembly is the primary
> competitor at the embedded tier, not Qdrant+Neo4j.
> **Run:** `benchmark/results/moat/20260819T102944Z_c4ae6cf/` (verify_report.py **PASS**,
> commit `c4ae6cf`, dirty=False; a prior run at `20260819T102023Z_0754b4b` reproduced the
> same ratios within noise). Harness: `benches/moat_bench.rs` via `benchmark/run_moat_bench.sh`.

## One-line verdict

**PROCEED — the moat is real at the embedded tier.** Every cross-dimension query clears the
ROUND2 G3-e bar (≥5× fused p50 at 100k) by more than an order of magnitude (min **114.9×**),
and the baseline structurally fails 2 of 5 bitemporal correctness scenarios it cannot express.

## Setup (fairness rules per spec §2)

- Corpus: deterministic seeded unit vectors, **100,000 nodes × 1024-dim**, 499,955 edges
  (~5/node, mild preferential skew), ~10% of edges retroactively closed; AS-OF selector
  2023-01-01 bisects the corpus. Synthetic — latency-comparable, recall-inert (recall claims
  live in the vector-bench suite, not here).
- Both stores **in-process in one Rust binary**; the RRF glue code is shared. The baseline
  gets compiled-Rust glue instead of its real-world TS/Python — reported wins are **lower
  bounds**.
- Baseline B1 = one SQLite file (WAL, synchronous=NORMAL): brute-force f32 scan (the
  sqlite-vec-stable model — its author's published numbers are for exactly this scan),
  recursive-CTE hops with the temporal window on every edge, and the published single-axis
  audit-history temporal pattern.
- 30 measured runs per shape after 3 warmups; distinct query vectors/seeds per run.
- Engine driven only through the public `Storage` API; both runs on the committed tree.

## Latency (p50 over 30 runs, 95% CI in parentheses)

| Query | Shape | Engine | SQLite assembly | Ratio |
|---|---|---|---|---|
| **Q1** | vec top-20 → 2-hop `references` → AS OF → RRF | **13.2 ms** (±0.7) | 2,488 ms | **187.9×** |
| **Q3** | 3-hop AS OF + vec-similar → RRF | **9.1 ms** (±0.4) | 1,044 ms | **114.9×** |
| Q4 *(control)* | pure vector top-10 | 5.7 ms (±0.2) | 528 ms | 92.0× |
| Q5 *(control)* | pure 3-hop traverse | 4.5 ms (±0.2) | 377 ms | 83.9× |

p99s track p50s closely on both sides (engine Q1 p99 18.2 ms; SQLite Q1 p99 2,536 ms) — no
tail pathology. In-process call counts are near-parity (Q1: 7 vs 8) — embedded-vs-embedded,
the §3.6 round-trip axis is not the differentiator; latency and correctness are.

- **Cross-dimension advantage grows with span, as the spec requires for a genuine moat:**
  Q1 (3 dimensions) 187.9× > Q3 114.9× > single-axis controls 92×/83.9×. The win is not a
  single-axis artifact: it compounds.
- Q2 (vec+lexical) **skipped and disclosed** — the engine FTS axis (S3) has not shipped;
  the shape is not comparable yet.
- Ingest (secondary, reported for honesty): SQLite bulk-inserts 100k+500k rows in **33.1 s**;
  the engine's durable ingest (WAL + HNSW + 50k retroactive edge retractions) takes
  **141.9 s**. The known ingest-side weakness (P31) stands; the moat claim is read-side.

## Bitemporal correctness gate (ROUND2 G3-e bar (a))

The WP-3.1 matrix scenarios (`tests/bitemporal_matrix_wp31_tests.rs`) run against **both**
stores inside the bench:

| Scenario | Engine | SQLite assembly | Why |
|---|---|---|---|
| Valid-time point query on superseded node | ✅ | ✅ | single-axis history table suffices |
| **Two-axis: belief at commit S1 about mid-2021** | ✅ | ❌ | no tx axis exists in the published audit-history pattern (bytefish.de documents the underlying stable-transaction-time defect) |
| Retroactive correction flips the same valid-time answer | ✅ | ✅* | *baseline flips it by destroying the pre-correction belief — which is exactly the two-axis query it then can't answer |
| Interval boundaries (start-inclusive, end-exclusive) | ✅ | ✅ | plain WHERE handles boundaries |
| **Audit chain with provenance (`caused_by` identities)** | ✅ | ❌ | history rows carry no provenance identity |

A sufficiently determined DIY team could hand-roll a two-axis schema in SQLite — no
**published** two-axis, retraction-preserving pattern was located (ROUND2 evidence), and the
one documented defect (per-statement `CURRENT_TIMESTAMP` across triggers) sits exactly where
tx-time needs to be stable. The honest claim stays the strict ROUND2 form: *the only
embedded, in-process engine with engine-enforced row-level bitemporality alongside vector
ANN and graph traversal* — naming LadybugDB (graph+HNSW, no temporal) and Graphiti
(app-layer bitemporality) when we say it.

## STOP numbers, applied

- Spec §3.6 (service-composed framing): PROCEED requires ≥2× round-trips **and** ≥30% p50 —
  round-trips are near-parity embedded-vs-embedded (ROUND2 conceded this axis), p50 saving
  is 99.5%. Not the operative gate here.
- **ROUND2 G3-e (embedded framing, the operative gate): correctness suite must-pass +
  ≥5× fused p50 at 100k → both cleared** (engine 5/5 scenarios; 114.9–187.9× vs the 5×
  bar). The pre-registered "would change my mind" condition — the SQLite assembly passing
  the correctness suite within ~2× — is falsified on both axes.

## Caveats (what this run does NOT show)

1. Synthetic vectors: no recall/quality claim (covered by vector-bench with real bge-m3,
   MARK XV P1). 2. Single-threaded query loop: no concurrency claim. 3. libSQL DiskANN
   baseline not yet run — it would attack Q4 (the scan) but not the CTE hops, the fused
   shapes, or the correctness gate; queue it behind WP-3.3. 4. Windows-only host for this
   run (`Windows-10-10.0.19045`, rustc 1.97.1); the harness is clone-and-run on all three
   CI OSes. 5. 1M-scale re-run not required — the §3.6 "inconclusive" branch was not hit.

## Recommendation to WP-3.3

Fund the read-side moat line (G3 positioning + the deferred GNSE items it gates); keep the
ingest-throughput caveat disclosed. The libSQL-DiskANN row and a real-corpus (bge-m3) run
are the two follow-ups worth scheduling before public positioning ships.

## Follow-up outcome (2026-08-20)

Both follow-ups were run — see [REPORT--MOAT-FOLLOWUPS](REPORT--MOAT-FOLLOWUPS.md).
This verdict **stands and is strengthened**:

- **The synthetic-corpus caveat was conservative.** On real bge-m3 embeddings at matched
  N, every vector-touching ratio *improved* (q1 52.3× → 67.2×, q4 16.2× → 22.8×); the
  graph-only control moved −4%. Random unit vectors are near-isotropic and are the
  hostile case for the HNSW walk, while a brute scan is distribution-blind.
- **libSQL/DiskANN does not close the gap.** The strongest embedded ANN competitor beat
  the brute scan by only 1.21×/1.88× and left the engine ~12–13× ahead on the vector axis,
  ~47× on the fused shape, at 8.5×–11.8× the engine's ingest cost.
- The ingest weakness against *plain* SQLite stands as disclosed.

Scale caveat unchanged: the follow-ups are at N=11,266 (set by the real corpus), and
ratios against an O(N) baseline grow with N — quote no ratio without its N.
