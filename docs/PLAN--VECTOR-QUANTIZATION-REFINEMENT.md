---
status: historical
---

# PLAN — Vector Quantization Refinement (Swarm Execution Plan)

**Status:** Shipped (all phases merged to `main` 2026-07-02, PRs #51–#58) · **Scope:** GenesisBlockDB engine (`src/lib.rs`, `src/router.rs`, `index.d.ts`) + tests + ADR
**Parent:** ADR--GENESISDB-VECTOR-QUANTIZATION (stale; rerank shipped resident, ADR still says "deferred")
**Root cause being corrected:** the f32-sidecar rerank arena ships as `pub f32_sidecar: Option<RwLock<Vec<f32>>>` (`src/lib.rs:843`) — fully heap-resident. It is loaded whole at open (`src/lib.rs:3708-3718`), written whole on save (`src/lib.rs:3572-3578`), and held **fully resident** for the collection's lifetime — a normal query slices only the over-fetched candidate rows, while the small-collection `exact_rerank_slots` brute-force path scans all live rows (`src/lib.rs:2595-2629`). The defect is residency, not per-query whole-reads. Verified per-vector resident RAM at 1536-dim: None = 12288 B; BQ rerank-off = 384 B (**32×**); **BQ rerank-on = 6528 B (only 1.88× vs None)** — the 6144 B sidecar is **94.1%** of resident bytes. The ADR (lines 107-108) explicitly warned a resident f32 rerank arena would cancel the RAM win and proposed mmap; `memmap2` was declined and the resident `Vec<f32>` shipped anyway. This plan moves the sidecar OFF-RAM via positioned disk reads (no mmap, Windows-safe) and adds the recall/ops refinements (BQ centering, per-query oversample, F16, SQ8 calibration, status exposure).

**Verified facts this plan is built on (independently code-checked):**
- f32 sidecar is fully resident; no mmap anywhere in GenesisBlockDB code (`mmap-rs` is only a transitive dep of `hnsw_rs`). — *confirmed*
- BQ uses raw `x > 0.0` sign with NO centering (`bq_pack`, `src/lib.rs:548-558`); BQ-alone recall = 0.6845 (catastrophic), BQ+rerank = 0.9655, SQ8+rerank = 0.9875 (= f32). — *confirmed*
- SQ8 uses a fixed global affine `SQ8_SCALE=SQ8_BIAS=127.5` (`src/lib.rs:522-539`); clamps L2/un-normalized values. — *confirmed*
- `RERANK_OVERFETCH = 8` is a compile-time constant with NO per-query/per-collection knob (`src/lib.rs:578`). `ef_search` *is* already a per-query knob on `HybridSearchInput` and is the template for the new `oversample` knob. — *partial→confirmed*
- ADR is stale: Status line says rerank "deferred", Action Items 4/5/6 unchecked, but rerank fully ships and is tested by `tests/rerank_tests.rs` (6 tests). — *confirmed*

---

## (a) Summary table — all tasks

| ID | Title | Model | Depends-on | Gate |
|----|-------|-------|------------|------|
| **P0-T0** | Sub-ADR: on-disk rerank sidecar design (positioned reads, no mmap) | Opus 4.8 | none | Opus 4.8 |
| **P0-T1** | `SidecarReader` positioned-read abstraction over `fvec_<name>.bin` | Opus 4.8 | P0-T0 | Opus 4.8 |
| **P0-T2** | Bounded LRU page cache for `SidecarReader` | Sonnet 4.6 | P0-T1 | Opus 4.8 |
| **P0-T3** | Swap `hybrid_search` rerank loop to `SidecarReader` (drop resident reads) | Opus 4.8 | P0-T1, P0-T2 | Opus 4.8 |
| **P0-T4** | `load()` stops slurping `fvec` into RAM; opens reader handle instead | Sonnet 4.6 | P0-T1 | Opus 4.8 |
| **P0-T5** | Compaction rewrites `fvec` by streaming, not via resident `Vec<f32>` | Opus 4.8 | P0-T1, P0-T4 | Opus 4.8 |
| **P0-T6** | `save_state` sidecar write: keep correct under the new model (verify/adapt) | Sonnet 4.6 | P0-T4, P0-T5 | Opus 4.8 |
| **P0-T7** | Migration / back-compat for pre-existing resident-era `fvec_<name>.bin` | Sonnet 4.6 | P0-T4 | Opus 4.8 |
| **P0-T8** | Tests: rerank recall parity + reopen round-trip + degraded-fvec | Sonnet 4.6 | P0-T3, P0-T4, P0-T5 | Opus 4.8 |
| **P0-T9** | RSS validation @500k, rerank on/off, resident vs on-disk (harness) | Sonnet 4.6 | P0-T3..T7 | Opus 4.8 |
| **P0-T10** | ADR de-stale: mark rerank shipped + record on-disk decision | Sonnet 4.6 | P0-T0 | Opus 4.8 |
| **P1a-T1** | BQ per-dim centering: compute per-dim mean at compaction | Opus 4.8 | P0 merged | Opus 4.8 |
| **P1a-T2** | Persist + load per-dim centering vector (`bqmean_<name>.bin`) | Sonnet 4.6 | P1a-T1 | Opus 4.8 |
| **P1a-T3** | Apply centering in `bq_pack` query + insert paths | Opus 4.8 | P1a-T1, P1a-T2 | Opus 4.8 |
| **P1a-T4** | Tests: BQ-alone recall lift on real bge-m3 (harness) | Sonnet 4.6 | P1a-T3 | Opus 4.8 |
| **P1b-T1** | `oversample: Option<u32>` on `HybridSearchInput` + resolution in `hybrid_search` | Sonnet 4.6 | P0 merged | Opus 4.8 |
| **P1b-T2** | NAPI+REST parity for `oversample` (`index.d.ts` + router passthrough) | Sonnet 4.6 | P1b-T1 | Opus 4.8 |
| **P1b-T3** | Tests: oversample knob (NAPI `.mjs` + REST `rest_api_tests.rs`) | Sonnet 4.6 | P1b-T2 | Opus 4.8 |
| **P2a-T0** | Design gate: f16 HNSW distance (native `DistL2<f16>` vs dequantize-on-insert) + `half` dep decision | Opus 4.8 | P0 merged | Opus 4.8 |
| **P2a-T1** | `Quant::F16` variant: enum + arena + HNSW + pack/unpack | Opus 4.8 | P2a-T0 | Opus 4.8 |
| **P2a-T2** | F16 snapshot/load + parse (`"f16"`) + `create_collection` plumbing | Sonnet 4.6 | P2a-T1 | Opus 4.8 |
| **P2a-T3** | Tests: F16 recall ≈ f32, round-trip, 2× RAM (harness) | Sonnet 4.6 | P2a-T2 | Opus 4.8 |
| **P2b-T1** | SQ8 calibrated/quantile scale computed at compaction (per-collection) | Opus 4.8 | P0 merged | Opus 4.8 |
| **P2b-T2** | Persist + apply SQ8 scale; default-off (back-compat fixed scale) | Sonnet 4.6 | P2b-T1 | Opus 4.8 |
| **P2b-T3** | Tests: SQ8 calibrated recall on L2/un-normalized vectors (harness) | Sonnet 4.6 | P2b-T2 | Opus 4.8 |
| **P2c-T1** | `/v1/status` + `status_sync`: per-collection quant, sidecar resident bytes, `index_lag` | Sonnet 4.6 | P0 merged | Opus 4.8 |
| **P2c-T2** | NAPI parity: expose same per-collection ops fields | Sonnet 4.6 | P2c-T1 | Opus 4.8 |
| **P2c-T3** | Tests: status exposes ops fields (REST + NAPI) | Sonnet 4.6 | P2c-T2 | Opus 4.8 |

---

## (b) Dependency / ordering overview

**Phase sequencing is strict: P0 lands as its own PR and merges to `main` BEFORE any P1/P2 task starts.** P0 changes the sidecar's residency contract (the data structure every later task touches), so all P1/P2 tasks depend on "P0 merged" to avoid rebasing onto a moving sidecar.

**Within P0 (the critical path):**
```
P0-T0 (design) ──┬──> P0-T1 (reader) ──┬──> P0-T2 (LRU cache) ──┐
                 │                      ├──> P0-T4 (load)        ├──> P0-T3 (search swap)
                 │                      │        └──> P0-T5 (compaction) ──┐
                 │                      │        └──> P0-T7 (migration)     │
                 │                      └────────────────────────> P0-T6 (save verify)
                 └──> P0-T10 (ADR de-stale, parallel)
P0-T3,T4,T5,T6,T7 ──> P0-T8 (correctness tests) ──> P0-T9 (RSS harness)
```
- **P0-T0 first** (design gate). Then **P0-T1 is the spine** — reader abstraction; nearly everything else depends on it.
- **P0-T2, P0-T4, P0-T7 parallelize** once T1 lands (cache, load, migration are independent surfaces).
- **P0-T3 and P0-T5 are the two correctness-critical Opus tasks**; T3 needs T1+T2, T5 needs T1+T4.
- **P0-T6** verifies save still emits a byte-correct `fvec`; depends on the final load/compaction shape.
- **P0-T8 → P0-T9** gate the whole PR: correctness before perf claim.
- **P0-T10** (ADR) runs in parallel from T0; no code dep.

**P1/P2 (after P0 PR merges) — three independent tracks parallelize:**
- **P1a** (BQ centering) → **P1a-T1 → T2 → T3 → T4** serial (math → persist → apply → measure).
- **P1b** (oversample knob) → **P1b-T1 → T2 → T3** serial; small, ships fast.
- **P2a** (F16), **P2b** (SQ8 calibration), **P2c** (status ops) are mutually independent and parallelize with each other and with P1a/P1b. Each is internally serial; **P2a leads with a design gate (P2a-T0)** that settles the f16 HNSW-distance / `half`-dep fork before P2a-T1 implements, then impl → persist/parity → test.

Recommended PR cadence: **PR1 = all of P0** (one reviewable unit, the residency fix). **PR2 = P1b** (smallest, unblocks ops tuning). **PR3 = P1a**. **PR4 = P2a**. **PR5 = P2b**. **PR6 = P2c**. P2c can move earlier if ops visibility is wanted before the recall work.

---

## (c) Orchestration flow

1. Each task is dispatched to its assigned executor model (Sonnet 4.6 or Opus 4.8) with **only its task description** (self-contained — no conversation context).
2. On completion, the diff goes to an **Opus 4.8 review gate** with the task's "Review gate" checklist. The reviewer must run the task's acceptance tests (`cargo test --no-default-features` for Rust on Linux/CI parity; `npm test` for NAPI/MCP) and confirm the named assertions exist and pass.
3. Reviewer **blocks** on any unmet review-gate item; executor iterates until the gate passes.
4. The orchestrator (main Claude) **integrates** passed tasks, runs the cross-task integration review (NAPI+REST parity sweep via the `napi-rest-parity` skill, full `cargo test` + `npm test`, and for storage/index changes the `run-bench-audit` harnesses), and assembles the PR.
5. **P0 PR merges to `main` before P1/P2 dispatch.** Each subsequent phase repeats the loop.
6. All Rust tests must pass under `--no-default-features` (core/napi split → Linux CI links with no `napi_*` symbols); NAPI surface tests require a local `npm run build` first (no prepare script).

---

# P0 — Move the rerank sidecar OFF-RAM (positioned disk reads, no mmap)

> Restores the full 4×/32× resident win (matches Qdrant's "compressed in RAM, original on disk, rescore top-N"). The over-fetched candidate rows are read at `offset = d_id * dim * 4` from `fvec_<name>.bin` via `seek + read_exact` (NOT mmap — mmap is exactly why `memmap2` was declined; positioned reads via `std::os::windows::fs::FileExt::seek_read` / `std::os::unix::fs::FileExt::read_at` are Windows-safe). A small bounded LRU page cache absorbs hot rows.

### P0-T0 — Sub-ADR: on-disk rerank sidecar design
- **Title:** Sub-ADR — on-disk f32 rerank sidecar via positioned reads
- **Parent refinement:** P0
- **Scope:** New file `docs/adr/ADR--GENESISDB-ONDISK-RERANK-SIDECAR.md` only. No code.
- **Why narrow:** Pure design artifact; defines the contract every P0 code task implements.
- **Complexity:** S
- **Executor model:** Opus 4.8 — sets the residency contract, I/O strategy, lock-order, and back-compat rules that the correctness-critical tasks must follow; getting this wrong cascades.
- **Depends-on:** none
- **Review gate (Opus 4.8) — must check:**
  1. The chosen API is positioned reads (`seek_read`/`read_at`), **not** mmap, and the doc states *why* (Windows safety, prior `memmap2` decline).
  2. Lock order is preserved/declared: the on-disk path must not introduce a lock that violates `meta → arena → sidecar`; reads hold a file handle + cache lock, not the arena write lock.
  3. Migration policy for legacy resident-era `fvec` files is specified (same byte layout → reuse file as-is; no rewrite needed).
  4. The over-fetch read pattern is defined: read only the `fetch` candidate rows (each `dim*4` bytes at `d_id*dim*4`), never the whole file; the brute-force `exact_rerank_slots` small-collection path is addressed (it may still read all N rows, but via the reader).
- **Acceptance criteria:** ADR file exists with Status `Accepted`, a "Decision", "Lock order", "Migration", and "Read pattern" section; no test (docs-only). Orchestrator confirms it cross-links ADR--GENESISDB-VECTOR-QUANTIZATION.
- **Risk / rollback:** None (doc). Rollback = delete file.

### P0-T1 — `SidecarReader` positioned-read abstraction
- **Title:** `SidecarReader` over `fvec_<name>.bin` (seek+read at `d_id*dim*4`, Windows-safe, no mmap)
- **Parent refinement:** P0
- **Scope:** `src/lib.rs`. Add a new struct `SidecarReader { file: File, dim: usize }` (place near `VectorCollection`, ~line 818). Method `fn row(&self, d_id: usize) -> Option<Vec<f32>>` reading exactly `dim*4` bytes at `offset = d_id*dim*4` using `#[cfg(windows)] FileExt::seek_read` / `#[cfg(unix)] FileExt::read_at` (gate both; no mmap). Method `fn len_rows(&self) -> usize` from file metadata `/ (dim*4)`. Change `VectorCollection.f32_sidecar` field type from `Option<RwLock<Vec<f32>>>` to `Option<RwLock<SidecarReader>>` is **out of scope for T1** — T1 only adds the struct + unit-coverage of the read math; field swap happens in T3/T4. Add a `fn write_rows` helper (append a `&[f32]` row to the file) used later by save/compaction.
- **Why narrow:** Pure I/O primitive with no callers changed yet; isolated and unit-testable.
- **Complexity:** M
- **Executor model:** Opus 4.8 — cross-platform positioned-read I/O with offset math; correctness of the byte arithmetic is load-bearing for all of rerank.
- **Depends-on:** P0-T0
- **Review gate (Opus 4.8) — must check:**
  1. No `memmap2`/`mmap`/`Mmap` anywhere; uses `seek_read`(win)/`read_at`(unix) behind `#[cfg]`.
  2. Offset math is exactly `d_id * dim * 4` and reads exactly `dim*4` bytes; partial-read handling (`read_exact`-style loop or `read_at` short-read retry) is correct.
  3. Out-of-range `d_id` returns `None`, never panics or reads past EOF.
  4. Little-endian decode matches the existing `from_le_bytes` convention (`src/lib.rs:3710-3713`).
- **Acceptance criteria:** New `tests/sidecar_reader_tests.rs`: write a known `fvec` of 5 rows × dim=4, assert `row(0)`, `row(4)` return exact f32 values, `row(5)` is `None`, and `len_rows()==5`. Run `cargo test --no-default-features --test sidecar_reader_tests`.
- **Risk / rollback:** Low — additive struct, no callers. Rollback = remove struct + test.

### P0-T2 — Bounded LRU page cache for `SidecarReader`
- **Title:** Small bounded LRU row cache in `SidecarReader`
- **Parent refinement:** P0
- **Scope:** `src/lib.rs` `SidecarReader` only. Add a fixed-capacity (e.g. 4096 rows, const `SIDECAR_CACHE_ROWS`) LRU keyed by `d_id` guarding the f32 row `Vec`s; `row()` checks cache before disk. Use a simple `Mutex<LruMap>` (hand-rolled or a tiny dep already in tree — prefer hand-rolled to avoid a new dependency). Cache is per-`SidecarReader` instance.
- **Why narrow:** Single file, single struct, behavior-preserving (cache hit must equal disk read).
- **Complexity:** M
- **Executor model:** Sonnet 4.6 — mechanical, well-scoped data structure; no cross-surface or lock-order concerns beyond the local cache mutex.
- **Depends-on:** P0-T1
- **Review gate (Opus 4.8) — must check:**
  1. Cache is bounded (eviction actually happens at capacity); no unbounded growth that re-creates the resident-RAM problem.
  2. A cached row is byte-identical to the disk read for the same `d_id`.
  3. The cache `Mutex` does not deadlock against the `RwLock` wrapping the reader (lock ordering documented).
  4. No new third-party dependency added to `Cargo.toml` without justification.
- **Acceptance criteria:** Extend `tests/sidecar_reader_tests.rs`: read the same `d_id` twice (assert equal), read > capacity distinct rows then re-read an evicted one (assert still correct from disk). `cargo test --no-default-features --test sidecar_reader_tests`.
- **Risk / rollback:** Low — cache is transparent. Rollback = bypass cache (read straight from disk).

### P0-T3 — Swap `hybrid_search` rerank loop to `SidecarReader`
- **Title:** Rerank loop reads candidate rows from disk, not the resident `Vec`
- **Parent refinement:** P0
- **Scope:** `src/lib.rs` `hybrid_search`, the rerank block at **lines 2595-2629**. Replace `sidecar.read()` resident slicing (`sc.get(start..start+dim)`) with `reader.row(d_id)`. Preserve current semantics exactly: missing row → keep quantized distance (degraded fallback, lines 2611-2624); re-sort ascending; `truncate(k2)`. Also update the `exact_rerank_slots` brute-force path (lines 2595-2607) to compute `n` from `reader.len_rows()` instead of `s.read().len()/dim`.
- **Why narrow:** One function, one block; pure substitution of the row source, no algorithm change.
- **Complexity:** M
- **Executor model:** Opus 4.8 — correctness-critical: this is the hot query path; a wrong offset or a broken degraded-fallback silently corrupts recall. Must preserve the deterministic brute-force regime.
- **Depends-on:** P0-T1, P0-T2
- **Review gate (Opus 4.8) — must check:**
  1. `d_id*dim` offset semantics match the prior resident layout exactly (same `embedding_offset` component units).
  2. Degraded fallback preserved: an absent/short row keeps the quantized distance (does NOT drop the candidate), matching old behavior (lines 2611-2624).
  3. The `exact_rerank_slots` small-collection brute-force path still triggers identically (`fetch >= n`) and stays deterministic.
  4. No `RwLock<Vec<f32>>` read of full sidecar remains in the query path; only per-candidate `row()` calls.
  5. **Brute-force regime latency is bounded.** When `exact_rerank_slots` triggers (`fetch >= n`, small collections) the path now issues up to `n` positioned reads per query — confirm those rows go through the LRU cache (P0-T2) and/or a small-collection threshold loads them once, so a small collection does not pay `n` cold disk reads on every query. This is the single most likely latency regression of the off-RAM move and MUST be measured in P0-T9.
- **Acceptance criteria:** Existing `tests/rerank_tests.rs` (6 tests) must still pass unchanged: `sq8_rerank_finds_exact_match`, `bq_rerank_distinguishes_magnitude`, reload round-trip, compaction survival, no-sidecar control, missing-sidecar degradation. Run `cargo test --no-default-features --test rerank_tests`. (These are the behavioral oracle — they must NOT be edited to pass.)
- **Risk / rollback:** Medium — hot path. Rollback = revert to resident read; gated behind P0 PR not yet merged.

### P0-T4 — `load()` stops slurping `fvec` into RAM
- **Title:** Open a `SidecarReader` handle on load instead of reading the whole `fvec` into a `Vec<f32>`
- **Parent refinement:** P0
- **Scope:** `src/lib.rs` load path **lines 3708-3718**. Replace the `fs::read(...).chunks_exact(4)...collect()` into `*sidecar.write()=v` with opening a `SidecarReader` (validate `len_rows() == coll.arena length / dim` for the "parallels the arena exactly" guard at line 3714; on mismatch leave sidecar `None`/disabled so search degrades to quantized, preserving current safety). Also adjust the `VectorCollection::new` sidecar field init (`src/lib.rs:858-862`) and the field type swap to `Option<RwLock<SidecarReader>>` if T3 did not already (coordinate: do the type swap here, T3 consumes it).
- **Why narrow:** Single load block; the validation guard semantics are preserved, only the storage backing changes.
- **Complexity:** M
- **Executor model:** Sonnet 4.6 — mechanical replacement following the T0/T1 contract; the tricky math lives in `SidecarReader`.
- **Depends-on:** P0-T1
- **Review gate (Opus 4.8) — must check:**
  1. No `fs::read` of the whole `fvec` remains; a file handle is opened, not the bytes slurped.
  2. The "parallels the arena exactly" guard (old `v.len()==arena.len()`) is preserved as a row-count check; mismatch disables the sidecar (degrades to quantized), never adopts a truncated file.
  3. Stage-time append still works: `stage()` (lines 921-949) appends to the sidecar — confirm the write path is reconciled (see P0-T6) or staged rows are appended to the file, not a dropped resident Vec.
  4. `VectorCollection::new` no longer allocates a resident `Vec<f32>` for the sidecar.
- **Acceptance criteria:** `tests/rerank_tests.rs` reload round-trip test passes; add to `tests/wal_compaction_tests.rs` or a new `tests/sidecar_ondisk_tests.rs` an assertion that after open of a quantized+rerank DB, a search returns the exact top-1 (proves reader path active). `cargo test --no-default-features --test rerank_tests` + the new test.
- **Risk / rollback:** Medium — touches open path. Rollback = revert to slurp.

### P0-T5 — Compaction rewrites `fvec` by streaming
- **Title:** Compaction streams live rows to a new `fvec` file, no resident `Vec<f32>`
- **Parent refinement:** P0
- **Scope:** `src/lib.rs` `perform_index_compaction` **lines 4024-4063**. Today it builds `new_sidecar: Vec<f32>` resident (line 4031) and writes back into the `RwLock<Vec<f32>>` (lines 4061-4063). Replace with: stream each live row from the old `SidecarReader` and append to a temp `fvec_<name>.bin.tmp` via `write_rows`, then atomically swap to the live file and reopen the reader. **Critical keying:** the existing loop iterates `meta_arena` in storage order and slices the old sidecar by `start_off = meta.embedding_offset`, in lock-step with `new_vec.append_range` (lines 4038-4047) — the streamed row source MUST be derived from `meta.embedding_offset / dim`, matching that keying, **NOT** a fresh `arena_id` counter, and the emitted row order MUST equal the `new_vec` append order exactly. (For fixed-dim collections `arena_id*dim == embedding_offset`, so keying off the wrong index silently desyncs the sidecar from the compacted arena while the load row-count guard still passes.) Preserve lock order `meta → arena → sidecar` (lines 4024-4027): hold the sidecar write lock while swapping the reader handle.
- **Why narrow:** One method, one loop; the live-set selection logic (lines 4034-4057) is untouched, only the sidecar materialization changes.
- **Complexity:** L
- **Executor model:** Opus 4.8 — lock-order-sensitive AND touches durable on-disk state; an incorrect swap under the meta/arena/sidecar locks risks deadlock or a torn `fvec`. Must keep arena-id remap and sidecar row order in lock-step.
- **Depends-on:** P0-T1, P0-T4
- **Review gate (Opus 4.8) — must check:**
  1. Lock order `meta → arena → sidecar` is preserved; the file swap + reader reopen happen under the held sidecar write lock (no read-while-rewriting window).
  2. Sidecar rows are emitted from `meta.embedding_offset / dim` (the same keying as `append_range`, lines 4038-4047), in the SAME order as `new_vec`/`new_meta` — NOT off a fresh `arena_id` counter. Rows must stay parallel to the compacted arena; the row-count load guard would pass even if rows were mis-ordered, so order/keying must be verified directly, not just by count.
  3. Atomic replace (write `.tmp` then rename) so a crash mid-compaction never leaves a partial live `fvec`; rename matches the existing `temp_dir` save pattern.
  4. On a `None` sidecar collection, the path is a no-op (no empty `fvec` created).
- **Acceptance criteria:** `tests/rerank_tests.rs` "compaction survival" test passes; extend it (or add to `tests/wal_compaction_tests.rs`) to: insert N, delete some, `perform_index_compaction`, assert reranked top-1 still exact AND `fvec` row count == live count. `cargo test --no-default-features --test rerank_tests` + `--test wal_compaction_tests`.
- **Risk / rollback:** High (durable state + locks). Rollback = revert to resident `new_sidecar` rebuild; covered by atomic `.tmp` so no on-disk corruption risk.

### P0-T6 — `save_state` sidecar write: verify/adapt under new model
- **Title:** Ensure `save_state` still emits a byte-correct `fvec_<name>.bin`
- **Parent refinement:** P0
- **Scope:** `src/lib.rs` `save_state` **lines 3572-3578**. Today it serializes the resident `RwLock<Vec<f32>>`. After T4/T5 the backing is a `SidecarReader` (file already on disk). Decide + implement one of: (a) if compaction/stage keep the live `fvec` always current on disk, `save_state` copies/links the current `fvec` into `temp_dir` (consistent with the atomic snapshot dir); or (b) stream rows from the reader into `temp_dir/fvec_<name>.bin`. Keep the manifest `"rerank": <sidecar present>` flag correct (line 3585).
- **Why narrow:** One save block; the surrounding manifest/meta/arena writes are untouched.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — mechanical once T4/T5 fix the residency; verifies a known-format byte write into the snapshot temp dir.
- **Depends-on:** P0-T4, P0-T5
- **Review gate (Opus 4.8) — must check:**
  1. The `fvec` landed in `temp_dir` is byte-identical to a freshly-streamed live sidecar (round-trips through load).
  2. Manifest `rerank` flag unchanged in meaning; absent ⇒ false on load.
  3. No resident `Vec<f32>` re-materialized just to write (would re-introduce the RAM spike during save).
  4. Snapshot atomicity preserved (writes into `temp_dir`, swapped by the existing save mechanism).
- **Acceptance criteria:** Reopen round-trip in `tests/rerank_tests.rs` passes; add an assertion that the saved `fvec_<name>.bin` size == `live_count * dim * 4`. `cargo test --no-default-features --test rerank_tests`.
- **Risk / rollback:** Low. Rollback = stream-write fallback.

### P0-T7 — Migration / back-compat for legacy `fvec` files
- **Title:** Open pre-existing resident-era `fvec_<name>.bin` transparently
- **Parent refinement:** P0
- **Scope:** `src/lib.rs` load guard (with T4). The legacy `fvec` byte layout (`flat le f32`, written at old lines 3575-3577) is IDENTICAL to what `SidecarReader` reads — so migration is "open as-is". Task = prove + document this: ensure the row-count validation (`len_rows()==arena_rows`) accepts an existing legacy file, and a DB written by the old resident build opens and reranks correctly with zero rewrite. Add a fixture or test that loads a pre-T0 sidecar layout.
- **Why narrow:** No format change; a back-compat assertion + fixture, no new code path beyond the guard.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — verification + fixture; the format is already compatible by construction.
- **Depends-on:** P0-T4
- **Review gate (Opus 4.8) — must check:**
  1. Legacy `fvec` (flat le-f32, no header) loads without rewrite and reranks correctly.
  2. The row-count guard does not reject a valid legacy file (off-by-one on `dim*4` division).
  3. A truncated legacy file still degrades to quantized (not adopted), matching old safety (line 3714).
- **Acceptance criteria:** New `tests/sidecar_migration_tests.rs`: hand-write a legacy-format `fvec` + matching arena/meta fixture, open, assert reranked search exact. `cargo test --no-default-features --test sidecar_migration_tests`.
- **Risk / rollback:** Low. Rollback = none needed (read-only compat).

### P0-T8 — Correctness tests: recall parity + reopen + degraded
- **Title:** On-disk rerank correctness suite
- **Parent refinement:** P0
- **Scope:** `tests/rerank_tests.rs` (extend) + new `tests/sidecar_ondisk_tests.rs`. Assert: (1) SQ8+rerank top-1 exact (parity with resident era); (2) BQ+rerank distinguishes magnitude; (3) save→reopen→search exact (reader survives round-trip); (4) compaction→search exact; (5) deleted/truncated `fvec` degrades to quantized (no empty result, no panic).
- **Why narrow:** Test-only; encodes the P0 behavioral contract.
- **Complexity:** M
- **Executor model:** Sonnet 4.6 — test authoring against a fixed contract; deterministic toy vectors (the existing rerank tests already use exact-match toy data, not approximate recall).
- **Depends-on:** P0-T3, P0-T4, P0-T5
- **Review gate (Opus 4.8) — must check:**
  1. Tests assert exact top-1 where the old resident tests did (no weakened assertions).
  2. Degraded-fvec path is exercised (delete the file mid-flight, assert quantized fallback not empty/panic).
  3. Tests run under `--no-default-features` (core/napi split, Linux CI).
  4. No test mutates the rerank algorithm to pass (oracle integrity).
- **Acceptance criteria:** `cargo test --no-default-features --test rerank_tests --test sidecar_ondisk_tests` all green; the 6 original `rerank_tests` assertions intact.
- **Risk / rollback:** None (tests). Rollback = drop new file.

### P0-T9 — RSS validation @500k, rerank on/off (harness)
- **Title:** Prove on-disk sidecar restores the resident RAM win at 500k
- **Parent refinement:** P0
- **Scope:** Reuse the existing audit harness (`cargo run --release --features bins --bin industrial-audit`, and the RSS/quant-matrix path documented in `docs/AUDIT--P33-RSS-QUANT-MATRIX.md`). Measure resident RSS at 500k for BQ rerank-on, resident (pre-P0) vs on-disk (post-P0), and SQ8 rerank-on. Record before/after in a new `docs/AUDIT--ONDISK-RERANK-RSS.md`. No engine code change.
- **Why narrow:** Measurement only; one harness invocation matrix + a results doc.
- **Complexity:** M
- **Executor model:** Sonnet 4.6 — runs the bins-gated harness on the Windows dev host and records numbers; the analysis target is known (expect BQ rerank-on RSS to drop from ~1.88× toward ~32× of None for the resident portion, sidecar no longer resident).
- **Depends-on:** P0-T3, P0-T4, P0-T5, P0-T6, P0-T7
- **Review gate (Opus 4.8) — must check:**
  1. RSS is measured via the engine's existing RSS probe (sysinfo, bins-only), not self-reported by the harness body.
  2. The on-disk BQ-rerank-on resident bytes ≈ BQ rerank-off (sidecar no longer counted resident); document the delta vs the verified 6528 B→384 B/vector target at 1536-dim.
  3. Recall@10 is re-confirmed unchanged (SQ8+rerank ≈ 0.9875, BQ+rerank ≈ 0.9655) — perf win must NOT cost recall.
  4. **Rerank latency is bounded, including the `exact_rerank_slots` brute-force regime** (P0-T3 item 5): record rerank p50/p99 before/after, and specifically size one collection so `fetch >= n` and assert its added per-query latency stays bounded (LRU-cached, not `n` cold disk reads). This regime is the named latency risk of the off-RAM move.
  5. Numbers are reproducible (command + seed recorded).
- **Acceptance criteria:** `docs/AUDIT--ONDISK-RERANK-RSS.md` with a before/after RSS table @500k and a recall re-confirmation row; harness command lines included. (Perf-relevant change ⇒ this harness gates the PR.)
- **Risk / rollback:** None (measurement). If RSS does NOT improve, P0 design is wrong → block PR, return to P0-T0.

### P0-T10 — ADR de-stale
- **Title:** Mark rerank as shipped + record on-disk decision in the quantization ADR
- **Parent refinement:** P0 (corrects ADR staleness)
- **Scope:** `docs/adr/ADR--GENESISDB-VECTOR-QUANTIZATION.md` only. Status line (18): change "f32-sidecar rerank deferred" → "shipped". Action Item 5 (220-229): `[~]`→`[x]`, remove "Oversample + rerank still deferred", note `fvec_<name>.bin` + `RERANK_OVERFETCH=8` + over-fetch/re-score path. BQ Outcome (261-263): remove "remains deferred" sentence; note tests/rerank_tests.rs green. Action Item 6 (231): record MARK XV P1 RSS+recall (SQ8 2.06×, BQ 2.93× RAM; recall SQ8+rerank 0.9875 / BQ-alone 0.6845 / BQ+rerank 0.9655). Add a forward pointer to the new on-disk sub-ADR (P0-T0).
- **Why narrow:** Doc-only edits to specified lines; no code.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — mechanical doc edit against an explicit line-by-line correction list.
- **Depends-on:** P0-T0
- **Review gate (Opus 4.8) — must check:**
  1. Status line no longer says "deferred"; Action Items 5/6 reflect shipped state.
  2. The on-disk sub-ADR is cross-linked.
  3. No claim contradicts code (e.g. don't claim mmap; the sidecar is positioned-read on-disk after P0).
  4. Recall/RSS numbers match the verified figures.
- **Acceptance criteria:** Doc diff only; orchestrator greps the ADR for "deferred" and confirms none remain on the rerank lines. No test.
- **Risk / rollback:** None. Rollback = revert doc.

---

# P1a — BQ per-dim centering (lift BQ-alone recall)

> BQ packs `x > 0.0` with no centering (`bq_pack`, `src/lib.rs:548-558`); bge-m3 dims are positive-biased, so many sign bits carry ~no information → BQ-alone recall 0.6845. Subtract a per-dim mean (computed at compaction) BEFORE the sign bit → better bit balance → higher BQ-alone recall. Standard "binary quantization with centering" (Faiss/Vespa/Qdrant ubinary).
>
> **Constraint (read first):** computing a per-dim mean needs an **f32 source**, which only exists when the collection carries the rerank sidecar (`VectorCollection::new`, `src/lib.rs:858`: sidecar allocated only when `rerank && quant != None`). A BQ collection created **without** rerank has a sign-only arena — the f32 is unrecoverable, so centering cannot be applied to it from stored data. Therefore P1a improves BQ quality **only for rerank-enabled BQ collections**. Note the measured 0.6845 "BQ alone" figure came from a harness where the f32 vectors were present; centering raises BQ's *traversal* quality (better sign-bit balance → fewer wasted candidates → cheaper/more accurate rerank for the rerank-enabled config), it does not retro-fit recall onto a sidecar-less BQ collection.

### P1a-T1 — Compute per-dim mean at compaction
- **Title:** Per-dim BQ centering vector computed during compaction
- **Parent refinement:** P1a
- **Scope:** `src/lib.rs` `perform_index_compaction` (BQ collections only) + `VectorCollection`. Add `pub bq_center: Option<RwLock<Vec<f32>>>` (len `dim`) to `VectorCollection`. During compaction of a `Quant::Binary` collection, compute the per-dim mean over the live arena's reconstructed/sidecar f32 values (use the sidecar reader if present; else skip centering — BQ-alone w/o sidecar still benefits only if f32 available; document this constraint). Store into `bq_center`.
- **Why narrow:** One method + one field; pure statistic computation, no query change yet.
- **Complexity:** M
- **Executor model:** Opus 4.8 — the centering math and the "what source vectors are available at compaction for a BQ collection" question are subtle (BQ arena is sign-only; mean needs f32 source = sidecar). Correctness of the statistic gates the recall win.
- **Depends-on:** P0 merged
- **Review gate (Opus 4.8) — must check:**
  1. Mean is over live rows only, per-dim, length == `dim`.
  2. The f32 source for the mean is the on-disk sidecar (post-P0) when present; behavior when absent is defined (skip centering, log).
  3. Lock order with the existing meta→arena→sidecar chain is preserved (bq_center written under compaction locks).
  4. No change to non-BQ collections.
- **Acceptance criteria:** Unit-ish integration test in new `tests/bq_centering_tests.rs`: build a BQ+rerank collection with a known positive-biased distribution, compact, assert `bq_center` ≈ per-dim mean. `cargo test --no-default-features --test bq_centering_tests`.
- **Risk / rollback:** Medium. Rollback = leave `bq_center = None` (falls back to uncentered, current behavior).

### P1a-T2 — Persist + load the centering vector
- **Title:** `bqmean_<name>.bin` snapshot + load
- **Parent refinement:** P1a
- **Scope:** `src/lib.rs` `save_state` (near 3572) + load (near 3700) + manifest. Write `bq_center` as flat le-f32 `bqmean_<name>.bin` when present; load it back; add a manifest flag/length. Absent ⇒ `None` (uncentered, back-compat).
- **Why narrow:** Mirrors the existing `fvec`/`meta` save+load blocks; pure serialization.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — mechanical, follows the established per-collection file persistence pattern.
- **Depends-on:** P1a-T1
- **Review gate (Opus 4.8) — must check:**
  1. Round-trips byte-exact; length == `dim`.
  2. Absent file ⇒ `None` ⇒ uncentered (old DBs unaffected).
  3. Written into `temp_dir` (snapshot atomicity) like the other per-collection files.
- **Acceptance criteria:** Extend `tests/bq_centering_tests.rs`: save→reopen, assert `bq_center` survives. `cargo test --no-default-features --test bq_centering_tests`.
- **Risk / rollback:** Low. Rollback = don't persist (recompute next compaction).

### P1a-T3 — Apply centering in pack paths
- **Title:** Subtract `bq_center` before the sign bit on insert + query
- **Parent refinement:** P1a
- **Scope:** `src/lib.rs` `bq_pack` callers: query path (`src/lib.rs:804-813`, BQ search), insert paths (`insert_f32` 757-760, `parallel_insert_f32` 778-785, `push_f32` 666-669). Either pass the center into a new `bq_pack_centered(emb, center)` or subtract before calling `bq_pack`. Query and insert MUST use the SAME center (else codes mismatch). When `bq_center` is `None`, behave exactly as today (uncentered).
- **Why narrow:** All callers of one packing function; semantics-preserving when center absent.
- **Complexity:** M
- **Executor model:** Opus 4.8 — must keep async-insert/query agreement (every insert path and the query path must subtract the identical center; a drift silently corrupts the index). Touches the async indexing insert paths.
- **Depends-on:** P1a-T1, P1a-T2
- **Review gate (Opus 4.8) — must check:**
  1. Query and ALL insert paths (single, parallel, arena push) apply the identical center; none missed.
  2. Centering changes the index — a collection's center must be stable post-compaction OR a re-pack of existing rows is triggered (define: center is fixed at compaction, applied to all subsequent inserts AND the compaction re-packs the arena/HNSW with the new center — verify the rehydrate uses the center).
  3. `None` center ⇒ byte-identical to current uncentered behavior (back-compat).
  4. Async-insert agreement preserved (no whole-distribution dependency at insert time beyond the persisted center).
- **Acceptance criteria:** `tests/bq_centering_tests.rs`: on a positive-biased toy set, assert centered BQ-alone (no rerank) returns the true nearest where uncentered does not. Plus a real-data recall check is deferred to P1a-T4. `cargo test --no-default-features --test bq_centering_tests`.
- **Risk / rollback:** Medium-High (index semantics). Rollback = force `bq_center=None`.

### P1a-T4 — BQ-alone recall lift (harness)
- **Title:** Measure BQ-alone recall uplift on real bge-m3
- **Parent refinement:** P1a
- **Scope:** Run the recall harness used for `docs/AUDIT--P33-RSS-QUANT-MATRIX.md` (n=3000, k=10, ef=200, real bge-m3, exact-L2 ground truth). Add a centered-BQ row to the quant matrix. Record in `docs/AUDIT--P33-RSS-QUANT-MATRIX.md` (or a new sibling). Target: BQ-alone recall meaningfully above the 0.6845 baseline.
- **Why narrow:** Measurement only.
- **Complexity:** M
- **Executor model:** Sonnet 4.6 — runs the harness, records the new row against the established matrix.
- **Depends-on:** P1a-T3
- **Review gate (Opus 4.8) — must check:**
  1. Same protocol as the existing matrix (n/k/ef/model/ground-truth) for comparability.
  2. Centered BQ-alone > 0.6845 (uncentered baseline); if not, centering is not helping → flag.
  3. BQ+rerank not regressed.
- **Acceptance criteria:** Updated audit doc with a "bq (centered)" recall row vs "bq (uncentered) 0.6845". Harness command recorded.
- **Risk / rollback:** None (measurement).

---

# P1b — Per-query oversample knob (NAPI + REST parity)

> `RERANK_OVERFETCH = 8` is a compile-time constant with no knob (`src/lib.rs:578`). Add `oversample` to `HybridSearchInput` mirroring the existing `ef_search` per-query override pattern. **This is a NAPI+REST parity task — the knob must be wired in BOTH `index.d.ts` and the REST route.**

### P1b-T1 — `oversample` field + resolution
- **Title:** `oversample: Option<u32>` on `HybridSearchInput`, resolved in `hybrid_search`
- **Parent refinement:** P1b
- **Scope:** `src/lib.rs`: add `pub oversample: Option<u32>` to `HybridSearchInput` (struct at 247-262, after `ef_search`). In `hybrid_search` `fetch` computation (lines 2581-2586), use `args.oversample.map(|o| o as usize).unwrap_or(RERANK_OVERFETCH)` as the multiplier. Keep `RERANK_OVERFETCH` as the default constant.
- **Why narrow:** One field + one expression; mirrors `ef_search` exactly.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — direct analog of the existing `ef_search` knob; mechanical.
- **Depends-on:** P0 merged
- **Review gate (Opus 4.8) — must check:**
  1. `None` ⇒ `RERANK_OVERFETCH` (default unchanged).
  2. Only affects the rerank `fetch` multiplier; no effect when no sidecar (the `else k2` branch untouched).
  3. Field placement keeps `#[cfg_attr(napi(object))]` derive valid.
- **Acceptance criteria:** Covered by P1b-T3. Compiles under `--no-default-features` and default.
- **Risk / rollback:** Low. Rollback = remove field.

### P1b-T2 — NAPI + REST parity passthrough
- **Title:** Expose `oversample` over both front-ends
- **Parent refinement:** P1b
- **Scope:** `index.d.ts` — add `oversample?: number` to `HybridSearchInput` (interface at line 116, beside `efSearch` at 134). `src/router.rs` — `hybrid_search_handler` (line 371) and `ranked_context_handler` deserialize `HybridSearchInput` directly, so the field flows automatically; **verify** both routes carry it and add no manual mapping is missing. Confirm the NAPI `hybridSearch` (index.d.ts:201) signature picks up the field (it's part of the object).
- **Why narrow:** Parity wiring only; the REST side is mostly automatic via the shared struct.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — parity check + a one-line `.d.ts` addition; use the `napi-rest-parity` skill.
- **Depends-on:** P1b-T1
- **Review gate (Opus 4.8) — must check:**
  1. `index.d.ts` `HybridSearchInput` has `oversample?: number` (NAPI surface) — drift check against the Rust struct.
  2. Both real routes — `/v1/search/hybrid` (`hybrid_search_handler`) and `/v1/reason/context` (`ranked_context_handler`) — accept `oversample`; they share `HybridSearchInput` via direct deser, so no per-route mapping is needed, only the `.d.ts` field add and a deser test.
  3. No REST handler silently drops the field.
- **Acceptance criteria:** Covered by P1b-T3.
- **Risk / rollback:** Low.

### P1b-T3 — Tests: oversample knob (NAPI + REST)
- **Title:** Oversample knob behavior tests on both surfaces
- **Parent refinement:** P1b
- **Scope:** `tests/rest_api_tests.rs` — POST hybrid search with `oversample` and assert 200 + sane results (and that a larger oversample on a quantized+rerank collection returns ≥ as-good top-1). `__test__/` — a NAPI `.mjs` test (e.g. extend an existing search test) passing `oversample` through `hybridSearch`.
- **Why narrow:** Test-only on both surfaces.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — straightforward request/response assertions on both transports.
- **Depends-on:** P1b-T2
- **Review gate (Opus 4.8) — must check:**
  1. REST test asserts the field is accepted (no 400/deser error) AND influences fetch (e.g. distinct from default on a constructed case).
  2. NAPI `.mjs` test passes `oversample` and asserts a valid result.
  3. Both run in the standard suites (`cargo test --no-default-features --test rest_api_tests`; `npm test`).
- **Acceptance criteria:** `cargo test --no-default-features --test rest_api_tests` + `node --test __test__/<file>.mjs` green.
- **Risk / rollback:** None (tests).

---

# P2a — `Quant::F16` variant (2×, near-f32 recall, no sidecar; mobile sweet spot)

### P2a-T0 — Design gate: f16 HNSW distance + `half` dependency decision
- **Title:** Decide native `DistL2<f16>` vs dequantize-on-insert, and whether to add the `half` crate
- **Parent refinement:** P2a
- **Scope:** Short design note (append to the F16 section of the quantization ADR or a design comment). Resolve two forks BEFORE P2a-T1 codes: (1) does `hnsw_rs`/`anndists` provide a `Distance` impl over `half::f16` so the HNSW can store f16 natively? If NOT, mandate the dequantize-on-insert contract (arena stores f16/u16; HNSW is handed f32 — note this means the HNSW copy stays f32, so resident RAM is arena 2× + HNSW 4×, unlike SQ8 where both shrink). (2) Add the `half` crate or hand-roll f16↔f32? Justify against the no-new-dep preference.
- **Why narrow:** Pure decision artifact that removes the unresolved fork inside P2a-T1; mirrors how P0-T0 de-risks P0.
- **Complexity:** S
- **Executor model:** Opus 4.8 — the distance/element pairing decision determines P2a-T1's entire shape; a wrong call invalidates the implementation mid-task.
- **Depends-on:** P0 merged
- **Review gate (Opus 4.8) — must check:**
  1. The `anndists`/`hnsw_rs` f16-distance question is answered with evidence (the crate's actual trait impls), not assumed.
  2. If no native f16 distance, the dequantize-on-insert contract is specified AND its RAM implication stated (arena 2×; HNSW copy still f32 → the F16 win is on the arena only, ~1.3× total resident at 1536-dim, not 2× — set expectations honestly).
  3. The `half`-dep-vs-hand-roll decision is justified.
- **Acceptance criteria:** A written decision (ADR section or design note) that P2a-T1 can implement without an open fork. No test.
- **Risk / rollback:** None (design). Rollback = revisit before coding.

### P2a-T1 — F16 enum + arena + HNSW + pack/unpack
- **Title:** Add `Quant::F16` end-to-end in the core type system
- **Parent refinement:** P2a
- **Scope:** `src/lib.rs`: `Quant` enum (492-516) add `F16`; `Quant::parse` accept `"f16"`/`"half"`; `as_str` ⇒ `"f16"`. `ArenaStore` (616-731) add `F16(Vec<u16>)` (store `half::f16` bits as `u16`, or a `half` crate type) — `byte_size`=2/elem, `push_f32`/`f32_at`/`append_range`/`to_bytes`/`from_bytes` arms. `VecIndex` (735-816) add `F16(Hnsw<'static, f16, DistL2>)` or reuse f32 distance on dequantized values (decide in T1 — prefer storing f16 but indexing with a DistL2 over f16 if `anndists` supports it; else dequantize-on-insert like SQ8). No sidecar (F16 is near-lossless).
- **Why narrow:** Single enum extended across its existing match arms; follows the SQ8/BQ pattern exactly.
- **Complexity:** L
- **Executor model:** Opus 4.8 — adds a new arena/index element type touching every `match` on `Quant`/`ArenaStore`/`VecIndex`; missing an arm is a silent correctness bug, and the f16↔f32 conversion + HNSW distance choice is non-trivial.
- **Depends-on:** P2a-T0 (the distance/element-pairing decision must be settled first)
- **Review gate (Opus 4.8) — must check:**
  1. EVERY match on `Quant`, `ArenaStore`, `VecIndex` has an `F16` arm (no `_ =>` swallowing it incorrectly).
  2. f16↔f32 round-trip is correct (`half` crate or manual), `byte_size`=2/elem, offsets stay in component units.
  3. HNSW distance for F16 ranks consistently with f32 (recall ≈ f32); decision (native f16 dist vs dequantize) is justified.
  4. No `f32_sidecar` allocated for F16 (it's near-lossless; `rerank` flag ignored or rejected for F16 — define).
- **Acceptance criteria:** Covered by P2a-T3; must compile under `--no-default-features`, default, AND `--no-default-features --features mobile` (F16 is the mobile sweet spot — verify it builds in the mobile feature set).
- **Risk / rollback:** Medium. Rollback = remove `F16` arm (additive). New `half` dep needs justification.

### P2a-T2 — F16 snapshot/load + create_collection plumbing
- **Title:** F16 persistence + creation surface
- **Parent refinement:** P2a
- **Scope:** `src/lib.rs` `from_bytes`/`to_bytes` F16 (done in T1, verify), manifest `quant` already string-driven (no change). `create_collection` (1214-1245) already takes `quant: Option<String>` → `Quant::parse` handles `"f16"`. `src/router.rs` `CreateCollectionInput` (61-69) + `create_collection_handler` (221-235) already string-driven — verify `"f16"` flows. `index.d.ts` `createCollection` (205) docstring mention `f16`.
- **Why narrow:** Mostly verification — the create surface is already string-generic; only persistence arms + docs.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — verification + doc; the generic string plumbing already supports a new quant name.
- **Depends-on:** P2a-T1
- **Review gate (Opus 4.8) — must check:**
  1. Creating a collection with `quant:"f16"` over BOTH NAPI and REST yields a `Quant::F16` collection (parity).
  2. F16 snapshot round-trips (save→load byte-exact, recall preserved).
  3. `CollectionInfo.quant` reports `"f16"`.
- **Acceptance criteria:** Covered by P2a-T3.
- **Risk / rollback:** Low.

### P2a-T3 — Tests: F16 recall ≈ f32, round-trip, 2× RAM
- **Title:** F16 correctness + RAM tests
- **Parent refinement:** P2a
- **Scope:** New `tests/f16_quant_tests.rs`: create f16 collection, insert toy vectors, assert top-1 exact (f16 is near-lossless on toy data); save→reopen→search exact; assert `arena.byte_size()` == `count*dim*2` (2× vs f32's *4). Optional harness row: F16 recall on bge-m3 ≈ f32 (extend the quant matrix audit).
- **Why narrow:** Test-only.
- **Complexity:** M
- **Executor model:** Sonnet 4.6 — test authoring against the new variant.
- **Depends-on:** P2a-T2
- **Review gate (Opus 4.8) — must check:**
  1. Top-1 exact on toy data (f16 lossless enough for distinct toy vectors).
  2. `byte_size` assertion proves the 2× footprint.
  3. Runs under `--no-default-features`; build also verified under `--features mobile`.
- **Acceptance criteria:** `cargo test --no-default-features --test f16_quant_tests` green; `cargo build --no-default-features --features mobile` succeeds.
- **Risk / rollback:** None (tests).

---

# P2b — SQ8 calibrated/quantile scale (optional, per-collection)

> SQ8 uses a fixed global affine `127.5/127.5` (`src/lib.rs:522-539`), clamping L2/un-normalized values (1.2 and 5.0 both → 255). A calibrated per-collection scale (quantile range computed at compaction) recovers recall on un-normalized vectors, at the cost of async-insert agreement (inserts before calibration use the old scale). **Default OFF** — back-compat keeps the fixed scale.

### P2b-T1 — Calibrated scale computed at compaction
- **Title:** Per-collection SQ8 quantile scale at compaction
- **Parent refinement:** P2b
- **Scope:** `src/lib.rs` `VectorCollection` add `pub sq8_scale: Option<RwLock<(f32, f32)>>` (scale,bias) — `None` ⇒ fixed `SQ8_SCALE/BIAS`. In `perform_index_compaction` for `Quant::ScalarU8` collections, compute a robust range (e.g. 1st/99th percentile of |component| from the sidecar f32 source, or arena dequantized) and derive (scale,bias). Refactor `sq8_q`/`sq8_dq` (525-539) to take an optional (scale,bias) param (keep the free-function constants as the default).
- **Why narrow:** One statistic + one field + parameterizing two small functions; gated to SQ8.
- **Complexity:** M
- **Executor model:** Opus 4.8 — the async-insert-agreement trade-off (inserts between calibrations use a different scale → must re-pack at compaction) is exactly the correctness subtlety the operator flagged; getting the calibrate→re-pack ordering wrong corrupts the index.
- **Depends-on:** P0 merged
- **Review gate (Opus 4.8) — must check:**
  1. Calibration source is the f32 sidecar (post-P0) when present; behavior without a sidecar defined (skip calibration).
  2. Compaction RE-PACKS the SQ8 arena + HNSW with the new scale so all rows share one scale (no mixed-scale codes).
  3. Subsequent async inserts use the persisted scale (read at insert time), preserving agreement until the next compaction.
  4. `None` ⇒ exact current fixed-scale behavior (back-compat, default).
- **Acceptance criteria:** New `tests/sq8_calibration_tests.rs`: an un-normalized (out-of-[-1,1]) toy set where fixed-scale clamps and loses ordering, but calibrated scale preserves top-1. `cargo test --no-default-features --test sq8_calibration_tests`.
- **Risk / rollback:** Medium-High. Rollback = `sq8_scale=None` (fixed scale, current behavior).

### P2b-T2 — Persist + apply calibrated scale; default-off
- **Title:** `sq8scale_<name>.bin` snapshot + default-off wiring
- **Parent refinement:** P2b
- **Scope:** `src/lib.rs` save (near 3572) + load (near 3700): persist `(scale,bias)` (8 bytes) when present; load back; manifest flag. Apply in `sq8_q`/`sq8_dq` call sites (push_f32 665, f32_at 677, insert_f32 754, parallel_insert_f32 773). A creation flag to opt-in (e.g. extend `create_collection`/`CreateCollectionInput` with `sq8_calibrate: Option<bool>`) OR auto-on only after first compaction — choose default-OFF and document.
- **Why narrow:** Serialization + applying an already-computed scale at the existing call sites.
- **Complexity:** M
- **Executor model:** Sonnet 4.6 — mechanical persistence + threading the scale through existing call sites once T1 defines the math; if a creation flag is added, that piece needs NAPI+REST parity (flag the parity in the review).
- **Depends-on:** P2b-T1
- **Review gate (Opus 4.8) — must check:**
  1. Round-trips byte-exact; absent ⇒ fixed scale (back-compat).
  2. ALL `sq8_q`/`sq8_dq` call sites use the per-collection scale (none left on the constant) when calibration is on; identical to constant when off.
  3. If an opt-in creation flag is added, it is wired in BOTH `index.d.ts`/`create_collection` AND `src/router.rs` `CreateCollectionInput` (parity).
- **Acceptance criteria:** Extend `tests/sq8_calibration_tests.rs`: save→reopen preserves calibrated ordering; with calibration off, SQ8 behaves byte-identically to pre-P2b. `cargo test --no-default-features --test sq8_calibration_tests`.
- **Risk / rollback:** Medium. Rollback = don't persist/apply (default-off path).

### P2b-T3 — Tests: calibrated recall on un-normalized vectors (harness)
- **Title:** SQ8 calibrated recall measurement
- **Parent refinement:** P2b
- **Scope:** Recall harness with an un-normalized / L2-metric embedding set (not the unit-normalized bge-m3 cosine set — fixed scale is already fine there). Compare SQ8 fixed vs SQ8 calibrated recall@10. Record in the quant-matrix audit doc.
- **Why narrow:** Measurement only.
- **Complexity:** M
- **Executor model:** Sonnet 4.6 — runs the harness with an un-normalized dataset; the win only shows on out-of-range data.
- **Depends-on:** P2b-T2
- **Review gate (Opus 4.8) — must check:**
  1. Dataset genuinely has components outside [-1,1] (else no clamping → no difference; would be a vacuous result).
  2. Calibrated SQ8 recall ≥ fixed SQ8 recall on that set; unit-normalized set unaffected.
  3. Protocol matches the matrix for comparability.
- **Acceptance criteria:** Audit doc row "sq8 (calibrated)" vs "sq8 (fixed)" on an un-normalized set; command recorded.
- **Risk / rollback:** None (measurement).

---

# P2c — Expose quant ops in `/v1/status` (ops/credibility; Qdrant `/metrics` parity)

### P2c-T1 — REST status: per-collection quant, sidecar resident bytes, index_lag
- **Title:** Extend `/v1/status` with per-collection quant ops
- **Parent refinement:** P2c
- **Scope:** `src/router.rs` `status_handler` (417-439) + its `ExtendedStatus` struct. Add a `collections: Vec<{ name, quant, count, sidecar_resident_bytes, index_lag }>` block. `sidecar_resident_bytes` is 0 after P0 (sidecar is on-disk) — report it explicitly to PROVE the win (and optionally a `sidecar_disk_bytes`). `index_lag` from `storage.index_lag()` (`src/lib.rs:1354`). Per-collection quant from `CollectionInfo` (`coll.info()` / `src/lib.rs:970`).
- **Why narrow:** One handler + one response struct; reads existing accessors.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — assembles existing getters into the status JSON.
- **Depends-on:** P0 merged
- **Review gate (Opus 4.8) — must check:**
  1. `sidecar_resident_bytes` reflects the POST-P0 reality (≈0 resident; on-disk reported separately) — the field exists to demonstrate the RAM win.
  2. `index_lag` uses the real `storage.index_lag()` accessor (async indexing backlog), not a stub.
  3. Per-collection quant string matches `CollectionInfo.quant`.
- **Acceptance criteria:** Covered by P2c-T3.
- **Risk / rollback:** Low. Rollback = remove the block.

### P2c-T2 — NAPI parity for status ops fields
- **Title:** Same per-collection ops fields over NAPI
- **Parent refinement:** P2c
- **Scope:** `src/lib.rs` the NAPI status accessor (`status_sync` 4405/4836 and any NAPI `status`/`collections` method) + `index.d.ts`. Ensure a NAPI caller can read per-collection quant + sidecar resident bytes + `index_lag` (NAPI already exposes `index_lag()` at 4716). Add/extend a NAPI-visible status object if `status_sync` (`DatabaseStatus`, 264-270) doesn't carry collections — likely add a `collections()` NAPI method returning `Vec<CollectionInfo>`-plus-ops, or extend `CollectionInfo`.
- **Why narrow:** Parity wiring of the same fields exposed in P2c-T1.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — parity task; use the `napi-rest-parity` skill to confirm both surfaces expose the same ops data.
- **Depends-on:** P2c-T1
- **Review gate (Opus 4.8) — must check:**
  1. The same three fields (quant, sidecar_resident_bytes, index_lag) are reachable over NAPI as over REST (no drift).
  2. `index.d.ts` regenerated/updated to match.
  3. `DatabaseStatus`/`CollectionInfo` derive attrs stay valid under `cfg_attr(napi(object))`.
- **Acceptance criteria:** Covered by P2c-T3.
- **Risk / rollback:** Low.

### P2c-T3 — Tests: status ops exposure (REST + NAPI)
- **Title:** Status exposes quant ops on both surfaces
- **Parent refinement:** P2c
- **Scope:** `tests/rest_api_tests.rs` — GET `/v1/status`, assert the response contains a `collections` array with `quant`, `sidecar_resident_bytes`, `index_lag` for a known collection. `__test__/` `.mjs` — assert the NAPI status/collections accessor returns the same fields.
- **Why narrow:** Test-only, both surfaces.
- **Complexity:** S
- **Executor model:** Sonnet 4.6 — assertion authoring.
- **Depends-on:** P2c-T2
- **Review gate (Opus 4.8) — must check:**
  1. REST test asserts all three fields present and typed correctly for a quantized+rerank collection (sidecar_resident_bytes ≈ 0 post-P0).
  2. NAPI test asserts parity.
  3. Runs in standard suites.
- **Acceptance criteria:** `cargo test --no-default-features --test rest_api_tests` + `node --test __test__/<file>.mjs` green.
- **Risk / rollback:** None (tests).

---

## Cross-cutting constraints (every task must respect)

- **`src/lib.rs` stays one file** — all engine changes land in it (only `src/query/` is split out by design). New structs (`SidecarReader`) go in `src/lib.rs`.
- **NAPI + REST parity** — any new query/creation field (P1b `oversample`, P2b opt-in flag, P2c status fields) MUST be wired in BOTH `index.d.ts`/NAPI and `src/router.rs`; the review gate explicitly checks drift (use the `napi-rest-parity` skill). Parity-critical tasks: **P1b-T2, P2b-T2 (if creation flag), P2c-T2**.
- **Async indexing** — insert-path changes (P1a-T3 centering, P2a-T1 F16 inserts, P2b SQ8 scale) must keep `add_node`/`execute_batch` enqueue semantics and async-insert agreement; `flush_index()` for read-your-write in tests.
- **Integration-only tests** — every new test is a `tests/*.rs` file (its own crate) or a `__test__/*.mjs`; no `#[test]` in `src/lib.rs`. Run Rust tests with `cargo test --no-default-features` (core/napi split, Linux CI parity).
- **Bins-gated harnesses** — all perf/recall measurement (P0-T9, P1a-T4, P2a-T3 optional, P2b-T3) uses `cargo run --release --features bins --bin industrial-audit` (or the relevant audit bin); never claim "no perf regression" without running them.
- **Windows dev host** — no iOS/Android builds locally; P2a F16 mobile-fitness is verified by `cargo build --no-default-features --features mobile` (compile only) and CI (`mobile-build.yml`), not device runs. Positioned reads (P0) MUST be Windows-safe (`seek_read`), which is the entire reason mmap was declined.
