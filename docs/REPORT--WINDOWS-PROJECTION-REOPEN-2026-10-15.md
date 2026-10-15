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

The fallback now uses a SQLite-managed temporary database. It replays there and
publishes only a successful result with the SQLite backup transaction into the
original connection. The original connection is restored for every returned
result. Failed replay and busy/locked publication preserve its data. Page size
matches the original; no open file is unlinked, no raw application SQL mutation
is introduced, and no schema/version validation is bypassed. Rusqlite's `backup`
feature enables the existing SQLite API; public APIs and journal format stay
unchanged.

## Evidence

Windows, `cargo test --no-default-features --features mobile`, one build job.
Local evidence: `G:\AI-Studio\verification\2026-10-15-fung-issue41`.

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
