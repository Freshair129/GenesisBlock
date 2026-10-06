---
status: active
superseded_by: null
---

# RCA — GenesisRAG17 startup CI failures and Windows Rust timeout

**Date:** 2026-10-06
**Status:** Confirmed from hosted CI logs and current source.

## Symptom

PR #217 (`583ad2ff4b313179dfebf4727fe7b1612bb31cc8`) had worker test failures
on Ubuntu, Windows and macOS, plus a duplicate Linux worker failure in the
rebuilt-addon job. The Windows `cargo test --no-default-features` job was
cancelled at its 15-minute limit while running `probe_vs_recall`.

## Evidence

- Hosted run `37399649307` reports 11 failures in each
  `npm test (genesisrag17-worker, <host>)` job. The failing constructors report
  `RECOVERY_REQUIRED: markerless database identity is missing` at
  `genesisrag17-worker/src/worker.mjs:991`; rebuilt-addon run `37399649310`
  repeats the worker failure on Ubuntu.
- The same worker error and Windows test cancellation appear in PR #216 runs
  `37390134973` and `37390135616`, so these are pre-existing mainline CI issues,
  not regressions introduced by the G3 diff.
- `genesisrag17-worker/src/worker.mjs` creates `dbPath/genesisrag17` and its
  child directories, then acquires `worker.lock`, before calling
  `GenesisDatabase.open({ path: dbPath })`.
- `src/lib.rs:13076-13079` defines a fresh database as a missing or empty
  directory. `src/lib.rs:13148-13151` rejects a non-empty markerless path
  without `identity.bin` as recovery-required. The guard is consistent with
  the P6 fail-closed contract.
- `genesisrag17-worker/README.md` requires a new isolated TEST path and says the
  worker creates its sidecars there; fresh-path initialization is therefore
  part of the worker behavior, not an externally provisioned fixture.
- `tests/zz_probe_discriminates.rs` calls itself a temporary experiment and
  runs 60 builds of 1,000 nodes each, 50 ground-truth queries and 32 probes per
  build. It has no pass/fail assertion. `.github/workflows/test.yml` gives the
  complete Rust suite 15 minutes; the Windows log starts this test and then
  reports it still running after 60 seconds before cancellation.
- The unprovisioned model snapshot accounts for explicitly skipped model tests;
  it does not explain the constructor failures.

## Root cause

### Worker initialization

The worker writes worker-owned directories beneath `dbPath` before the native
engine classifies the path. A genuinely new store consequently looks non-empty
but has no `identity.bin`, so the engine correctly rejects it as ambiguous
markerless state. Pre-seeding test fixtures would hide a worker startup defect;
weakening the engine guard would risk initializing over markerless data.

### Windows suite timeout

An exploratory workload with no acceptance assertion is registered as a
normal Rust integration test. It runs on every host as part of the required
full suite, consumes the remaining Windows job budget and is cancelled by the
15-minute timeout.

## Why the issue escaped earlier detection

The current hosted CI did detect both problems and blocked merge; the failures
had also appeared in earlier PR CI. The G3 local verification was scoped to
Rust recovery tests, Clippy, formatting and docs, and did not run the worker
package or the full Windows Rust suite. The temporary diagnostic's placement
under `tests/` made its default-CI cost look like ordinary regression coverage
despite having no assertion.

## Fix (decided)

1. Keep `Storage::open`'s markerless recovery guard unchanged.
2. Serialize startup with a stable sibling SQLite coordination database and a
   rollback-journal `BEGIN EXCLUSIVE` transaction. Use zero busy timeout and
   fail clearly on contention; keep the mutex file stable and let SQLite
   recover after process death. Retain the persistent in-store worker lock for
   compatibility and ownership checks, with strict current/legacy metadata
   validation before PID liveness probes.
3. Canonicalize the store against its existing parent. Reject linked
   worker-sidecar directories, and prune only the fixed set of empty owned
   directories when identity is absent. Open the native store before creating
   sidecars; serialize token-checked worker-lock removal at close through the
   same SQLite transaction. Older worker binaries must be stopped before an
   upgrade because they do not observe the sibling mutex.
4. Add regressions for fresh-path identity, second-owner rejection, stale
   legacy-lock recovery, markerless data preservation, mutex contention and
   retry, mutex recovery after process exit, linked sidecar rejection, and
   malformed ownership metadata.
5. Keep `probe_vs_recall` available as an explicit manual diagnostic, but mark
   it ignored in the ordinary correctness suite with a visible reason. Its
   command is `cargo test --no-default-features --test
   zz_probe_discriminates -- --ignored --nocapture`.

## Proposed prevention

- Preserve the SQLite bootstrap and legacy-lock recovery rules in the worker
  ADR and README; do not unlink/recreate the sibling coordination database.
- Require all three hosted worker matrices, the rebuilt-addon worker job, and
  the Windows full Rust suite to pass before merge. Do not use
  `continue-on-error` or hide failures.
- Keep non-asserting long-running diagnostics out of default correctness gates;
  report intentional ignores explicitly and retain a manual invocation.

## Outcome (measured)

**Local gates PASS; hosted CI PENDING.** The original focused regression was
RED with `RECOVERY_REQUIRED: markerless database identity is missing` before
implementation. On Node 24.19.0, the worker suite now reports 28 tests: 22
passed, 0 failed, and 6 skipped because the pinned ONNX model snapshot is not
provisioned locally. The SQLite mutex tests passed for busy/retry and recovery
after its owner process exited; close-time contention preserved the owner and
allowed a successful retry. `cargo test --no-default-features` completed
with exit code 0; `zz_probe_discriminates` was visibly ignored in the default
run. Root `npm test` passed 29/29. Both core and default Clippy commands,
`cargo fmt --all -- --check`, `npm run docs:validate` (0 violations in 241
files), and `git diff --check` passed. Hosted PR checks and final gate remain
required before merge. The Astra architecture/review gate returned
`PASS_WITH_LIMITATIONS` with no remaining code blockers; hosted CI is still a
merge prerequisite.

**Residual path boundary:** Node `lstat`/`realpath` checks reject detected
symbolic links and junctions but do not prove that every Windows-specific
reparse tag is rejected. The deployment parent must remain application-
controlled; hostile concurrent path swapping would require native
handle-relative operations and is out of scope.
