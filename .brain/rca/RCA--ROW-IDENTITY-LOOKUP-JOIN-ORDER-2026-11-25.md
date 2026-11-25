---
version: "0.1.0b"
status: candidate
created_at: "2026-11-25,Claude"
scope: Relational upsert latency growth for FUNG issue 135
---

# Row identity lookup join order

Parent: `docs/MASTER-SPEC--GENESIS-DB.md` unified storage boundary. This is a
LOW-risk bounded repair (one query, no schema, API, or WAL change), authorized by
the FUNG owner's request to fix FUNG #135 in GenesisBlock.

## Symptom

FUNG routed recordings stopped after about 11 minutes. Each source commits one
2-second audio chunk per transaction: 2 new rows (`audio_chunks`,
`capture_source_chunks`) plus upserts of 3 existing rows (`capture_sources`,
`mobile_recording_checkpoints`, `recordings`). Commit latency grew with database
age. When one commit exceeded the writer's 640 ms queue budget, the source failed
with `QueueOverflow` and the recording ended `partial`.

## Evidence

- In FUNG's writer measurement (debug build), commit time grew from 62 ms at
  commit 60 to 380 ms at commit 480. The other custody reads stayed flat.
- 500 unrelated new-row transactions did not slow later commits. So the growth is
  not driven by WAL or transaction count alone.
- `active_row_id_for_key` joins `hql2_row_identity_registry r` with
  `hql2_record_revisions v`. Without `ANALYZE` statistics, SQLite 3.50.4 plans `v`
  as the outer table:
  `SEARCH v USING INDEX sqlite_autoindex_hql2_record_revisions_1 (database_id=? AND namespace=? AND kind=?)`.
  That walks every row revision in the namespace on each call.
- On FUNG's UAT projection (6,645 revisions), the query took 113 ms per call. It
  runs for every upsert of an existing row, three times per commit
  (`build_record_revision_transaction`, `preflight_relational_event`,
  `projection_apply_event`), so 9 calls per FUNG chunk commit.
- With `CROSS JOIN`, the plan becomes
  `SEARCH r USING INDEX ... (database_id=? AND namespace=? AND table_name=?)` then
  `SEARCH v USING INDEX idx_hql2_revisions_visibility (... record_id=?)`. The call
  took 0.34 ms and returned an identical result.

## Root cause

The query leaves join order to SQLite's planner. Without statistics, the planner
judges the registry primary-key probe cheap and drives from the revision table.
That makes each existing-row upsert O(row revisions in the namespace) instead of
O(revisions of that row).

## Why detection failed

- Existing tests check correctness on small namespaces. Nothing measured
  per-commit cost as history grows.
- GenesisBlock never runs `ANALYZE`, so the plan never self-corrected.
- FUNG's earlier acceptance recordings were short (under 3 minutes) on fresh
  profiles.

## Repair

Use `CROSS JOIN` so the key registry (`r`) is always the outer loop. Same
predicates, same result, no schema change.

## Prevention

`tests/hql2_row_identity_lookup_scaling_tests.rs` times 40 upserts of one row
before and after 4,000 inserts into another table in the same namespace. It
requires the ratio to stay below 2.0. Without the repair it measured 5.71x on
`main` and 4.99x on `2cf4855`. With it, 1.01x and 1.42x.

## Out of scope (recorded separately)

Upserts still slow down as rows accumulate in the same table: about 2–3x per
4,000 rows, mixed with upserts of the same row. FUNG's capture path is not
affected, because its new rows go to different tables than its updated rows. In
FUNG's writer measurement with this repair, commits stayed flat at 28–34 ms over
480 commits. This needs its own investigation.
