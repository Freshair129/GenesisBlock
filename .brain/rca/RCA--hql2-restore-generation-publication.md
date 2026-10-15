# RCA: Restore Exposes the Target Before P6 Generation Publication

## Status / Date

Root cause confirmed by a focused read-only restore regression / 2026-10-05.

## Symptom

`Storage::restore_backup` returns success, but opening the restored target in
read-only mode and pinning its generation fails with `Error { reason:
"read-only" }`. The restore target is visible despite lacking a generation
that can be pinned without a write.

Reproduction: run
`cargo test --locked --offline --no-default-features --jobs 1 --target-dir
D:\CodexBuilds\GenesisBlock-HQL2-Execution-20261004 --test
hql2_restore_generation_tests`. Before the fix, the single test fails at
`pin_generation` with `read-only` (0 passed, 1 failed).

## Evidence

- `src/lib.rs:21350-21357` opens and drops the staging `Storage` for snapshot/WAL
  validation; `src/lib.rs:21360-21362` then renames staging to the caller-visible
  target without publishing a generation.
- `src/lib.rs:7563-7571` shows `pin_generation` lazily calls
  `publish_generation_unlocked` if the persisted generation is absent or stale.
  That path requires a writable store (`src/lib.rs:4377-4379`), explaining the
  measured read-only failure.
- The approved H2-D11 R5 contract requires restore to complete normal recovery,
  P6 generation publication, and independent validation before success
  (`docs/adr/ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS.md:450-454`).
- The existing backup/restore parity test opens the restored target writable
  and executes HQL afterward (`tests/hql2_revision_backup_restore_tests.rs:391`);
  that later query can lazily publish the generation and masks the restore-time
  gap.
- `Storage::stable_frontier()` is the last durable WAL-frame sequence, while
  `txn_frontier()` is the frame sequence of the last transaction-API commit
  (`src/lib.rs:14206-14219`). `GenerationInfo.wal_frontier` is the data frontier
  covered by publication and `publication_seq` is the following local P6 frame
  (`src/lib.rs:4377-4405`; `docs/IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22.md:432-443`).

## Root Cause

The restore validation path verifies that the copied snapshot and WAL can be
opened, but it does not publish a P6 generation at the recovered frontier or
independently prove that the published generation can be pinned. It then
renames the validated staging directory into view. Generation publication is
deferred to a later writable query through `pin_generation`, so a successful
restore is not immediately usable as a read-only database.

The first post-fix assertion compared the generation's `txn_frontier` to the
backup manifest's `stable_frontier`. That assertion was invalid: graph writes
advance the WAL-frame frontier without advancing the transaction-API frontier.
The publication must bind `generation.wal_frontier` to the manifest frontier;
the signed local `GenerationPublished` receipt may advance the restored live
WAL frontier without changing the backup's declared data frontier.

## Why the Issue Escaped Detection

The prior restore test used a writable `Storage::open` and performed HQL/typed-IR
queries after restore. Those queries invoke the lazy publication path before
checking history parity. No test attempted a read-only generation pin at the
restore-success boundary.

## Fix (Decided)

While the target is still private staging data, explicitly publish a P6
generation after recovery. Drop the writable validator, reopen staging
read-only, pin and validate a lease, and rename only after that independent
validation succeeds. Keep the existing `full` retention validation profile so
shutdown does not fold the restored journal. This implements the approved R5
contract and does not change the backup bundle format, schema, or user-database
state.

## Proposed Prevention

Every restore-success regression must validate the target before any writable
query can trigger lazy generation publication. Require a read-only reopen,
generation pin, lease validation, stable-frontier check, and retained-history
floor check before treating the target as published.

## Outcome (Measured)

