---
status: draft
---

# SPEC — Epoch-Segmented Indexes (epoch-HNSW / vector time-travel)

**Status:** Draft (2026-08-19) · **Funded by:** [DECISION--WP33-GNSE-BACKLOG](DECISION--WP33-GNSE-BACKLOG.md) (WP-3.3 gate, USER)
**Semantics authority:** [ADR--GENESISDB-JOURNAL-HISTORY](adr/ADR--GENESISDB-JOURNAL-HISTORY.md) (tx-time epoch, `history_horizon`, I6 horizon honesty) — this spec decides *mechanisms*, never overrides those semantics.
**Complexity:** C-3 (Doc → this Spec → Plan → Code) · **Scope (H-lock):** `src/lib.rs` (metadata, retraction paths, `hybrid_search`, `execute_query_ir`, compaction, snapshot), `docs/`, `tests/` · **Out of scope:** journal frame/segment byte format (unchanged), sync wire, valid-time semantics (unchanged), FTS axis.

> Terminology guard: "epoch" here = a frame-seq interval on the **tx-time** axis (per
> ADR--JOURNAL-HISTORY). It is unrelated to `EXPANSION-SPEC--GENESIS-DB.md`'s
> crossbeam epoch-based memory reclamation.

## 1. Problem (evidence-pinned)

`tx_as_of` (WP-2.2) is honest but **post-resolution**: candidates are enumerated from
*current* in-memory indexes, then `apply_tx_view` (`src/lib.rs:5774–5815`) rewrites each
survivor from its `node_versions` chain. Three consequences:

1. **Retracted nodes cannot be resurrected.** `retract_node_memory` removes the node from
   `nodes` (`lib.rs:8683`), `id_to_u32` (`:8675`), and adjacency (`:8641–8672`), so no
   candidate source ever offers it to the chain — pinned by the `#[ignore]`d TDD RED test
   `matrix_retraction_belief_before_still_serves`
   (`tests/bitemporal_matrix_wp31_tests.rs:190`). Capabilities disclose this as
   `"tx_as_of": "implemented_post_resolution"` (`lib.rs:7121`).
2. **Vector search has no tx axis.** `hybrid_search`'s only visibility gate is a live-map
   lookup (`if let Some(node) = self.nodes.get(&u32_id)`, `lib.rs:8090`); a retracted
   node's HNSW entry, arena slot, and metadata row all still exist but are silently
   dropped. "Vector top-k as this replica believed at commit N" is inexpressible.
3. **Compaction destroys history indiscriminately.** `perform_index_compaction`'s
   live-set filter (`lib.rs:10517`) discards every non-live arena row — even under
   `retention=full`, where the journal retains the history the vectors belong to.

## 2. Target contract

Within `history_horizon() <= t <= stable_frontier()` and under a history-retaining
profile (`full` / `budget:N`):

- **C1 (candidate completeness):** TRAVERSE and vector search with `tx_as_of = t` MUST
  enumerate every entity that existed at t — including nodes/edges retracted after t —
  then resolve each through its version chain at t (existing `apply_tx_view` rewrite).
- **C2 (no false positives):** entities first committed after t MUST NOT appear.
- **C3 (horizon honesty, unchanged):** `t < history_horizon()` keeps failing
  `beyond_horizon`; capabilities keep advertising `{history_horizon, tx_epoch_start}`.
- **C4 (default-profile neutrality):** under `FrontierOnly` (the default) behavior and
  cost are unchanged — history is already forfeited at every checkpoint there, and this
  feature MUST NOT tax the profile that doesn't use it.
- **C5 (capabilities upgrade):** `tx_as_of` advertises `"epoch_candidates"`; a new
  `vector_tx_as_of` capability advertises availability + the active retention profile.

## 3. Design

Three mechanisms, no journal format change (all derived, rebuildable state):

### 3.1 Epoch stamps on vector metadata (`meta_<name>.bin` v2)

`NodeMetadata` (`lib.rs:1059–1074`) gains two fields:

