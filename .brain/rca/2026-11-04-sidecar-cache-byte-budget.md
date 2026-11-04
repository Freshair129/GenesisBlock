# RCA: sidecar decoded-row cache used a row-only bound

Date: 2026-11-05
Repository: `Freshair129/GenesisBlock`
Scope: PR #281 sidecar-cache byte accounting
Classification: confirmed source-level retained-memory bound gap; no runtime OOM or customer impact has been observed.

## Symptom

`SidecarReader` retained decoded `f32` rows in an LRU capped at 4,096 entries.
Collection dimension is independently admitted through 65,535, so the row
limit alone allowed up to `4,096 × 65,535 × 4 = 1,073,725,440` bytes of
decoded payload in one full reader cache, before LRU metadata, returned row
copies, allocator overhead, and other readers. This is a source-derived upper
bound, not an observed allocation or OOM.

## Root cause

The original insertion path evicted only when the row count reached
`SIDECAR_CACHE_ROWS`. `cache_bytes()` measured `Vec<f32>` capacity but had no
admission or eviction effect. A 1,024-dimension example in the source comment
made the 4,096-row cache appear to be about 16 MiB, although that is not a
dimension-independent limit.

## Evidence

- `src/lib.rs`: the engine accepts dimensions through 65,535 and the sidecar
  rerank path loads decoded rows through `SidecarReader::row`.
- Before the fix, `SidecarCache::put` enforced only the 4,096-entry limit;
  `cache_bytes()` summed actual vector capacities without controlling insertions.
- Hosted test-first run for test-only head
  `b96f36497045498e40ee7c648c10bc4d666adab2`, Ubuntu job `126718951950`, ran 73
  tests: 70 passed, two new byte-bound tests failed, and one existing test was
  ignored. The max-dimension test reported retained payload above 16 MiB; the
  individually oversized row was retained at 16,777,220 bytes where the
  expected cache payload was zero. The existing row-count and truncate tests
  passed.
- An earlier test attempt used write-only `File::create` fixtures, so four
  cases failed at `SidecarReader::row(...).unwrap()` before reaching their
  assertions. The fixture was corrected to use readable temporary files; that
  run is not counted as evidence of the cache behavior.
- No runtime OOM, production incident, customer impact, allocation profile,
  or unauthenticated remote reachability has been established.

## Corrective design

Retain the existing `SIDECAR_CACHE_ROWS = 4096` entry ceiling and add a
16 MiB per-reader decoded-row-capacity ceiling. Measure each retained row as
`Vec::capacity() × size_of::<f32>()`; evict least-recent rows until both limits
hold; do not cache a single row whose capacity exceeds the byte budget; keep
returning the decoded disk row; and clear byte accounting when truncate clears
the cache. Updating a cache hit must not change retained-byte accounting.

This bound covers retained decoded row capacity in one `SidecarReader` only.
It excludes the LRU map/deque, caller-owned and transient copies, other readers
or collections, allocator overhead, and total process RSS. The public
`SidecarReader::new` constructor's transient allocation for arbitrary caller
dimensions remains outside this change. No schema, WAL, file format, query
API, planner, authentication, or rerank arithmetic change is intended.

## Prevention and validation status

Add direct tests for maximum admitted dimension, byte-based LRU eviction,
the existing row-count cap, a row larger than the budget, exact returned row
values, cache-hit accounting, and truncate clearing. The test-first RED above
confirmed the byte-bound gap after correcting the fixture. Implementation
GREEN, full hosted CI, security-policy disposition, and independent whole-PR
review remain pending; none is implied by this RCA or the earlier vector-memory
measurements.
