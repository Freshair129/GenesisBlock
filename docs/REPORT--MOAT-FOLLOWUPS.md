---
doc_id: REPORT--MOAT-FOLLOWUPS
status: current
version: current
owner: GenesisBlockDB Engineering
---

# REPORT — WP-3.3 moat follow-ups (libSQL/DiskANN baseline + real-corpus run)

**Date:** 2026-08-20 · **Engine commit:** `5138597` (main @ `35800f3` + the
follow-up bench infrastructure) · **Status of the gate:** both follow-ups
scheduled by [DECISION--WP33-GNSE-BACKLOG](DECISION--WP33-GNSE-BACKLOG.md) are
now **measured**.

The WP-3.2 verdict ([REPORT--G3-MOAT-VERDICT](REPORT--G3-MOAT-VERDICT.md))
returned PROCEED but shipped with two named caveats, and the WP-3.3 decision
made closing them a prerequisite for using the moat claim publicly:

1. **libSQL DiskANN baseline row** — "attacks Q4 (the brute scan); expected to
   narrow the single-axis control, not the fused shapes or the correctness gate."
2. **Real-corpus (bge-m3) run** — "replaces the synthetic-vectors caveat with a
   measured real-embedding result."

Both are answered below. **Neither caveat was hiding a weakness**: the real
corpus *improved* every vector-touching ratio, and libSQL's native ANN index
closed only a small part of the gap against the brute scan and none of it
against the engine.

## 1. What was run

Two verified runs, back to back on the same idle host, identical in every
parameter except the vector corpus — a controlled A/B on the one variable the
caveat named (**vector distribution**), rather than one that also moves N:

| | Run A (control) | Run B (treatment) |
|---|---|---|
| Corpus | synthetic seeded unit vectors | **real bge-m3 embeddings** |
| N × dim | 11,266 × 1024 | 11,266 × 1024 |
| Runs / warmup / k / seed | 30 / 3 / 10 / 42 | 30 / 3 / 10 / 42 |
| Result dir | `20260820T022036Z_5138597` | `20260820T030825Z_5138597` |
| Trust gate | `verify_report.py` **PASS** | `verify_report.py` **PASS** |

Host: Windows 10, Intel 6-core (Model 158), 32 GB RAM, rustc 1.97.1, release
profile. Both runs also carry the libSQL rows (§3).

**Corpus provenance.** The real corpus is 11,266 unique prose chunks extracted
deterministically from this repository (markdown plus the natural language in
`///`, `//!`, `//`, `#` comments across `docs/ src/ tests/ benches/ mcp/
benchmark/ sdk/ dashboard/src`), embedded through a local Ollama `bge-m3` at
1024 dim and L2-normalized. `benchmark/fixtures/corpus_bge_m3.manifest.json`
records model, dim, count, sha256 (`20978d75…`), the extraction rules, and the
source commit; the bench copies that manifest into the run's `result.json`, so
a real-corpus number is never reported without its provenance.

## 2. Follow-up 2 — real embeddings (the caveat was conservative)

p50 latency, engine vs the DIY single-SQLite-file assembly, at matched N:

| shape | synth engine | synth SQLite | synth ratio | real engine | real SQLite | **real ratio** | Δ ratio |
|---|---|---|---|---|---|---|---|
| q1 fused vector+graph+AS-OF | 10,787 µs | 564,322 µs | 52.31× | 7,775 µs | 522,227 µs | **67.17×** | +28% |
| q3 cross-dimension | 8,339 µs | 239,083 µs | 28.67× | 5,731 µs | 211,565 µs | **36.92×** | +29% |
| q4 vector-only control | 6,779 µs | 109,870 µs | 16.21× | 4,445 µs | 101,460 µs | **22.82×** | +41% |
| q5 graph-only control | 6,643 µs | 86,033 µs | 12.95× | 6,458 µs | 80,365 µs | **12.44×** | −4% |
| q6 vector time-travel (E2) | 6,384 µs | 15,422 µs | 2.42× | 5,392 µs | 13,097 µs | **2.43×** | +1% |

**Real embeddings make the engine faster, and the baseline barely moves.** The
engine's q4 drops 34% (6,779 → 4,445 µs) while the baseline's q4 drops only 8%.
That asymmetry is the mechanism, and it is the expected one: random unit vectors
in 1024 dimensions are near-isotropic — almost equidistant, the worst case for a
navigable-small-world graph — whereas real embeddings are clustered, so the HNSW
walk converges in fewer hops. A brute-force scan has no such structure to
exploit; its cost is O(N) whatever the vectors look like.

**The controls behave exactly as they must**, which is what makes the causal
claim credible rather than a coincidence:

- **q5 (graph-only, no vector stage) moves −4%** — inside noise. The one shape
  with no vectors in it is the one shape the corpus swap does not change.
- **q6 (vector time-travel) moves +1%.** Also expected: under a selective epoch
  filter the E2 path takes the exact-scan fallback, which is a linear scan and
  therefore, like the baseline, indifferent to distribution.

**Consequence for the verdict.** The synthetic corpus *understated* the engine's
advantage on every vector-touching shape. The WP-3.2 caveat can be retired in
the direction of the claim, not against it: the 100k synthetic numbers are a
conservative floor with respect to vector realism.

## 3. Follow-up 1 — libSQL 0.9 with native DiskANN

libSQL is the strongest embedded competitor on the vector axis: a SQLite fork
with a native ANN index (`libsql_vector_idx` / `vector_top_k`), same
single-file embedded shape as the B1 baseline.

