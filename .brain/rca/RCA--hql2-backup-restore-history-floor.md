# RCA: HQL2 Backup Restore Advances the Retained History Floor

## Status / Date

Root cause confirmed; regression and fix verified / 2026-10-05.

## Symptom

Restoring a database whose source uses the `full` retention profile preserves
its current records and stable frontier, but advances `history_horizon()` from
`0` to `9`. The durable Node revision at `tx_from=3` remains in
`projection.sqlite`; HQL2 HistoryScan returns it from the source and returns no
row from the restored database.

Reproduction: run `h2_d11_revision_identity_schema_floors_and_history_survive_backup_restore`
in `tests/hql2_revision_backup_restore_tests.rs`. Before the fix, the test
fails at the source/restored floor equality assertion with `left: 0`, `right:
9`; an earlier run reached the HQL assertion and showed one source history row
versus zero restored rows.

## Evidence

- Before the fix, parent `HEAD:src/lib.rs:21355` passed `retention: None`
  to the writable validation open inside `Storage::restore_backup`
  (`src/lib.rs:21228-21357`). This revision passes `Some("full")` at
  `src/lib.rs:21355`.
- `src/lib.rs:19730-19737`: a checkpoint folds when retention is
  `FrontierOnly`; `Full` does not fold.
- `src/lib.rs:25727-25730`: dropping writable `Storage` calls `save_state()`.
- `src/lib.rs:22791-22795`: a successful fold advances the journal history
  boundary to the checkpoint frontier.
- `src/lib.rs:23715-23727`: HQL2 Node/Edge HistoryScan obtains its source floor
  from `Storage::history_horizon()`.
- The approved H2-D11 R5 contract requires history selectors below the source
  floor to fail closed and its minimum verification list requires backup/
  restore parity for retention floors and annotation selectors
  (`docs/adr/ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS.md:391-403,
  735-744`).
- The existing U9 backup/restore test checks live graph, relational, vector,
  and frontier restoration, but does not check H2-D11 history floors or
  HistoryScan parity (`tests/backup_restore_u9_tests.rs:145-225`).

## Root Cause

The internal restore-validation open inherits the default `frontier_only`
retention profile because `retention` is unset. Its writable `Storage` is
dropped before the staging directory is published; the drop checkpoint folds
the staged journal at the current frontier. That fold discards the earlier
history segments and raises `history_horizon()`, while the copied H2-D11
projection still contains revisions below the new floor. HistoryScan then
correctly filters those revisions out. The restore validation step therefore
mutates the only staged copy instead of validating it without shortening its
retained history.

## Why the Issue Escaped Detection

The backup/restore regression covered current graph, relational, and vector
behavior plus stable-frontier equality. It did not compare retained history
floors or execute HQL/typed-IR HistoryScan against the restored database. The
H2-D11 R5 parity requirement was present, but had no restore-specific fixture.

## Fix (Decided)

Open the extracted staging database with the `full` retention profile during
restore validation. This keeps the automatic shutdown checkpoint from folding
the journal being validated; it does not change bundle format, schema, or the
retention profile callers may choose when they later open the restored
database. Keep regression assertions for stable frontier, history floor,
revision/projection identity, and HQL/typed-IR HistoryScan and ChangeScan
parity. This implements the already-approved H2-D11 R5 parity requirement; it
does not claim completion of the broader P14 rehearsal gate.

## Outcome (Measured)

The minimum fix sets `retention` to `Some("full")` for the internal restore
verification open. The same regression now passes and confirms equal source /
restored frontiers, history horizons, database identity, durable revision and
row IDs, schema package, annotation selectors, source floors, and HQL/typed-IR
HistoryScan and ChangeScan results. The focused backup, durable-revision,
History/Change, row-history, and annotation ACL/source targets pass 59/59
across seven test binaries. The explicit HQL2 sweep passes 389/0/1 across 35
targets, and the full locked/offline Rust suite exits 0 (three soak cases
ignored). The full suite also executes the unrelated 60-build
`probe_vs_recall` experiment; its 1271.40-second test pass is informational,
not performance/P1 qualification. No hosted CI or independent-review result is
claimed.
