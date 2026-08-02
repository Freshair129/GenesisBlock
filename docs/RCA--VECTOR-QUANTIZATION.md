---
proposed_id: RCA--VECTOR-QUANTIZATION
type: rca
status: historical
tier: process
proposed_at: 2026-06-30
proposed_by: agent
---

# RCA--VECTOR-QUANTIZATION

**Status:** Accepted — root cause confirmed against source; corrective plan scoped (see [[PLAN--VECTOR-QUANTIZATION-REFINEMENT]])
**Date:** 2026-06-30
**Deciders:** Engine owner (Boss)
**Related:** [[ADR--GENESISDB-VECTOR-QUANTIZATION]] (the decision this RCA re-examines)

## Summary (TL;DR)

The per-collection vector quantization feature does **not** deliver its advertised RAM
reduction when reranking is enabled. The exact-f32 rerank sidecar — which is *mandatory*
for usable recall, because BQ-alone recall is 0.6845 — is held **fully resident** as a
`RwLock<Vec<f32>>`, loaded whole into a flat `Vec<f32>` at collection-open time. That one
resident f32 copy is ~94% of a BQ-rerank-on collection's per-vector resident bytes, so
BQ+rerank collapses from a nominal **32×** reduction to **~1.9×** (and SQ8+rerank from 4×
to ~1.3×). The [[ADR--GENESISDB-VECTOR-QUANTIZATION]] explicitly warned (lines 107–108)
that a resident f32 arena for rerank "cancels the RAM win" and proposed `mmap`; `memmap2`
was then declined and the resident sidecar shipped anyway — the engine now does the exact
thing the ADR warned against. The ADR is also stale: it still describes f32-sidecar rerank
as "deferred" though it shipped on 2026-06-23.

## Background / Timeline

- **2026-06-22** — [[ADR--GENESISDB-VECTOR-QUANTIZATION]] proposed. Layer A (SQ8, 4×) and
  Layer B (BQ, 32×, "DESIGNED, opt-in"). Layer B's design (ADR lines 103–111) calls for
  oversample + rerank where "exact vectors for rerank come from the on-disk `vec_<name>.bin`
  via **`mmap`** (memmap2), *not* a resident f32 arena — otherwise the RAM win is cancelled."
