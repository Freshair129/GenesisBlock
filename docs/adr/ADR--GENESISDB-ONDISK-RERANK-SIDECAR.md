---
proposed_id: ADR--GENESISDB-ONDISK-RERANK-SIDECAR
type: adr
status: accepted
aliases:
  - ADR
phase: 35
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-07-02T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-ONDISK-RERANK-SIDECAR

## Context

[[RCA--VECTOR-QUANTIZATION]] (confirmed) found that `f32_sidecar:
Option<RwLock<Vec<f32>>>` — the exact-f32 rerank buffer described in
[[ADR--GENESISDB-VECTOR-QUANTIZATION]] — is **resident**, not on disk. Every
rerank-enabled collection keeps a full flat `Vec<f32>` (arena_id `i` at
`[i*dim..(i+1)*dim]`) in RAM, parallel to the quantized arena. Because rerank
is mandatory for usable recall (BQ alone: 0.6845; BQ+rerank: 0.9655 — see
`project_mark_xv_rss500k` results), this sidecar is **94% of resident bytes**
in practice and cancels the RAM win the quantization ADR promised: BQ+rerank
measured 1.88× not the claimed 32×, SQ8+rerank 1.33× not 4×. The 2026-06-22
ADR entry already flags this — Option B (BQ) was accepted as "a no-mmap
on-disk f32 sidecar (heavier — own PR)" and never implemented; the earlier
mmap-based Option B design was explicitly declined (`memmap2` cited three
times in that ADR as "deferred"/not adopted, over Windows-safety concerns
never resolved). This ADR is that deferred PR's design gate.

The fix must move `f32_sidecar` off the resident heap onto disk, read via
**positioned reads**, while preserving durability, lock order, and the exact
degraded-fallback semantics the query path already relies on
(`src/lib.rs`, `hybrid_search` rerank block).

## Decision

Replace the resident `Vec<f32>` backing with a `SidecarReader` that opens a
file handle on `fvec_<name>.bin` and reads individual rows with **positioned
reads** — `std::os::windows::fs::FileExt::seek_read` on Windows,
`std::os::unix::fs::FileExt::read_at` on Unix, both behind `#[cfg]`. A small
bounded LRU page cache sits in front of the file to absorb hot rows.

### 1. Positioned reads, not `mmap` — and why

`mmap`/`memmap2` is **not used**, for two compounding reasons:

- **It's exactly what the quantization ADR already declined.** That ADR
  named `memmap2` as the mechanism for Option B's exact-rerank sidecar three
  separate times and never adopted it — deferred pending validation "against
  the snapshot-reload and Windows-mmap-file-locking edge cases" that was
  never done. Introducing it now would resurrect a decision this project has
  twice chosen not to make.
- **Windows safety.** `mmap`'d files on Windows hold an OS-level file mapping
  that commonly locks out rename/delete/truncate of the underlying file while
  the mapping is live (unlike POSIX, where an unlinked-but-mapped file keeps
  working). GenesisBlockDB's snapshot/compaction path **atomically replaces**
  on-disk files (`.tmp` write + rename — see Decision §5, and the existing
  compaction/save pattern for `vec_<name>.bin`/`meta_<name>.bin`). A live
  `mmap` on `fvec_<name>.bin` would risk the rename failing, or a torn read
  racing a reopen, on the exact platform this engine is developed and tested
  on (Windows 10, per `CLAUDE.md`). A mapped-but-superseded page is also a
  silent correctness hazard: nothing forces re-`mmap` after an atomic file
  swap, so a stale mapping can serve pre-compaction rows without error.
- **Positioned reads have none of this.** `seek_read`/`read_at` operate on an
  explicit byte range through a normal handle; they never hold a mapping open
  across a rename, they compose with the engine's atomic-replace pattern for
  free, and both are already the sanctioned primitive on this repo's target
  platforms — no new dependency, no `unsafe`.

The RAM cost of positioned reads is bounded by the LRU cache (§4), not by
file size, which is the whole point: the sidecar becomes O(cache) resident
instead of O(collection) resident.

