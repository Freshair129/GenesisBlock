---
proposed_id: ADR--GENESISDB-HNSW-CAPACITY
type: adr
status: accepted
aliases:
  - ADR
phase: 35
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-06-22T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-HNSW-CAPACITY

## Context

Every per-collection HNSW index was built with a hardcoded reservation:

```rust
Hnsw::new(16, 1_000_000, 16, ef_construction, DistL2 {})   // max_elements = 1_000_000
```

`max_elements` is **not** a hard cap and **not** vector-data storage — it sizes
`hnsw_rs`'s per-layer pointer tables (`PointIndexation::new`, hnsw.rs:447-455)
via `Vec::with_capacity`. An earlier note (SPEC--MULTI-COLLECTION §1/§4) estimated
this at **~8 MB** and deprioritized it. **That estimate was wrong.** hnsw_rs's
layer-fraction formula (hnsw.rs:454) does not shrink geometrically across layers
as assumed; the reservation **compounds to >100 MB per index**. A backtrace of
the abort (`RUST_BACKTRACE=1`) caught it red-handed:

```
memory allocation of 133084264 bytes failed   (~127 MB, one layer table)
 9: hnsw_rs::hnsw::PointIndexation::new        hnsw.rs:455 (Vec::with_capacity)
11: VectorCollection::build_hnsw                lib.rs:391  Hnsw::new(16, 1_000_000, 16, ..)
12: VectorCollection::ensure_hnsw               lib.rs:397
```

This was latent under the old single global index, but **multi-collection made
it acute**: every collection lazily builds its own index, so N collections eagerly
committed N × >100 MB. Opening several DBs at once (e.g. parallel integration-test
threads) stacked the reservations into an out-of-memory `abort()` — surfacing on
Windows as the fast-fail exit `0xC0000409` (STATUS_STACK_BUFFER_OVERRUN). It
presented as a *flaky teardown crash* (intermittent, only under parallelism, after
tests passed) but was really an **intermittent OOM during the run**: single-threaded
never reproduced it; the failing allocation size varied (127–254 MB) because it was
whichever layer table happened to push the process over the commit limit.

## Decision

Size the HNSW reservation to the **actual data**, not a hardcoded million. hnsw_rs
appends points with a plain `Vec::push` under its internal write lock
(hnsw.rs:511), so the reservation is a soft hint — exceeding it just reallocates
(amortized). A small initial capacity is therefore safe at any scale.

```rust
const HNSW_MIN_CAP: usize = 1024;

fn build_hnsw(ef_construction: usize, cap: usize) -> Hnsw<'static, f32, DistL2> {
    Hnsw::new(16, cap.max(HNSW_MIN_CAP), 16, ef_construction, DistL2 {})
}
```

- **`ensure_hnsw` (lazy create, count unknown):** reserve `HNSW_MIN_CAP` (1024)
  and let inserts grow it on demand.
- **`rehydrate` (snapshot/WAL reload, count known):** reserve `meta.len()` exactly
  — no growth needed.

No reliance on a separate "rebuild at 2×" scheme (the spec's §4 alternative):
hnsw_rs's own `Vec::push` growth is correctness-safe (verified by reading the
crate), so the simple size-to-data form is sufficient.

## Consequences

### Positive
- A freshly-created index reserves ~tens of KB instead of >100 MB. Many
  collections (or many DBs) coexist without stacking huge idle reservations.
- The flaky OOM/`0xC0000409` is gone: full `cargo test` is clean across repeated
  runs at default parallelism (was ~30–50% crash rate before).
- Lower idle/small-collection memory footprint — directly benefits the
  multi-collection model (per-model spaces, agentic memory).

### Negative / Trade-offs
- A large load that starts from a lazily-created index grows the layer tables via
  reallocation (amortized doubling, ~log₂(N/1024) reallocs). The copy work is
  pointer-sized and negligible against HNSW graph construction; query latency is
  unaffected. Steady-state RAM at scale is unchanged (the tables fill regardless).

## Alternatives Considered
| Alternative | Reason Rejected |
|---|---|
| Keep `max_elements = 1_000_000` | The root cause; over-reserves >100 MB/index. |
| Rebuild index at 2× capacity from the arena on overflow (spec §4) | Unnecessary — hnsw_rs grows safely via `Vec::push`; adds insert-path complexity for no correctness gain. |
| Make tests run single-threaded | Hides a real over-allocation; doesn't fix production multi-collection footprint. |
| Join background threads / serialize teardown | Misdiagnosis — the crash was OOM during the run, not a teardown thread race; those changes were reverted. |

## Verification
- `tests/hnsw_capacity_tests.rs` (2): `many_collection_indexes_do_not_oom` opens
  64 collections (each an index + vector) in one process and searches each —
  deterministically OOM-aborted pre-fix (~8 GB of reservations), passes after;
  `index_grows_past_initial_floor` bulk-inserts 1500 vectors (> the 1024 floor)
  and asserts all are indexed (no loss on realloc) and the grown index still
  returns k neighbors.
- Full `cargo test` clean across 3 repeated full runs at default parallelism
  (exit 0; previously intermittent `0xC0000409`).

### Outcome (measured 2026-06-22)
Shipped. Per-index reservation cut from >100 MB to the data size; intermittent
OOM-abort under parallel/multi-collection load eliminated.

---
### Related Links
- **Vector spaces:** [[ADR--GENESISDB-MULTI-COLLECTION]]
- **Async indexing:** [[ADR--GENESISDB-ASYNC-INDEXING]]
- **Index design:** [[ADR--GENESISDB-HNSW-HYBRID-INDEXING]]

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-22 | Proposed & accepted & shipped: size HNSW reservation to data (`HNSW_MIN_CAP` + grow-on-demand) instead of hardcoded 1M; fixes >100 MB/index over-allocation and the intermittent OOM/`0xC0000409` under multi-collection/parallel load. |