- **2026-06-23** — SQ8 + BQ shipped as the "full resident cut." `memmap2` was **declined**
  (ADR decision-update block, lines 24–32: "`memmap2` was declined, which reshapes BQ's
  rerank"). BQ shipped first as a **no-rerank** cut.
- **2026-06-23 (same line, later)** — f32-sidecar rerank **shipped**: `f32_sidecar:
  Option<RwLock<Vec<f32>>>`, `fvec_<name>.bin` persistence, `RERANK_OVERFETCH = 8`, the
  over-fetch + exact-f32 re-score path in `hybrid_search`, and `tests/rerank_tests.rs`
  (6 deterministic tests, green). Because `memmap2` was declined, the sidecar was made a
  resident `Vec<f32>` rather than an on-disk mmap — i.e. the exact design the ADR rejected.
- **2026-06-24 (MARK XV P1)** — RSS + recall matrix measured on real bge-m3 embeddings
  (see [[AUDIT--P33-RSS-QUANT-MATRIX]], [[project_mark_xv_rss500k]]). The measurement is
  what surfaced the resident-sidecar cost: BQ resident reduction observed at ~2.93× (not
  32×), SQ8 ~2.06× (not 4×). *(Frame note: the ~2.93× / ~2.06× figures are **whole-process
  RSS** reductions at 1M scale — they fold in node bookkeeping, WAL, HNSW graph overhead, and
  allocator slack, a broader frame than the per-vector arena+HNSW+sidecar accounting that
  yields the 1.88× / 1.33× in Evidence. The two are complementary, not contradictory: both
  show the resident sidecar erasing most of the quantization win.)*
- The ADR (dated 2026-06-22, status updated 2026-06-23) **predates and contradicts** the
  shipped rerank: its Status line, Action Item 5 (`[~]`), and BQ Outcome block all still
  say the f32-sidecar rerank stage is "deferred."

## Root Cause

The rerank sidecar is **resident, not memory-mapped**. Three facts from `src/lib.rs`
establish the mechanism:

1. **Declared fully heap-resident.** The sidecar is a `Vec<f32>` behind a lock, one f32
   per dimension per vector, parallel to the quantized arena
   (`src/lib.rs:837-843`):

   ```rust
   /// Optional exact-f32 sidecar for rerank: a flat `Vec<f32>` parallel to the
   /// quantized arena (vector at arena_id `i` occupies `[i*dim .. (i+1)*dim]`...
   /// Persisted as `fvec_<name>.bin`.
   pub f32_sidecar: Option<RwLock<Vec<f32>>>,
   ```

2. **Loaded whole on open.** At collection-open the entire `fvec_<name>.bin` is read into
   RAM via `fs::read` + `chunks_exact(4)` and parked in the lock — no demand paging, no
   windowing (`src/lib.rs:3708-3716`). It is written back wholesale on `save_state`
   (`src/lib.rs:3573-3578`). Every staged vector also appends its full f32 slice to the
   sidecar (`src/lib.rs:927-932`).

3. **No mmap anywhere.** GenesisBlockDB never calls any mmap API for the sidecar. (The
   `mmap-rs` entry in `Cargo.lock` is a transitive dependency of the third-party `hnsw_rs`
   crate, not a path GenesisBlockDB code exercises.)

The defect is **shipping the exact thing the ADR warned against.** The ADR's own Layer-B
design states (ADR lines 107–108):

> 3. Exact vectors for rerank come from the on-disk `vec_<name>.bin` via **`mmap`**
>    (memmap2), *not* a resident f32 arena — **otherwise the RAM win is cancelled.**
>    Rerank touches only the top `k*oversample` rows, so only those pages fault in.

`memmap2` was declined (over a Windows-portability concern), but **no non-mmap on-disk
alternative was substituted** — the implementation fell back to a resident `Vec<f32>`,
which is precisely the "resident f32 arena" the ADR said would cancel the RAM win. The
quantized arena (u8 / u64) is still small, but the full-precision copy needed for rerank
is now resident again, so the dominant term in resident bytes is back to f32.

## Evidence

### Per-vector resident RAM (1536-dim, e.g. bge-m3 / OpenAI)

Resident RAM per vector is **arena element + HNSW's own copy** (hnsw_rs holds an
independent copy of every vector — ADR lines 24–27; `VecIndex` enum at `src/lib.rs:735-739`;
`insert_f32` converts and hands the vector to `h.insert` at `src/lib.rs:750-762`) **plus
the f32 sidecar when rerank is on**. Element sizes: f32 = 1536×4 = 6144 B; SQ8/u8 =
1536×1 = 1536 B; BQ/u64 = `bq_words(1536)` = 24 words × 8 = 192 B (1 bit/dim).

| Config            | arena (B) | HNSW copy (B) | sidecar f32 (B) | resident/vec (B) | reduction vs f32 |
|-------------------|----------:|--------------:|----------------:|-----------------:|-----------------:|
| None (f32)        | 6144      | 6144          | —               | **12288**        | 1.00×            |
| SQ8, rerank off   | 1536      | 1536          | —               | **3072**         | 4.00×            |
| SQ8, rerank on    | 1536      | 1536          | 6144            | **9216**         | **1.33×**        |
| BQ, rerank off    | 192       | 192           | —               | **384**          | 32.0×            |
| BQ, rerank on     | 192       | 192           | 6144            | **6528**         | **1.88×**        |

Citations: arena element sizing `byte_size` `src/lib.rs:651-657` and `bq_words`
`src/lib.rs:544-546`; HNSW independent copy `src/lib.rs:735-739`, `src/lib.rs:750-762`;
sidecar field `src/lib.rs:837-843`, append `src/lib.rs:927-932`, sidecar allocated only
when `rerank = true` AND `quant != None`.

The arithmetic of the defect: for BQ rerank-on, the f32 sidecar is **6144 / 6528 = 94.1%**
of resident bytes, and the resident reduction vs f32 is **12288 / 6528 = 1.88×** — versus
the **32×** the same arena delivers with rerank **off** (12288 / 384). The headline 32×
is real only in the rerank-off configuration; with rerank on (the configuration needed for
usable recall) it is ~1.9×.

### Why the sidecar exists: BQ is unusable without it

The reason this expensive resident copy was added at all is recall. Measured on real
bge-m3 embeddings (n=3000, k=10, ef=200, exact-L2 ground truth; [[AUDIT--P33-RSS-QUANT-MATRIX]],
[[REPORT--2026-06-24-MARK-XV-P1-RSS-DISK-TH]]):

