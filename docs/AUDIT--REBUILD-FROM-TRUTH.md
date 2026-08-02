---
status: historical
---

# AUDIT--REBUILD-FROM-TRUTH: Source-of-Truth Rebuild Tests

**Date:** 2026-06-28
**Suite:** `tests/rebuild_from_truth_tests.rs`
**Result:** 10/10 PASS

## Motivation

A database engine must guarantee that persisted state is a faithful replica of
in-memory state. These tests validate the full round-trip: write data →
persist (snapshot or WAL) → destroy in-memory state (drop Storage) → reload →
assert every field is identical. This catches serialization bugs, field
omissions, index reconstruction errors, and clock drift.

## Test Matrix

| # | Test | What's verified |
|---|------|----------------|
| 1 | `snapshot_roundtrip_nodes_edges_vectors` | 20 nodes + 19 edges + vector search survive snapshot save/reload |
| 2 | `wal_only_roundtrip` | 10 nodes + edge + vector search recover from WAL replay only |
| 3 | `node_properties_survive_roundtrip` | Nested JSON props, unicode, arrays survive snapshot |
| 4 | `edge_properties_survive_roundtrip` | Edge props (weight, type) survive snapshot |
| 5 | `graph_indices_survive_roundtrip` | `out_idx` / `in_idx` (adjacency lists) correctly rebuilt |
| 6 | `idempotent_double_reload` | open→save→reopen→save→reopen produces identical state |
| 7 | `superseded_node_chain_survives` | `caused_by` version chain preserved across reload |
| 8 | `large_batch_roundtrip` | 1000 nodes + 999 edges + vector search survive snapshot |
| 9 | `labels_survive_roundtrip` | Multi-label nodes preserve all labels |
| 10 | `logical_clock_persists` | Lamport clock value identical after reload |

## Key Findings

- **All tested fields round-trip faithfully**: id, labels, props (nested JSON +
  unicode), embedding vectors, edge relationships, caused_by chains, logical clock.
- **Graph indices are rebuilt on load**, not stored — `out_idx` and `in_idx` are
  reconstructed from the edge list, which is the correct design for crash safety.
- **Vector search works after both snapshot reload and WAL-only replay** — HNSW
  index is rehydrated from the arena on load.
- **1000-node batch** completes the full cycle in ~120s (debug build); release
  would be significantly faster.

## Run Command

```bash
cargo test --no-default-features --test rebuild_from_truth_tests
```
