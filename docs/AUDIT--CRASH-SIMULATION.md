# AUDIT--CRASH-SIMULATION: WAL & Snapshot Crash Recovery Tests

**Date:** 2026-06-28
**Suite:** `tests/crash_simulation_tests.rs`
**Result:** 15/15 PASS | 0 regressions across full suite

## Motivation

A storage engine that claims durability must survive mid-write crashes without
data loss or silent corruption. GenesisBlockDB uses a WAL (Write-Ahead Log) +
snapshot architecture with a defined recovery hierarchy:

1. Load snapshot (`state.json` + binary files) if present and valid
2. Fall back to line-by-line WAL replay if snapshot is missing or corrupt
3. Skip unparseable WAL lines (truncated tail or garbage entries)

These tests validate that hierarchy under 15 distinct failure scenarios.

## Recovery Architecture (as tested)

```
Startup
  |
  +-- state.json exists and parses?
  |     YES -> load vec_*.bin, meta_*.bin, nodes.bin, edges.bin
  |             (missing/corrupt files: skip silently, degrade gracefully)
  |     NO  -> WAL replay (genesis-graph.wal)
  |             line-by-line JSON deserialization
  |             bad lines skipped (no propagation)
  |
  +-- Both missing? -> empty database, no panic
```

## Test Matrix

| # | Test | Corruption Type | Assertion |
|---|------|----------------|-----------|
| 1 | `truncated_wal_recovers_intact_entries` | WAL truncated mid-JSON-line | Lines before truncation point recovered |
| 2 | `garbage_wal_entry_skipped_gracefully` | Garbage line injected mid-WAL | All valid entries (before + after) recovered |
| 3 | `missing_state_json_falls_back_to_wal` | `state.json` deleted | Full WAL replay, all nodes + edges recovered |
| 4 | `corrupt_state_json_falls_back_to_wal` | `state.json` overwritten with garbage | WAL fallback, all data recovered |
| 5 | `missing_vec_bin_recovers_nodes_without_vectors` | `vec_default.bin` deleted | Nodes exist (from `nodes.bin`), vectors lost, no panic |
| 6 | `truncated_vec_bin_no_panic` | `vec_default.bin` truncated to 50% | Engine loads without crash |
| 7 | `corrupt_nodes_bin_recovers_via_wal` | `nodes.bin` overwritten with garbage | No panic (snapshot partial load) |
| 8 | `corrupt_edges_bin_no_panic` | `edges.bin` overwritten with garbage | Nodes survive, edge data lost, no crash |
| 9 | `empty_wal_no_panic` | WAL zeroed + snapshot deleted | Empty DB, no panic |
| 10 | `double_crash_recovery` | Two successive WAL tail truncations | >=8/10 nodes survive across both crashes |
| 11 | `post_snapshot_wal_entries_survive` | Normal operation (no corruption) | Data written after snapshot survives via WAL |
| 12 | `corrupt_meta_bin_no_panic` | `meta_default.bin` overwritten | Node data survives from `nodes.bin` |
| 13 | `edge_recovery_from_wal` | Snapshot deleted, edges in WAL | Nodes + edges fully recovered |
| 14 | `stale_temp_save_dir_no_interference` | Leftover `temp_save/` dir with partial files | Clean load from real snapshot |
| 15 | `vector_search_works_after_wal_recovery` | Snapshot deleted, WAL-only recovery | Vector search returns correct nearest neighbors |

## Key Findings

### Strengths
- **No panics under any tested corruption.** Every scenario loads gracefully.
- **WAL is an independent recovery source.** Deleting the entire snapshot triggers full WAL replay with no data loss.
- **Line-level granularity.** Truncated or garbage WAL lines are skipped; all valid lines before and after are recovered.
- **Snapshot atomicity.** `state.json` is written last; crash mid-snapshot leaves no `state.json` which triggers WAL fallback.
- **Double crash resilience.** Two successive crashes with WAL corruption still recover the vast majority of data.

### Degradation Modes (by design)
- Missing `vec_*.bin`: nodes exist but vectors are empty (search returns nothing until re-indexed)
- Corrupt `nodes.bin` / `edges.bin`: snapshot loader skips the corrupt file; data may be lost if WAL was compacted
- Corrupt `meta_*.bin`: metadata lost but node/edge data preserved

### No Checksum/CRC
The WAL relies on JSON schema validity (serde deserialization) rather than checksums. A bit-flip that produces valid JSON but wrong data would not be detected. This is acceptable for the current maturity level but worth revisiting for production hardening.

## Run Command

```bash
cargo test --no-default-features --test crash_simulation_tests
```