| quant | rerank | recall@10 | vs f32  | p50 µs |
|-------|:------:|----------:|--------:|-------:|
| none  | –      | 0.9875    | —       | 1671   |
| sq8   | off    | 0.9485    | −0.039  | 1686   |
| sq8   | **on** | 0.9875    | **±0**  | 1694   |
| bq    | off    | **0.6845**| −0.303  | 227    |
| bq    | **on** | 0.9655    | −0.022  | 317    |

BQ-alone recall of **0.6845** is catastrophic — not shippable for retrieval. Rerank lifts
it to 0.9655 and lifts SQ8 to exact-f32 parity (0.9875). So rerank is **not optional** on
any quantized collection that must hold recall; the resident sidecar that cancels the RAM
win is therefore on the *mandatory* path, not an edge case.

## Impact

- **Mobile / embedded (MARK XVI).** The single-device, memory-constrained targets
  ([[project_mark_xvi_mobile]]) are the primary beneficiaries of quantization. With the
  resident sidecar, a BQ collection that should fit in ~1/32 of the f32 footprint actually
  needs ~1/1.9 — the RAM ceiling the feature was supposed to raise is barely moved when
  recall is preserved.
- **1M → 2M single-device scale.** The scale roadmap ([[project_mark_xv_rss500k]],
  [[ROADMAP]]) banks on the quantization RAM win to reach 2M vectors on one host. At 1.88×
  instead of 32× (BQ) or 1.33× instead of 4× (SQ8), the resident budget the plan assumed
  does not materialize once rerank is on.
- **Credibility / "false advertising."** BQ's headline "32× reduction" (ADR lines 100–102)
  is accurate only with rerank off — i.e. only in the configuration no one can ship for
  retrieval. Quoting 32× while the usable configuration delivers ~1.9× misrepresents the
  feature. SQ8's "4×" has the same gap (~1.3× with rerank on).
- **Operators are blind to it.** There is no surfaced metric for per-collection quant mode
  or resident sidecar bytes, so an operator cannot see that their "compressed" collection
  is mostly f32 in RAM.

## Contributing Factors

- **ADR stale / "deferred" language.** The ADR Status line, Action Item 5 (`[~]`), and BQ
  Outcome block all say f32-sidecar rerank is deferred, though it shipped on 2026-06-23
  with `fvec_<name>.bin`, `RERANK_OVERFETCH = 8`, and the over-fetch + re-score path,
  exercised by `tests/rerank_tests.rs`. The doc that contained the "don't make it resident"
  warning was never updated to reflect that a resident sidecar shipped, so the warning was
  effectively lost. (ADR Status line and Action Items 4/5/6 need correction.)
- **`memmap2` declined without a non-mmap substitute.** The Windows-portability concern that
  killed `memmap2` (see [[feedback_bench_windows]]) was reasonable, but the response was to
  make the sidecar resident rather than to use positioned disk reads (`seek` + `read_exact`)
  — a portable, non-mmap way to fetch only the over-fetched candidate rows from disk. The
  decision threw out the on-disk requirement along with the mmap mechanism.
- **BQ uses raw sign with no centering.** `bq_pack` sets a bit iff a component is `> 0.0`
  (`src/lib.rs:548-558`); there is no per-dimension mean/median subtraction at any call site
  (`src/lib.rs:661-670`, `src/lib.rs:756-760`, `src/lib.rs:778-785`, `src/lib.rs:804-813`).
  For positively-biased embeddings (e.g. bge-m3 after layer norm) many sign bits carry
  near-zero information, depressing BQ-alone recall (0.6845) and thus **increasing the
  dependence on the costly rerank sidecar**.
- **Oversample is hardcoded.** `RERANK_OVERFETCH = 8` is a compile-time constant with no
  per-query or per-collection knob (`src/lib.rs:575-578`, `src/lib.rs:2573-2586`). (Note:
  `HybridSearchInput.ef_search` *is* a per-query override — `src/lib.rs:247-262`,
  resolved per-query → per-collection → global at `src/lib.rs:2573-2586`, and reachable
  over REST via `hybrid_search_handler`, `src/router.rs:371-374` — but the over-fetch
  multiplier specifically is not tunable.) Operators have no latency/recall dial on the
  rerank pool.
- **SQ8 fixed global affine scale.** `SQ8_SCALE = SQ8_BIAS = 127.5` is identical for every
  dim/collection/vector and hard-clamps out-of-range values (`src/lib.rs:518-539`). Lossless
  for L2-normalized cosine embeddings, but un-normalized / L2 vectors clamp and lose
  magnitude — another lever that can raise SQ8-alone recall and reduce rerank dependence.