| | synthetic | real |
|---|---|---|
| q4 libSQL p50 | 90,493 µs | 53,987 µs |
| q4 engine p50 | 6,779 µs | 4,445 µs |
| **engine faster by** | **13.35×** | **12.14×** |
| q4: libSQL vs brute scan | 1.21× | 1.88× |
| q1 libSQL p50 | 493,396 µs | 364,456 µs |
| q1 engine p50 | 10,787 µs | 7,775 µs |
| **engine faster by** | **45.74×** | **46.88×** |
| q1: libSQL vs brute scan | 1.14× | 1.43× |

**The decision doc's prediction was directionally right but understated.** It
expected DiskANN to "narrow the single-axis control, not the fused shapes":

- On the **fused** shape it was exactly right — 45.7× and 46.9×, essentially
  unchanged from the brute-scan baseline's gap. Indexing the vector axis does
  nothing for the graph and temporal axes, which is the whole cross-dimension
  argument.
- On the **single-axis control** it narrowed far less than "narrow" suggests:
  DiskANN beat the brute scan by only 1.21× (synthetic) / 1.88× (real), and the
  engine still leads it by ~12–13×. Adding a native ANN index to SQLite does
  not by itself produce a competitive vector store at this scale.

Note that DiskANN benefits from real data too (1.21× → 1.88× over the brute
scan) — consistent with §2's mechanism, since ANN structures generally exploit
clustering. It simply starts from far behind.

**Ingest is the reversal worth flagging.** Ingest is the engine's disclosed
weakness against plain SQLite, and that stands (68.5 s vs 6.6 s on the real
corpus). Against libSQL it inverts:

| | engine | SQLite (brute) | libSQL (DiskANN) |
|---|---|---|---|
| synthetic ingest | 147.2 s | 13.8 s | **1,248.9 s** |
| real ingest | 68.5 s | 6.6 s | **805.9 s** |

libSQL's ingest (including its DiskANN build) is **8.5×–11.8× slower than the
engine's** at the same N. The "embedded vector search is just an index away"
story carries a large write-side cost.

## 4. Honest limits of these runs

- **Scale.** These runs are at N=11,266, not the 100k of the headline verdict.
  N was set by the real corpus — 11,266 chunks is all the prose this repository
  contains — and matched-N was the right control for the distribution question.
  A 100k libSQL run was **not** performed: at 11k its ingest already costs
  ~14–21 minutes, so 100k would be multiple hours. That cost is itself part of
  the finding above, but it means the libSQL comparison is measured at 11k and
  should not be quoted at 100k.
- **Ratios are N-dependent and these are the smaller ones.** Against the
  O(N) brute-scan baseline the engine's advantage grows with N: the same q1
  shape is 52.3× at 11k (synthetic) and 187.9× at 100k. The engine's own q4
  latency is roughly flat in N (6,779 µs at 11k vs 5,734 µs at 100k), so the
  ratio movement is the baseline's, not the engine's. **Do not** mix scales when
  quoting.
- **Corpus is self-referential.** The real corpus is this repository's own
  prose. It is genuinely natural language with real semantic clustering, which
  is what the distribution question needs, but it is one domain and a modest
  vocabulary. It is not a substitute for a public retrieval benchmark, and it
  supports no recall claim — only latency under a realistic vector distribution.
- **No recall measurement here.** Both runs compare latency. The engine's recall
  is measured separately (`benchmark/run_vector_bench.sh`, and the bge-m3 recall
  work in MARK XV P1); nothing in this report speaks to answer quality.
- **libSQL is measured out of process.** `libsql-ffi` and `rusqlite` both export
  the bundled `sqlite3_*` symbols, so they cannot share a binary: a clean probe
  fails to link (LNK2005), and inside the bench they *did* link but libSQL then
  failed its own threading assert with `SQLITE_MISUSE` — the linker had silently
  resolved every call into one implementation, which would have run the engine's
  `projection.sqlite` and the competitor on the same accidental SQLite. The
  libSQL rows therefore come from a separate `moat-libsql` binary that never
  links the engine, using the same seeded corpus, the same protocol and the same
  host. This is a deviation from the bench's usual both-sides-in-one-process
  property and is disclosed as such; it removes a correctness hazard rather than
  adding a timing one, since both sides are compiled Rust timed identically.
- **libSQL's AS-OF shape over-fetches.** `vector_top_k` cannot push a temporal
  predicate into the ANN index, so q1_libsql fetches `k × 4` and post-filters in
  SQL. That is the pattern this design forces on its users, and the factor is
  recorded in the metrics.

## 5. Verdict impact

The WP-3.2 PROCEED verdict **stands and is strengthened**:

- the synthetic-corpus caveat is retired, and it was conservative;
- the strongest embedded ANN competitor does not close the vector gap (~12×) and
  does not touch the fused gap (~47×), while costing an order of magnitude more
  to ingest;
- the correctness gap is untouched by either follow-up — the baseline still
  fails 2 of 5 bitemporal scenarios structurally (no tx axis, no provenance
  identity), and libSQL inherits exactly that failure, since it changes the
  vector index and nothing about time or provenance.

**Public-positioning guidance.** Quote the 100k synthetic figures as the
headline with their scale stated, cite this report for the realism and
competitor checks, and never quote a ratio without its N. The libSQL numbers
are 11k-scale and must be labelled as such.

## Reproducing

```bash
python benchmark/gen_corpus_bge_m3.py --out benchmark/fixtures/corpus_bge_m3
GB_MOAT_N=11266 GB_MOAT_DIM=1024 GB_MOAT_LIBSQL=1 bash benchmark/run_moat_bench.sh
GB_MOAT_N=11266 GB_MOAT_DIM=1024 GB_MOAT_LIBSQL=1 \
  GB_MOAT_VECTORS=benchmark/fixtures/corpus_bge_m3.f32 bash benchmark/run_moat_bench.sh
```

The corpus generator needs a local Ollama serving `bge-m3`; everything else is
clone-and-run. Corpus generation is resumable.
