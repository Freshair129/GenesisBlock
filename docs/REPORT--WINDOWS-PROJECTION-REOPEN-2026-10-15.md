---
version: "0.1.0b"
status: candidate
created_at: "2026-10-15,Codex"
related_issue: 230
scope: bounded projection replay repair from FUNG engine pin 77e07f7
---

# Windows projection replay and reopen

Related: [GenesisBlock #230](https://github.com/Freshair129/GenesisBlock/issues/230)
and [FUNG #41](https://github.com/Freshair129/FUNG/issues/41).
RCA: `.brain/rca/RCA--WINDOWS-PROJECTION-REOPEN-2026-10-15.md`.

## Failure and repair

With a retained v1→v2 schema journal and an already-current v2 projection, replay
starts with a historical schema. The old fallback tries to delete the projection
while its SQLite connection remains open, ignores the Windows deletion failure,
and retries against the same old schema. On Windows the new regression fails on
the unchanged engine with `REL_SCHEMA_VERSION_CONFLICT`.

A missing or stale journal fingerprint now takes a staged path. A compatible
history is replayed against a candidate copied from the current projection;
schema-history conflicts use a fresh candidate. Replay, edge rebuilding,
backfill and validation finish before the candidate is published into the
original connection. If incremental replay fails, the candidate is discarded and
a fresh replay is attempted without ever mutating the live projection.

A versioned SHA-256 fingerprint streams authoritative journal files in 64 KiB
chunks. Unchanged opens skip replay; append or compaction changes the digest and
re-enters staged synchronization. SQLite backup publication preserves the open
connection; rusqlite drops an unfinished backup handle on error, allowing SQLite
to roll back the destination transaction. No open file is unlinked, raw
application SQL mutation is introduced, schema/version validation is bypassed,
or public format/API changed.

## Evidence

Windows, `cargo test --no-default-features --features mobile`, one build job.
Local evidence: `G:\AI-Studio\verification\2026-10-15-fung-issue41`.

The following logs were captured before the candidate-staging integrity port was
added to this branch; they describe the original repair attempt, not the current
PR head. Current-head validation is tracked below.

- RED: `engine-red.log`, new upgraded-projection regression failed on `77e07f7`.
- GREEN: retained schema history, Thai/pre-upgrade row, post-upgrade row, edit
  after reopen, and two further reopen cycles passed (`engine-green.log`).
- Failure safety: 2/2 passed (`engine-failure-safety.log`): explicit invalid
  replay and an external SQLite write lock preserve original rows/connection.
- Regression suites: 36/36 passed (`engine-regressions.log`): durability slices
  0/1, projection reopen, relational resource limit, U2/U2 contracts and Wave A
  commit correctness. These include mutation identity and checkpoint recovery.

The consumer must pin this repair and qualify its own profile. Actual FUNG
profile recovery and native close/reopen evidence belongs to the FUNG report.
This report does not claim an exhaustive engine suite, installer acceptance or
general forced-crash qualification. The branch starts at FUNG's exact engine
pin and intentionally includes no unrelated engine changes.

## Staged-replay port validation

The implementation has been ported additively onto the exact FUNG-pinned
engine baseline 77e07f7b127f65a1f92cfc688ef15f322115d5f8 in PR #233. The
current-main validation in the issue RCA is not evidence for that older branch.
PR-head focused and full test/build results remain pending.