The regression reproduced the original `read-only` error before the fix (0
passed, 1 failed). After explicitly publishing at the manifest WAL frontier,
then reopening staging read-only and pinning/validating the lease, the focused
restore/P6 set passes 21/21 across `backup_restore_u9_tests` (7),
`hql2_revision_backup_restore_tests` (1), `hql2_restore_generation_tests`
(3), `p6_generation_tests` (6), and `wave_b_collection_lifecycle_tests` (4).
The explicit root-HQL2 sweep passes 393/0/1 across 37 targets;
`zz_probe_discriminates` was excluded.
`cargo test --locked --offline --no-default-features --jobs 1 --no-fail-fast --
--skip probe_vs_recall` exits 0 with zero failures; the long probe was filtered,
the three soak tests remain ignored, and the parser child entrypoint is covered
by its parent. `cargo fmt --all -- --check`, `git diff --check`, and
`npm run docs:validate` pass (239 files, 0 violations); `npm run
agents:validate` passes (6 agents, 12 routes). Bundle manifest frontier,
history horizon, source floors, revision identities, schema, annotation
selectors and HQL/typed-IR history parity remain covered.

## Independent Review Follow-up — 2026-10-05

### Symptom

A valid backup whose final WAL frame was an existing `GenerationPublished`
receipt failed restore with `GENERATION_STALE: restored WAL frontier mismatch`.
Separately, a bundle-info hashing failure could return an error after the
staging directory had already been renamed to the caller-visible target.
After receipt-last support was added, a manifest frontier one frame beyond the
recovered WAL could also collide with a newly written receipt's
`publication_seq`, making an incomplete bundle appear restorable.

### Evidence

- The new receipt-last test failed before the correction with the exact
  `GENERATION_STALE` error and passes afterward. It asserts that the restored
  `txn_frontier` is unchanged and the existing publication remains pin-able.
- `publish_generation_unlocked` returns the existing generation when its
  `publication_seq` already equals the current WAL frame frontier. For a valid
  terminal receipt, that sequence equals the manifest frontier while the
  generation's `wal_frontier` is the preceding frame.
- The old success branch called `fs::rename` before `backup_info`; the latter
  opens and hashes the source bundle and is fallible. The current success branch
  prepares `BackupBundleInfo` before rename, leaving the rename as the only
  fallible publication operation.
- A RED regression rewrites only the manifest frontier to one frame beyond the
  unchanged artifact WAL. Before the recovery guard, restore returned `Ok` and
  exposed the target because the new receipt's `publication_seq` equaled that
  incorrect manifest value. After the guard, restore returns
  `GENERATION_STALE: recovered WAL frontier mismatch` and leaves no target.
- A failed final `fs::rename` previously bypassed the earlier error cleanup
  arm. The current rename-error branch explicitly removes the staging directory.

### Root Cause

Restore treated the generation's covered data frontier as the only valid match
for the manifest frame frontier, ignoring the valid state where the manifest
ends exactly at the generation's publication receipt. Its return path also put
fallible source-bundle I/O after the target publication boundary.

### Why the Issue Escaped Detection

The earlier restore fixture exported without first publishing a generation, so
restore always exercised new-generation publication. The subsequent receipt
compatibility check compared generation coordinates only after publication and
did not first prove that normal recovery reached the bundle's declared frame.
Existing result tests did not observe or induce bundle I/O failure after staging
validation; code ordering allowed that error to occur after rename. Rename
failure cleanup also had no explicit branch.

### Proposed Prevention

Require normal recovery to reproduce the exact manifest frame frontier before
publication. Keep explicit regression coverage for both valid generation
states: a bundle requiring local publication and one already ending in its
publication receipt. Assert frame-frontier/receipt relationships and unchanged
nonzero `txn_frontier`, reject a manifest/WAL mismatch before target creation,
and clean staging on both validation and rename failures. Keep all fallible
bundle reads, digest work and independent read-only generation validation
before rename; after rename, return only the prepared result.

### Resolution (Measured)

Restore first checks exact recovered/manifest frame-frontier equality, then
accepts either a new generation whose `wal_frontier` matches or a validated
terminal receipt whose `publication_seq` matches. Bundle metadata is prepared
before rename, and rename failure attempts staging cleanup. Receipt-last and
manifest mismatch tests are RED before and GREEN after the corrections; both
successful restore paths preserve a nonzero `txn_frontier`. The independent
review confirmed the core paths. Cleanup after read-only-validation and rename
failures is best-effort; those specific failure paths are not fault-injected,
so cleanup is not claimed as guaranteed. Full Rust verification exited 0 with
`probe_vs_recall` filtered; HQL2 passed 393/0/1 across 37 targets.
