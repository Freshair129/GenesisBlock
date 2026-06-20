---
proposed_id: INCIDENT--EDGE-U32-BUILD-BREAK-AND-RAM-MISDIAGNOSIS
type: incident
status: resolved
aliases:
  - INCIDENT
  - RCA
tier: process
cluster: implementation_flow
role: "Incident report & root-cause analysis"
severity: high
detected_at: 2026-06-20
resolved_at: 2026-06-21
proposed_by: agent
related:
  - CR--EDGE-ENDPOINTS-STRING-AND-EMBEDDING-DEDUP
  - SPEC--MULTI-COLLECTION-VECTOR-SPACE
  - AUDIT--P12-SCIENTIFIC-VERIFICATION-REPORT
commits:
  - b5e9771
  - 75d560e
  - 663ba7b
---

# INCIDENT — Edge u32 build break & RAM misdiagnosis

## 1. Summary

A docs-vs-code audit found the working tree of `src/lib.rs` in a non-compiling,
half-finished state: an uncommitted refactor had changed edge endpoints
(`EdgeInput/EdgeOutput.from/to`) from `String` to `u32` (raw interned arena
ids). This broke the build, the entire test/bench suite, and every client SDK,
and introduced latent correctness/availability defects. Separately, a snapshot
"instant-load" path left the HNSW index empty (silent search outage), and an
initial performance hypothesis about peak RAM was wrong by three orders of
magnitude. All issues are resolved on branch `fix/edge-endpoints-string-revert`.

## 2. Severity & impact

- **Severity:** High (engine does not build; core suite cannot run).
- **Build:** `cargo check --lib` failed with `E0308` at `src/lib.rs:1695`.
- **Tests/benches:** every target touching edges failed to compile
  (`grl_retrieval_tests`, `ephemeral_nodes_tests`, `state_transition_tests`,
  `scientific_audit`, `snb_bulk_ingestion`, `hql_query_stress`, …).
- **Clients:** `index.d.ts`, Go `models.go`, Python SDK all address nodes by
  `String`; the `u32` engine would reject string node-ids on edge creation.
- **Latent runtime defects** (would have shipped if it compiled):
  - `add_edge` panicked via `unwrap()` when an endpoint `u32` was not interned.
  - Edge endpoint `u32` is replay-order dependent → not stable across a
    WAL-only rebuild (silent mis-pointing of edges).
  - Snapshot instant-load never rehydrated HNSW → `SEARCH`/hybrid returned
    `"HNSW not init"` until a manual rebuild.

## 3. Detection

Triggered by a user request to audit documentation against the codebase. Three
parallel review agents (docs claims / code surface / SDK drift) surfaced the
edge type conflict; `cargo check` confirmed the compile break as ground truth.

## 4. Root-cause analysis

**Primary cause.** An incomplete architectural change pushed an internal
identifier (`u32` arena id) outward to the public API and the on-disk
persistence format. Internal ids are an implementation detail: they are not
known to clients (`NodeOutput` only exposes the `String` id) and not stable
across a WAL replay. Exposing them inverted the correct layering (stable string
identity at the boundary, fast `u32` inside).

Evidence the `String` contract was the intended design: tests, benchmarks, both
SDKs, and the generated TypeScript binding all use string node-ids — only the
hand-edited Rust struct diverged.

**Secondary cause (silent search outage).** `Storage::open` called
`rehydrate_hnsw_index()` only on the WAL-replay branch; the `try_load_state()`
snapshot branch populated the vector/metadata arenas but never rebuilt HNSW.

**Process cause (RAM misdiagnosis).** Peak RAM of 15.89 GB
(`AUDIT--P12-SCIENTIFIC-VERIFICATION-REPORT`, 2026-06-01) was initially blamed
on `init_hnsw()` hardcoding `max_elements = 1_000_000`. Reading hnsw_rs 0.3.4
(`PointIndexation::new`) showed `max_elements` is only a `Vec::with_capacity`
hint (~8 MB), not a hard cap. A measured run of the current engine showed ~1 GB
at 32k nodes — i.e. the 15.89 GB figure is a stale Mark VII artifact, and the
real avoidable cost was triple-stored embeddings, not HNSW reservation. The
lesson: an unverified hypothesis was nearly actioned; source-reading +
measurement corrected it.

## 5. Resolution

| Commit | Action |
|---|---|
| `b5e9771` | Revert edge `from/to` to `String`; intern to `u32` only inside `index_edge_internal`; resolve via `get_u32` at all traversal/meta/compaction/BFS sites. Remove `add_edge` panic path. Rehydrate HNSW on **both** load paths. Fix pre-existing `ldbc_lite` bench rot. |
| `75d560e` | Drop the redundant in-memory f64 `node.embedding` (arena/HNSW are the source of truth) via `insert_node_lean()` — measured −44% RSS. |
| `663ba7b` | Record `SPEC--MULTI-COLLECTION-VECTOR-SPACE`; correct the RAM diagnosis in writing. |

## 6. Verification

- `cargo check --all-targets` clean.
- `cargo test` green: 20 passed, 0 failed across 21 binaries.
- RAM measured (5k nodes / dim 1536): net RSS 147 MB → 82 MB (−44%).

## 7. Lessons & preventive actions

1. **Never expose internal ids at the API/persistence boundary.** Keep stable
   string identity outward; intern to `u32` internally only.
2. **The build/test suite is the contract.** A change that breaks tests, both
   SDKs, and the generated binding is a design smell, not a mass-update chore.
3. **Verify performance hypotheses before acting** — read the dependency source
   and measure; do not pattern-match a number to a guessed cause.
4. **Symmetry in load paths.** Snapshot and WAL-replay must reconstruct
   identical in-memory state (HNSW included); add a regression test that opens a
   snapshot-loaded DB and asserts `SEARCH` returns results.
5. **Don't commit half-finished refactors to a working tree** that leave the
   crate uncompilable; gate on `cargo check` before pausing work.
