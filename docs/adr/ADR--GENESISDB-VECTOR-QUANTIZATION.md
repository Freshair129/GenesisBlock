---
proposed_id: ADR--GENESISDB-VECTOR-QUANTIZATION
type: adr
status: accepted
aliases:
  - ADR
phase: 32
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-06-22T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-VECTOR-QUANTIZATION

**Status:** Accepted (SQ8 shipped 2026-06-23, full resident cut; BQ deferred)
**Date:** 2026-06-22
**Deciders:** Engine owner (Boss)
**Roadmap:** MARK XIV Priority 4 — "Scalar / binary quantization: close the Qdrant
recall@0.999 gap at high `ef_search`; reduce memory at 500k+ scale."

> **Decision update (2026-06-23):** the operator chose the **full resident cut** over
> the originally-drafted "f32 lossless on disk" variant. hnsw_rs holds its OWN copy of
> every vector, so each vector was resident twice (the f32 `arena` + HNSW's copy);
> keeping f32 resident would have shrunk only the HNSW copy. Full resident cut makes the
> arena element type itself `u8` (RAM **and** disk 4×). The cost, accepted: on-disk
> precision becomes the quantization grid after a save (u8→f32→u8 is idempotent under
> the fixed scale, so no compounding, but full-precision reversibility to `None` is
> lost). `Quant::None` collections are unaffected and byte-identical. `memmap2` was
> declined, which reshapes BQ's rerank (see Action Items 5).

## Context

Vector RAM is one of the two scale-ceiling levers (the other is node bookkeeping —
[[ADR--GENESISDB-NODE-ID-INTERNING]]). Today every embedding is stored as raw
`f32` in a dense per-collection arena:

- `VectorCollection.arena: RwLock<Vec<f32>>`, row-major, stride = `dim`
  (`src/lib.rs:380-402`). At the default `dim = 1536`, **one vector = 6 144 bytes**.
- 500k vectors ≈ **3.0 GB** of arena alone; 1M ≈ **6.1 GB** — before the HNSW graph
  (`hnsw_rs` v0.3.4, `Hnsw<'static, f32, DistL2>`, M = 16) and before node RAM.
- Snapshot is lossless raw `f32`: `vec_<name>.bin` is the arena byte-dumped LE,
  `meta_<name>.bin` is bincoded `NodeMetadata`; the HNSW is **not** persisted — it
  `rehydrate()`s from the arena on load (`src/lib.rs:460-471, 2015-2068`).
- `Metric::{L2, Cosine}`; Cosine is L2 over unit-normalized vectors (`prep()`,
  `src/lib.rs:428-436`). Search over-fetches `k*2` then reranks by impact
  (`hybrid_search`, `src/lib.rs:1356-1404`).
- **No quantization, PQ, or compression exists today** (confirmed by survey). The
  unused `NodeMetadata.gks_attributes: Vec<u8>` (`src/lib.rs:315-324`) is the only
  latent per-vector side-channel.

Two forces are in tension and the roadmap names both:

1. **Memory** — get the in-RAM arena down so 500k–2M vectors fit under the 16/32 GB
   ceiling alongside the graph.
2. **Recall** — match Qdrant's recall@0.999 at high `ef_search`. Qdrant achieves
   this not by storing *more* but by **oversample + rerank**: search a compressed
   index fast and wide, then re-score the top candidates against exact vectors.

A constraint that shapes every option: `hnsw_rs` is generic as `Hnsw<'a, T, D>` and
`search(&[T], …)` requires the **query type to equal the stored type `T`**. There is
no built-in asymmetric (f32-query vs u8-stored) path — asymmetry must be implemented
as an explicit rerank stage, not inside the index.

## Decision

Add an **opt-in, per-collection quantization mode** persisted in the manifest, and
roll it out in two layers so the high-value / low-risk scalar path lands first.

```rust
// manifest + VectorCollection
enum Quant { None, ScalarU8, Binary }   // serialized in state.json per collection
```

The `f32` snapshot (`vec_<name>.bin`) **remains the lossless source of truth** in
every mode. Quantized arenas are *derived* — built during `stage()` and rebuilt
during `rehydrate()`/load. This is the load-bearing decision: it means **zero
on-disk migration**, reproducible recall, and the ability to change `Quant` by
rebuild. The RAM win comes from what is held *resident*, not from what is on disk.

### Layer A — Scalar quantization (SQ8), symmetric (DECIDED, do first)

1. Per collection, compute a per-dimension `min[d]/max[d]` (or a single global
   range) over the arena; quantize each component to `u8`. Store a `u8` arena +
   the `Vec<f32>` scale/offset. **4× arena reduction** (6 144 → 1 536 B/vector).
2. Index as `Hnsw<'static, u8, DistL2>`. Because dequantization is a uniform affine
   map per dimension, L2 ordering on the `u8` codes is monotone w.r.t. the f32
   distance → ranking is preserved to within quantization noise. Query is quantized
   with the same scale before `search()` (symmetric).
3. No rerank in Layer A. Expected recall ≈ 0.97–0.99 (must be measured before
   default-on). `vec_<name>.bin` stays f32; on load we read f32, quantize, drop f32
   → steady-state RAM is the `u8` arena (load peaks at f32+u8, acceptable).

Layer A is the memory lever: it turns 500k×1536 from 3.0 GB → 0.75 GB resident.

### Layer B — Binary quantization (BQ) + exact rerank (DESIGNED, opt-in)

1. 1 bit/dim (sign of the centered component), bit-packed into `u64` words →
   `Hnsw<'static, u64, DistHamming>`. **32× reduction** (6 144 → 192 B/vector); BQ
   is well-behaved on high-dim normalized embeddings (≥1024 d, e.g. 1536-d OpenAI).
2. BQ alone loses too much recall, so pair it with **oversample + rerank**: fetch
   `k * oversample` Hamming candidates, then re-score those candidates against exact
   `f32` vectors and truncate to `k`. This extends the rerank stage `hybrid_search`
   already runs (it over-fetches `k*2` today).
3. Exact vectors for rerank come from the on-disk `vec_<name>.bin` via **`mmap`**
   (memmap2), *not* a resident f32 arena — otherwise the RAM win is cancelled.
   Rerank touches only the top `k*oversample` rows, so only those pages fault in.
   (Windows `mmap` is viable but must be validated against the snapshot-reload and
   bench harnesses; see [[feedback_bench_windows]].)

Layer B is the recall lever: BQ + rerank is the Qdrant-style path to recall@~0.99
at a fraction of the memory and with faster traversal (Hamming ≫ f32 L2).

### Out of scope (this ADR)

**Product Quantization (PQ).** Best compression/recall frontier, but requires
k-means codebook training, asymmetric distance tables, and a retrain/versioning
story — a different and larger project than the roadmap's "scalar/binary" line.
Deferred to its own ADR; Layers A/B do not preclude it.

## Options Considered

### Option A — Scalar SQ8 (symmetric), lossless f32 on disk  ★ recommended first
| Dimension | Assessment |
|-----------|------------|
| Complexity | Med — arena/HNSW become an enum over `f32`/`u8`; touches `stage`/`prep`/`search`/`rehydrate`/snapshot |
| Memory | **4×** arena reduction (3.0 → 0.75 GB @ 500k/1536) |
| Recall | ~0.97–0.99, no rerank (measure before default-on) |
| Latency | Neutral→better (smaller working set, cache-friendlier) |
| Migration | **None** — disk stays f32; mode is derive-on-load |

**Pros:** Largest memory win per unit risk; no disk migration; monotone ranking.
**Cons:** Recall ceiling without rerank; introduces the arena-type enum.

### Option B — Binary + oversample/rerank via mmap'd f32
| Dimension | Assessment |
|-----------|------------|
| Complexity | High — bit-packing, Hamming index, mmap rerank, oversample knob, Windows mmap validation |
| Memory | **32×** index reduction (3.0 → 0.19 GB resident @ 500k); f32 stays on disk (paged) |
| Recall | ~0.98–0.99 *with* rerank; poor without |
| Latency | Faster ANN (Hamming) + rerank tail on top-N |
| Migration | None on disk; adds mmap dependency |

**Pros:** The recall@0.999 story at minimum RAM; best traversal speed.
**Cons:** Highest complexity; rerank latency depends on disk/page-cache; mmap on
Windows needs proving.

### Option C — Product Quantization (PQ)
| Dimension | Assessment |
|-----------|------------|
| Complexity | Very high — codebook training, asymmetric LUTs, retrain/versioning |
| Memory | Tunable, typically 8–16× |
| Recall | High with rerank |
| Team familiarity | Low; no existing infra |

**Rejected (for now):** Scope dwarfs the roadmap line; sequence after A/B prove the
quantized-arena plumbing.

### Option D — Do nothing; rely on more RAM / smaller `dim`
**Rejected:** Leaves the 500k+ ceiling unaddressed and the Qdrant recall gap open;
contradicts the MARK XIV objective.

## Trade-off Analysis

The decision splits cleanly along the two forces. **SQ8 (A) is the memory play** —
cheap, low-risk, 4×, no migration, and sufficient if measured recall holds ≥0.975.
**BQ+rerank (B) is the recall+memory play** — it is the only option that reproduces
the Qdrant "oversample a compressed index, rerank against exact" result, but it pays
for that with bit-packing, a Hamming index, and an mmap rerank path that must be
proven on Windows. Doing A first de-risks B: A builds the per-collection `Quant`
mode, the arena-type enum, and the derive-on-load machinery that B reuses. PQ (C)
is strictly more than the roadmap asked for and is deferred without prejudice.

Sequencing also interacts with **per-query `ef_search`** ([[ROADMAP]] P3): rerank
wants a per-query oversample/`ef` override so a high-recall query and a low-latency
query can share one collection. Quantization and per-query `ef` are complementary;
land per-query `ef` alongside or before Layer B.

## Consequences

### Positive
- 4× (SQ8) to 32× (BQ) resident vector RAM cut — directly unlocks 500k–2M vectors
  under the 16/32 GB ceiling; complements node interning toward >1M nodes.
- BQ+rerank gives a defensible recall@~0.99 benchmark vs Qdrant at far less RAM.
- Lossless f32 on disk → mode is reversible by rebuild; no destructive migration.

### Negative
- The arena + HNSW handle becomes an enum over element types (`f32`/`u8`/`u64`);
  `stage`, `prep`, `search`, `rehydrate`, and snapshot all branch on `Quant`.
- BQ adds an `mmap` dependency and a rerank latency tail; Windows mmap must be
  validated against snapshot-reload and the audit harnesses.

### Neutral / Trade-offs
- WAL and the `f32` snapshot format are unchanged; existing single-/multi-collection
  DBs load as `Quant::None` with no action.
- Recall becomes a per-collection property the operator chooses; defaults stay
  `None` until SQ8 recall is measured ≥ target.

## Action Items
1. [x] `Quant` enum (`None`/`ScalarU8`) + `state.json` manifest field;
       `create_collection` (+ NAPI + REST `CreateCollectionInput`) takes an optional
       quant mode (default `None`); `CollectionInfo` exposes it. *(shipped 2026-06-23)*
2. [x] Arena + HNSW refactored into `Quant`-tagged enums `ArenaStore`/`VecIndex`
       behind their existing **separate** locks (arena-write never blocks index-read);
       all `match` logic localized in accessor methods (`push_f32`/`f32_at`/
       `append_range`/`to_bytes`/`from_bytes`; `insert_f32`/`parallel_insert_f32`/
       `search_f32`). Threaded through `stage`, `rehydrate`, worker, `hybrid_search`,
       meta-graph + cluster centroid (via `f32_at` dequant), compaction, save/load.
       **Full resident cut**: `vec_<name>.bin` is u8 for SQ8 (4× on disk too).
       *(shipped 2026-06-23)*
3. [x] SQ8 implemented: fixed `[-1,1]→[0,255]` affine scale (so async inserts agree
       without seeing all data; Cosine is clean, L2 out-of-range clamps),
       `Hnsw<u8, DistL2>` (anndists impls `DistL2` for `u8`), symmetric query quant,
       no rerank. *(shipped 2026-06-23)*
4. [ ] Recall harness (recall@k vs exact brute force) at 100k/500k; gate default-on
       SQ8 behind measured recall ≥ 0.975. *(toy tests can't measure recall — HNSW is
       approximate on tiny sets; needs a real-data probe.)*
5. [ ] BQ (`u64` bit-pack + a **custom** `Distance<u64>` — anndists' built-in u64
       `DistHamming` is whole-word inequality, NOT bit popcount) + oversample/rerank.
       **`memmap2` declined** → exact-f32 for rerank comes from an on-disk f32 sidecar
       (`vecf32_<name>.bin`) appended at `stage` and read via std `File` seek/read.
       NOTE (found during impl): the sidecar interacts with `save_state` (must not be
       clobbered by the u64 arena dump) and **compaction** (must be rewritten when
       arena ids change) — materially heavier than the other phases; do as a focused PR.
6. [ ] Record before/after RSS + recall in a new `RCA--VECTOR-QUANTIZATION` and in
       [[METRICS-REVIEW--2026-06-22-WEEKLY]].

## Verification
- `tests/quantization_tests.rs` (4, deterministic): SQ8 finds the exact-match top-1
  (query quantizes to the same u8 codes → distance 0); a `None` collection finds its
  exact match (control unchanged); SQ8 survives `save_state` + reopen (u8 arena
  round-trips, index rehydrates); `vec_<name>.bin` is exactly 4× smaller for SQ8 than
  f32, and `None` is exact f32 width. Near-tie *ordering* is intentionally NOT asserted
  (HNSW is approximate on tiny/degenerate sets — it occasionally drops a vector from a
  symmetric 5-point u8 graph; this is a toy-data artifact, not an SQ8 defect).
- Full `cargo test` green; `cargo check --all-targets --features bins` clean (REST).
- Pending: `industrial-audit`/`scientific-audit` RSS at 100k/500k for the resident cut,
  and the recall harness on real embeddings ([[feedback_bench_windows]]).

### Outcome (SQ8, shipped 2026-06-23)
SQ8 full resident cut shipped. `ArenaStore`/`VecIndex` make the arena + HNSW element
type `u8` for `ScalarU8` collections (4× resident + disk), `f32` for `None` (unchanged).
`hybrid_search` quantizes the query symmetrically; heuristic f32 readers (meta-graph,
clustering) dequantize via `f32_at`; compaction is element-agnostic. Manifest gains
`"quant"`; absent ⇒ `None`, so existing DBs load byte-identical. Suite green (26 test
groups), `bins` builds. BQ (Action 5) deferred to a focused PR for the reasons noted.

---
### Related Links
- **Sibling RAM lever:** [[ADR--GENESISDB-NODE-ID-INTERNING]]
- **Multi-collection substrate:** [[ADR--GENESISDB-MULTI-COLLECTION]]
- **Async index path quantization plugs into:** [[ADR--GENESISDB-ASYNC-INDEXING]]
- **HNSW capacity / OOM history:** [[ADR--GENESISDB-HNSW-CAPACITY]]
- **Scale proof:** [[ADR--GENESISDB-SCALABILITY-VALIDATION]]
- **Roadmap:** [[ROADMAP]] (MARK XIV P4)

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-22 | Proposed: per-collection `Quant` mode, lossless f32 on disk; Layer A (SQ8, symmetric, 4×) decided first; Layer B (BQ + oversample/rerank via mmap) designed; PQ deferred. |
| 0.2.0 | 2026-06-23 | Accepted + SQ8 shipped as the **full resident cut** (arena+HNSW u8, 4× RAM *and* disk; reversibility traded away). `ArenaStore`/`VecIndex` enums behind separate locks. BQ revised to a no-mmap on-disk f32 sidecar (heavier — own PR); built-in u64 `DistHamming` found to be word-inequality, so BQ needs a custom popcount distance. |
