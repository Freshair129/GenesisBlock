---
proposed_id: CR--EDGE-ENDPOINTS-STRING-AND-EMBEDDING-DEDUP
type: change-request
status: merged
aliases:
  - CR
tier: process
cluster: implementation_flow
role: "Change request"
complexity: C-2
proposed_at: 2026-06-21
proposed_by: agent
branch: fix/edge-endpoints-string-revert
related:
  - INCIDENT--EDGE-U32-BUILD-BREAK-AND-RAM-MISDIAGNOSIS
  - SPEC--MULTI-COLLECTION-VECTOR-SPACE
commits:
  - b5e9771
  - 75d560e
  - 663ba7b
---

# CR — Edge endpoints as String + embedding dedup

## 1. Summary

Restore the build and the green test suite by reverting edge endpoints to
`String` (interning to `u32` internally), then remove the redundant in-memory
f64 embedding copy for a measured ~44% RAM reduction. Adds the design spec for
the follow-on per-collection vector-space work.

## 2. Motivation

See `INCIDENT--EDGE-U32-BUILD-BREAK-AND-RAM-MISDIAGNOSIS`. The working tree did
not compile; the `u32` edge change broke tests/SDKs and was the wrong layering.
Independently, profiling identified the in-memory f64 embedding as the largest
avoidable per-node memory cost.

## 3. Scope

**In scope**
- Edge endpoint type at the API/persistence boundary (`String`).
- `index_edge_internal` and all edge-reading call sites.
- `add_edge` panic removal; HNSW rehydrate on snapshot load.
- In-memory embedding dedup (`insert_node_lean`).
- Bench compile rot in `benches/ldbc_lite.rs`.

**Out of scope (tracked separately)**
- Per-collection / multi-model vector spaces — `SPEC--MULTI-COLLECTION-VECTOR-SPACE`, phases P-C/P-D.
- HQL `/v1/query/hql` raw-string vs `{query}` SDK contract drift.
- `retract_edge` stub, gossip `PullRequest` handler, dead `LogicalPlanner`.

## 4. Changes

| # | Change | Files | Commit |
|---|---|---|---|
| 1 | `EdgeInput/EdgeOutput.from/to` → `String`; intern to `u32` inside `index_edge_internal`; resolve via `get_u32` at neighbors / detect_communities / generate_meta_graph / compaction / retrieve_context BFS / WAL replay / reconcile / snapshot / execute_batch | `src/lib.rs` | b5e9771 |
| 2 | Remove `add_edge` `unwrap()` panic on unknown endpoint | `src/lib.rs` | b5e9771 |
| 3 | Rehydrate HNSW on **both** load paths (snapshot + WAL) | `src/lib.rs` | b5e9771 |
| 4 | `ldbc_lite` bench: `rand::gen_range`, `NodeInput.lang`, `neighbors` arity | `benches/ldbc_lite.rs` | b5e9771 |
| 5 | `insert_node_lean()` strips f64 `embedding` from in-memory node store | `src/lib.rs` | 75d560e |
| 6 | Design spec + RAM-diagnosis correction | `docs/SPEC--MULTI-COLLECTION-VECTOR-SPACE.md` | 663ba7b |

## 5. Behavior / contract changes

- **Node read responses no longer echo raw embeddings.** `neighbors`,
  `retrieve_context`, and node reads return `embedding = None` (omitted in JSON
  via `skip_serializing_if`). Vectors remain available through the search path.
  This is intentional and a better default for graph results. The `add_node`
  return value still includes the embedding for that call.
- No change to edge endpoint *semantics* (still string node-ids) — this restores
  prior behavior; SDKs/bindings need no change.

## 6. Backward compatibility & migration

- **WAL:** edges serialize string endpoints again (matches pre-refactor and the
  on-disk `tests/*/genesis-graph.wal` fixtures). No migration required.
- **Snapshots:** `nodes.bin` now persists lean nodes; older snapshots with
  embedded vectors load fine (embeddings stripped on read; `vector.bin` arena is
  authoritative). Forward/backward compatible.
- **Durability:** WAL `Event::Node` still carries the full f64 embedding for
  replay → arena rebuild.

## 7. Risk & rollback

- **Risk:** Low. Pure-revert of an uncommitted change to known-good logic plus a
  localized, test-verified dedup.
- **Residual risk:** a consumer relying on embeddings being present in node read
  responses (none found in repo; no test depended on it).
- **Rollback:** `git revert 75d560e b5e9771` (the dedup and the edge revert are
  independent commits); the spec doc is documentation-only.

## 8. Testing & verification

- `cargo check --all-targets`: clean.
- `cargo test`: 20 passed, 0 failed, 21 binaries.
- RAM measured (`scientific-audit`, 5k nodes / dim 1536): net RSS 147 MB →
  82 MB (−44%); extrapolated 32k ≈ 960 MB → 525 MB.

## 9. Follow-up

- Merge `fix/edge-endpoints-string-revert` → `main`.
- Schedule P-C/P-D (per-collection vector spaces) per
  `SPEC--MULTI-COLLECTION-VECTOR-SPACE`.
- Add a snapshot-load regression test asserting `SEARCH` returns results.

## 10. Approval

| Role | Name | Decision | Date |
|---|---|---|---|
| Author | agent | proposed | 2026-06-21 |
| Reviewer | Boss | approved | 2026-06-21 |
| Merge | Boss | merged to `main` (fast-forward `3392feb..11dd357`) | 2026-06-21 |