## Corrective Actions

The full corrective plan lives in [[PLAN--VECTOR-QUANTIZATION-REFINEMENT]]. Priority order:

- **P0 — Off-RAM rerank sidecar (the primary fix; makes the feature work as advertised).**
  Replace the resident `RwLock<Vec<f32>>` sidecar with **positioned disk reads** of only the
  over-fetched candidate rows from `fvec_<name>.bin`: `seek` to offset `d_id * dim * 4` and
  `read_exact` one row, for each of the `k * RERANK_OVERFETCH` candidates, behind a small
  LRU page cache. **No `mmap`** (positioned reads are portable; mmap was the reason `memmap2`
  was declined). This restores the full 4× (SQ8) / 32× (BQ) **resident** win and matches the
  Qdrant model — "compressed in RAM, original on disk, rescore top-N." This is the action
  that closes the root cause; the rest are recall/ops refinements.
- **P1a — BQ per-dim centering.** Compute a per-dimension mean at compaction and subtract it
  before the sign bit, improving bit balance and BQ-alone recall (reduces rerank dependence).
- **P1b — Per-query oversample knob.** Expose `RERANK_OVERFETCH` as
  `HybridSearchInput.oversample`, wired in **both** NAPI and REST (parity per CLAUDE.md), so
  callers can dial latency vs recall.
- **P2a — F16 `Quant` variant.** 2× reduction, near-f32 recall, **no sidecar needed** — the
  sweet spot for mobile where the rerank-disk round-trip is least welcome.
- **P2b — SQ8 calibrated / quantile scale (optional).** Per-collection scale computed at
  compaction; trades async-insert agreement for recall on un-normalized / L2 vectors.
- **P2c — Ops surface.** Expose per-collection quant mode, sidecar resident bytes, and
  `index_lag` in `/v1/status` (and the NAPI status surface) — credibility/observability,
  matching Qdrant's `/metrics`.

## Verification / How we will confirm

The fix is the RAM curve, so it must be measured, not asserted:

1. **RSS at 500k, before and after P0**, with rerank **on** and **off**, captured via the
   `industrial-audit` / `scientific-audit` harness
   (`cargo run --release --features bins --bin industrial-audit`; see [[run-bench-audit]],
   [[AUDIT--P33-RSS-QUANT-MATRIX]]). Acceptance: after P0, BQ-rerank-on resident reduction
   approaches the rerank-off figure (target ≫ 1.9×, toward the ~32× / ~4× arena limits net
   of the unavoidable disk-side f32 and any LRU cache), confirming the sidecar no longer
   dominates resident bytes.
2. **Recall must not regress.** Re-run the bge-m3 recall matrix (n=3000, k=10, ef=200,
   exact-L2 ground truth). Acceptance: BQ+rerank ≥ **0.9655** and SQ8+rerank ≥ **0.9875**
   (the current measured values) — P0 changes *where* the f32 vectors live, not the
   re-score math, so recall must hold to within noise.
3. **Latency budget.** Record rerank p50/p99 before/after; the positioned-read pool adds
   disk I/O on the rerank path, so confirm the added latency is bounded and the LRU cache
   keeps hot candidates resident.
4. **Cross-surface parity & correctness.** `tests/rerank_tests.rs` (reload round-trip,
   compaction survival, no-sidecar control, missing-sidecar degradation) must stay green
   under the disk-backed sidecar; run with `--no-default-features` on Linux CI per the
   core/napi split.

## Related Links

- [[ADR--GENESISDB-VECTOR-QUANTIZATION]] — the decision this RCA re-examines (Status / Action
  Items 4/5/6 / BQ Outcome block require the staleness corrections noted above).
- [[ADR--GENESISDB-NODE-ID-INTERNING]] — the other scale-ceiling lever (node bookkeeping)
  named alongside vector RAM in the ADR's Context.
- [[PLAN--VECTOR-QUANTIZATION-REFINEMENT]] — the companion corrective plan (P0–P2c).
- [[ROADMAP]] — MARK XV / XVI scale and mobile goals that depend on the RAM win.
- [[project_mark_xv_rss500k]] — MARK XV P1 RSS + recall measurements that surfaced this.
- [[AUDIT--P33-RSS-QUANT-MATRIX]], [[REPORT--2026-06-24-MARK-XV-P1-RSS-DISK-TH]] — the recall
  matrix tables quoted in Evidence.