### 2. Lock order: `meta → arena → sidecar`, preserved and declared

The engine's existing lock order for a `VectorCollection` — acquired in
`perform_index_compaction` today — is:

```
coll.metadata.write()  →  coll.arena.write()  →  coll.f32_sidecar.write()
```

This ADR does **not** change that order. Rules for the on-disk path:

- **Query-time reads never take the arena write lock.** A `SidecarReader::row()`
  call acquires only its own cache `Mutex` (guarding the LRU) plus the OS-level
  file handle (no explicit lock — `seek_read`/`read_at` are independently
  positioned per-call, so concurrent readers on the same `File` do not
  serialize on a Rust lock beyond the small cache critical section). It never
  acquires `coll.arena`'s lock, so rerank cannot contend with a concurrent
  compaction's arena mutation beyond the outer `RwLock<SidecarReader>` that
  replaces today's `RwLock<Vec<f32>>`.
- **The outer `RwLock<SidecarReader>` keeps its position in the order.**
  Wherever code today takes `f32_sidecar.write()` last (after `metadata` and
  `arena`), it continues to do so — swapping the reader handle during
  compaction (Decision §5 / P0-T5) happens under that same last-acquired
  write lock, so a reader mid-`row()` either completes against the old handle
  or blocks until the swap finishes; it never observes a half-swapped file.
- **The cache `Mutex` is strictly inside the `RwLock<SidecarReader>` scope**
  (i.e. `sidecar.read()/write()` is acquired first, the cache mutex second,
  and released before returning). No code path acquires the cache mutex and
  then reaches back out to `arena` or `metadata` — that would invert the
  order and is disallowed.
- Net effect: the on-disk path adds one new, strictly-nested lock (the LRU
  cache mutex) and does not introduce any new ordering relative to
  `meta`/`arena`/`sidecar`.

### 3. File layout

Unchanged from today's resident format — this is a **read-path** change, not
a format change:

- File: `fvec_<name>.bin`, one per rerank-enabled collection, same directory
  as `vec_<name>.bin` / `meta_<name>.bin`.
- Row `d_id` (an arena id, i.e. `embedding_offset / dim`) occupies byte range
  `[d_id*dim*4, (d_id+1)*dim*4)`.
- Each `f32` is little-endian, matching the existing `f32::from_le_bytes`
  convention (`src/lib.rs` load path, ~line 3710-3713) — no re-encode needed.
- No header, no per-row framing: pure flat array, `len_rows() = file_len /
  (dim*4)`.

### 4. Bounded LRU page cache

A fixed-capacity LRU (e.g. `SIDECAR_CACHE_ROWS` rows, hand-rolled — no new
crate dependency) sits inside `SidecarReader`, keyed by `d_id`, storing
decoded `Vec<f32>` rows:

- **Rationale:** rerank re-fetches the same over-fetched candidate rows
  within a query (oversample × top-k) and across nearby queries against a hot
  working set; a small cache absorbs that locality without re-introducing
  full residency. Capacity is a small constant independent of collection
  size, so worst-case resident bytes are `O(cache_rows * dim * 4)`, not
  `O(collection_size)` — the property this ADR exists to restore.
- **Bounded, not advisory:** eviction must actually occur at capacity (LRU
  discard), never grow past it. An unbounded or soft-bounded cache would
  recreate the exact resident-RAM problem this design fixes.
- **Placement:** per-`SidecarReader` instance (one cache per collection, not
  global), consistent with `VectorCollection` already being the unit of
  isolation for arena/HNSW/metric/dim.
- Cache misses fall through to a positioned read and populate the cache; a
  cache hit must be byte-identical to what a fresh disk read would return
  (verified by tests, not just assumed).

## Migration / back-compat

The on-disk byte layout is **unchanged** (§3), so no migration or rewrite is
needed for legacy resident-era `fvec_<name>.bin` files: `SidecarReader`
simply opens the existing file and reads it with the same offset math the
resident loader used to slice out of the in-memory `Vec`. A collection
written by the old resident code path and one written by the new on-disk
path are byte-identical on disk. The only migration is behavioral, at load
time: the "parallels the arena exactly" row-count guard (`v.len() ==
coll.arena.read().len()`, load path ~line 3714) is preserved as a
`len_rows() == arena_len` check; on mismatch the sidecar is left disabled
(`None`) so search degrades to quantized distances — the existing safety
behavior, not a new failure mode.