| Field | Meaning |
|---|---|
| `created_seq: u64` | frame seq of the commit that staged this row (from `persist()`'s returned seq, same source WP-2.3 uses for `caused_by`) |
| `retired_seq: u64` | 0 = live; else the frame seq of the retraction/orphaning commit |

- Snapshot encoding: manifest `mv: 1 → 2` (`lib.rs:9469–9471`); `decode_metadata_snapshot`
  extends its existing sniff-and-migrate ladder (`:1116+`, GBP1/bincode/V0) — v1 rows
  migrate with `created_seq = 0, retired_seq = 0` (pre-epoch rows: addressable as "always
  existed", consistent with `tx_epoch_start` floor semantics).
- `retract_node` stamps `retired_seq` in every collection's metadata row instead of
  leaving it untouched (today's `:8636` comment); `node_to_arena` removal stays (current
  write paths unaffected). u32→id recovery for historical candidates uses the
  `node_versions` projection, which already stores both (`node_u32`, `id`) per row
  (DDL `lib.rs:3024–3039`) — no new persistent mapping.
- Orphaned re-embed slots (`:2818–2820`) get `retired_seq` stamped at re-embed time,
  which also makes today's dedupe-by-node-id (`:8135–8141`) epoch-correct.

### 3.2 Retired-adjacency overlay (graph side — what turns the RED test green)

`retract_node` today hard-deletes incident edges from `edges`/`out_idx`/`in_idx`
(`:8641–8672`). Instead, move them into a **retired overlay**
(`out_idx_retired`/`in_idx_retired`: `DashMap<u32, HashSet<u128>>` + retained
`EdgeOutput`s tagged with the retracting frame seq):

- Current-view reads (`neighbors`, no `tx_as_of`) never consult the overlay — zero change.
- `tx_as_of = t` traverse enumerates `current ∪ {retired where retired_seq > t}`, then
  resolves per existing `apply_tx_view`; retracted seed/target nodes resolve via
  `node_versions` (id-addressable after retraction by design, `:5657–5658`).
- Lifecycle mirrors the tombstone registry: rebuilt deterministically from journal replay
  and CRDT reconcile (the `NodeRetract` event already carries what's needed); GC'd at the
  same boundary that folds history — when the fold advances `history_horizon()` past
  `retired_seq`, the overlay entry drops with it (C3/C4: under `FrontierOnly` every
  checkpoint folds, so the overlay stays empty).

### 3.3 Vector time-travel: filtered ANN + exact-scan floor (no second index)

For `hybrid_search` with `tx_as_of = t`:

- **Primary path:** `hnsw_rs` 0.3.4 already ships `search_filter` + `FilterT`
  (unused today; `VecIndex::search_f32` calls plain `.search()`, `lib.rs:1689–1740`).
  The epoch predicate is `created_seq <= t && (retired_seq == 0 || retired_seq > t)`
  over the metadata row — a drop-in filter, no second index, no HNSW rebuild.
- **Recall guard:** filtered ANN degrades under selective filters (the standard
  filtered-ANN failure). The existing exact-scan recall floor (`:8050–8055`) and
  `RERANK_OVERFETCH` (`:7997–8006`) extend to the filtered path; when the predicate's
  survivor fraction falls below a threshold (default 10%), fall back to **exact arena
  scan** under the predicate + sidecar rerank — correct by construction, and historical
  queries are audit-shaped (rare, latency-tolerant).
- Post-search, each candidate resolves through `node_versions` at t (props/labels/window
  rewrite), replacing the live-map gate at `:8090` for the tx path only. HNSW stays
  append-only and non-persisted; retraction still never mutates the graph.
- The retracted-node candidates this surfaces are exactly the rows compaction now
  preserves (§3.4).

### 3.4 Compaction respects the horizon

`perform_index_compaction`'s live-set filter (`:10517`) becomes horizon-aware: keep a
non-live row iff `retired_seq >= history_horizon()` **and** the profile retains history.
Beyond-horizon rows are still physically destroyed — same destruction point as today,
now aligned with the journal's own horizon instead of racing ahead of it. Under
`FrontierOnly` the filter reduces to today's behavior exactly (C4).

## 4. Phasing (each phase independently shippable, PR-per-phase)

| Phase | Delivers | DoD |
|---|---|---|
| **E1** | §3.2 overlay + graph-side epoch candidates | RED test un-`#[ignore]`d and green; full suite green; overlay GC covered by retention tests; `FrontierOnly` A/B shows no regression |
| **E2** | §3.1 stamps (mv:2 + migrate) + §3.3 filtered/exact vector path + §3.4 compaction rule + C5 capabilities | new `vector_tx_as_of` test matrix (quadrants incl. resurrect + reopen + compact-then-query); moat-bench gains a vector-time-travel row; snapshot roundtrip + v1→v2 migration tests |
| **E3** *(evidence-gated — do NOT schedule)* | True per-epoch HNSW sub-indexes aligned to sealed-segment `[min_seq, max_seq]` (`:2390–2410`), merged at query time | Trigger: measured E2 historical-query latency exceeding budget on a real consumer, or filter-selectivity pathology in practice. Until then E2's exact-scan floor is the honest fallback. |

## 5. Invariants preserved

- **I-epoch-1:** journal bytes, frame format, and `SignedEvent` payloads unchanged; every
  new structure is derived and rebuildable (WAL stays the durability authority).
- **I-epoch-2:** I6 horizon honesty — no surface answers beyond the horizon; fold remains
  the single destruction boundary for both journal history and (now) vector history.
- **I-epoch-3:** default-profile cost neutrality (C4) — verified by A/B bench.
- **I-epoch-4:** current-view read paths (`neighbors`, `hybrid_search` without
  `tx_as_of`) keep their exact semantics and perf envelope.
- **I-epoch-5:** never codify a gap as expected behavior — the RED test flips by fixing
  the engine, not the assertion (storage-readiness audit rule).

## 6. Non-goals

Valid-time vector filtering beyond the existing `as_of` post-filter; per-epoch HNSW
before E3's trigger fires; sync-wire changes (CommitFrame prev-hash chain stays a
separate deferred item); FTS/S3; changing the default retention profile.
