# RCA: Windows Relational Projection Replay Reopens the Locked Database

## Status / Date

Candidate-staging repair ported to PR #233 at ddfc6e9; PR-head validation pending / 2026-10-15.

## Symptom

FUNG issue #41 reports that opening an existing Windows profile fails in
`Storage::open` with `REL_SCHEMA_VERSION_CONFLICT`, before the application can
register its schemas. The projection passes SQLite integrity checks, but its
registry contains the current schema while the retained journal includes older
schema revisions.

## Evidence

- GenesisBlock issue #230 links the reproduction to FUNG commit
  `77e07f7b127f65a1f92cfc688ef15f322115d5f8`.
- `projection_sync_on_open` retries failed WAL replay through
  `projection_reopen_fresh` in `src/lib.rs`.
- `projection_reopen_fresh` previously ignored errors from removing
  `projection.sqlite` while `projection_db` still owned its open connection.
- Windows keeps the file locked by that connection, so the retry could reopen
  the existing schema and repeat the conflict.

## Root Cause

Recovery treated deleting and reopening a derived SQLite projection as a
portable replacement operation. The live connection remained open during the
remove attempt; Windows rejected the removal, the error was discarded, and the
new connection could still address the old projection.

## Why Detection Failed

Existing recovery coverage verified successful cold projection rebuilds but did
not exercise an additive relational-schema upgrade followed by reopen while the
current projection connection still held the file open. It also lacked a
failure-path assertion that the pre-rebuild schema and data remain available.

## Prevention

Preflight schema history. On any journal fingerprint miss, replay and validate
on an engine-owned temporary SQLite candidate: compatible histories begin from
a backup of the current projection, while incompatible schema histories begin
blank. If compatible replay fails, discard that candidate and retry from blank.
Publish only a complete candidate into the existing connection through SQLite's
backup API. This keeps the original projection untouched on replay errors and
lets SQLite roll back an unfinished publication transaction. Hash the
authoritative journal in bounded chunks with a versioned SHA-256 domain so
unchanged histories skip candidate creation; appends and compaction invalidate
the marker, and an unreadable marker takes the repair path. Cover upgrade/reopen,
preserved rows, candidate replay failure after an earlier event, failed blank
rebuild, append/compaction invalidation, repeated unchanged reopen, and an
interrupted backup copy.

## Outcome

Focused validation after the staged-replay integrity change passed on Windows:

- `cargo test --locked --offline --no-default-features --jobs 1 --target-dir C:\Users\suanr\AppData\Local\Temp\GenesisBlock-Codex-Cargo\issue-230-projection-target-20261015 --test projection_replay_recovery_tests -- --nocapture` — 6 passed, 0 failed.
- `cargo test --locked --offline --no-default-features --jobs 1 --target-dir C:\Users\suanr\AppData\Local\Temp\GenesisBlock-Codex-Cargo\issue-230-projection-target-20261015` — exit 0; unit tests 7/7, every enabled integration target passed (including projection recovery 6/6), expected ignored tests remained ignored, and doc tests 0.
- `cargo build --locked --offline --no-default-features --jobs 1 --target-dir C:\Users\suanr\AppData\Local\Temp\GenesisBlock-Codex-Cargo\issue-230-projection-target-20261015` — exit 0.
- `rustfmt --edition 2021 --check src/lib.rs tests/projection_replay_recovery_tests.rs` and `git diff --check` — passed after the final source change.

The focused suite covers additive schema upgrade/reopen with row preservation,
a failed blank rebuild leaving the prior schema and row usable, a valid event
followed by an invalid event proving failed incremental replay does not change
the live projection, append and compaction fingerprint invalidation, repeated
unchanged reopen, and rollback after an abandoned SQLite backup has copied a
page. The upgrade fixture removes the sync marker after v1-to-v2 while retaining
the v2 registry and data, so it exercises the marker-less legacy history path.
These results validate the isolated current-main worktree only. The candidate-staging
change has now been ported to PR #233; PR-head tests and build remain pending.

An earlier serialized run on D: failed while linking
`hql2_storage_join_oracle_differential_tests` with MSVC `LNK1180` (insufficient
disk space). Moving the target directory to C: resolved that environment issue.