## Read pattern

Rerank never reads the whole `fvec_<name>.bin`. It reads exactly the
candidate set the caller asks it to rescore:

- **Normal path:** for each of the `fetch` over-fetched candidates, one
  `row(d_id)` call — `dim*4` bytes at `d_id*dim*4`, through the LRU cache.
  Total I/O per query is bounded by `fetch`, not by collection size.
- **Brute-force path (`exact_rerank_slots`, small collections):** when
  `fetch >= n` (the whole collection is the candidate set), the path still
  goes through `row()` per candidate — i.e. up to `n` positioned reads — not
  a raw whole-file slurp. For small collections this is cheap and cache-warm
  after the first query; the P0 implementation must confirm (and the P0-T9
  RSS/latency harness must measure) that this regime does not degrade into
  `n` cold disk reads per query. That measurement, not this ADR, is the gate
  for whether the brute-force regime needs an additional small-collection
  optimization (e.g. a one-time full-cache warm) — out of scope here.

## Consequences

### Positive
- Restores the RAM win the quantization ADR promised: sidecar resident bytes
  become `O(cache_rows)` instead of `O(collection_size)`, unblocking the
  4×/32× SQ8/BQ resident targets at 1M+ vectors.
- No file-format migration; existing databases load unchanged.
- No new third-party dependency, no `unsafe` beyond the already-`#[cfg]`-gated
  platform `FileExt` calls.
- Lock order is unchanged, so this does not introduce a new deadlock surface.

### Negative / Trade-offs
- Rerank now does real disk I/O per query instead of memory access; latency
  moves from "RAM slice" to "cache hit or positioned read." Must be measured
  against the P0-T9 RSS/latency harness, especially the brute-force regime.
- One more moving part (LRU cache + file handle lifecycle) that must be
  re-created correctly across load/compaction/reopen (P0-T4 through T7).
- Compaction must now rewrite `fvec_<name>.bin` by streaming rather than by
  mutating a resident `Vec`, adding an atomic-swap step to that path.

## Alternatives Considered
| Alternative | Reason Rejected |
|---|---|
| `mmap`/`memmap2` | Already declined twice by [[ADR--GENESISDB-VECTOR-QUANTIZATION]]; Windows file-locking/rename hazard conflicts with this engine's atomic-swap snapshot/compaction pattern; stale-mapping-after-swap is a silent correctness risk. |
| Keep resident `Vec<f32>` | The status quo — measured to cancel the quantization RAM win (94% of resident bytes); the whole reason this ADR exists. |
| Unbounded/no cache | Re-creates full residency under any workload that touches most of the collection; defeats the fix. |
| Global cache across all collections | Collections already have independent identity (arena/HNSW/metric/dim per `VectorCollection`); a per-instance cache keeps sizing and eviction local and simple, matching existing isolation. |

## Verification
- Docs-only ADR; no test. Downstream tasks (`tests/sidecar_reader_tests.rs`,
  `tests/rerank_tests.rs`, `tests/wal_compaction_tests.rs`) implement and
  verify this contract per `docs/PLAN--VECTOR-QUANTIZATION-REFINEMENT.md` P0.

---
### Related Links
- **Parent ADR:** [[ADR--GENESISDB-VECTOR-QUANTIZATION]]
- **Root Cause:** [[RCA--VECTOR-QUANTIZATION]]
- **Refinement plan:** `docs/PLAN--VECTOR-QUANTIZATION-REFINEMENT.md` (P0)
- **Precedent for async-thread + lock-order discipline:** [[ADR--GENESISDB-ASYNC-INDEXING]]

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-07-02 | Proposed & accepted: on-disk rerank sidecar via positioned reads (no mmap), lock order `meta → arena → sidecar` preserved, unchanged file layout, bounded LRU cache. |
