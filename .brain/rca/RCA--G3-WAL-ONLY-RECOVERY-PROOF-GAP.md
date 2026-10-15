---
status: active
superseded_by: null
---

# RCA — G3 WAL-only recovery proof gap

## Symptom

The G3 Rust differential/reopen target reports 2/2 passing cases, but that
result does not establish that the cases reopened from the authoritative WAL
without materialized snapshot or projection files.

## Evidence

- In tests/g3_oracle_differential_tests.rs, remove_materialized_state
  discards every fs::remove_file result with let _ = ...; it does not
  distinguish NotFound from permission, sharing, or other I/O failures.
- The helper does not assert that each listed materialized file is absent
  before the database is reopened.
- The temporal case records named frontiers during fixture setup, but neither
  recovery path captures the final pre-close stable frontier and compares it
  with the recovered frontier.
- Both paths compare pre-reopen and post-reopen query results. Those equality
  checks can pass if a materialized file was not removed.
- At pre-fix baseline `aed35b7f5dee320a703b41db035d30daa71139ee`, the focused
  target passed 2/2; this was result-comparison evidence only and predates the
  merged correction.

## Root Cause

The test's recovery precondition is best-effort rather than asserted: file
removal failures are swallowed, and the test has no recovered-frontier
assertion. Thus the test can pass without proving a WAL-only reopen that covers
all durable fixture mutations. This finding concerns test evidence; it does
not establish a runtime storage-engine defect.

## Why the issue escaped detection

The tests asserted golden-result equality before and after reopen, and their
names described a WAL reopen. They did not assert the fault precondition
(materialized files absent) or the durability postcondition (recovered stable
frontier covers the final pre-close frontier). Passing output equality was
therefore mistaken for proof of the recovery path.

## Proposed prevention

1. Make removal return an error; ignore only NotFound and fail on all other
   errors.
2. Assert every listed materialized file is absent before reopening.
3. Capture the final stable frontier after fixture mutations and before close;
   assert the reopened frontier is greater than or equal to it.
4. Retain exact pre/post query equality and fixture-golden equality.
5. Keep this correction test-only in tests/g3_oracle_differential_tests.rs,
   then run the focused target and its specified G1/G2/crash regression matrix.
6. Report WAL-only recovery as NOT_PROVEN until these assertions are
   implemented and pass; after they pass, claim only the bounded test proof.
   Do not promote it to power-loss, release, deployment, or broad P7 evidence.

## Implementation follow-up — CI Clippy failure (2026-10-06)

### Symptom

PR #217's `fmt + clippy` check failed after the test-only implementation was
committed.

### Evidence

- The repository workflow runs
  `cargo clippy --no-default-features --all-targets -- -D warnings`.
- Clippy reported `clippy::io_other_error` at the new absent-file assertion in
  `tests/g3_oracle_differential_tests.rs`, where the code constructed
  `std::io::ErrorKind::Other` with `std::io::Error::new`.
- The exact workflow command reproduced the finding locally.

### Root Cause

The new assertion used the generic `Error::new(ErrorKind::Other, ...)`
constructor instead of the standard `std::io::Error::other(...)` constructor.
Because CI promotes warnings to errors, this lint failed the job.

### Why the issue escaped detection

The initial local verification ran formatting and test matrices but omitted the
workflow's Clippy commands. The independent verification gate therefore did
not exercise the same lint acceptance check as CI.

### Proposed prevention

1. Use `std::io::Error::other(...)` for this assertion.
2. Include both CI Clippy commands in future Rust verification:
   `cargo clippy --no-default-features --all-targets -- -D warnings` and
   `cargo clippy --all-targets -- -D warnings`.

## Resolution — PR #217 merged 2026-10-06

The test-evidence root cause is resolved for the bounded `g3.oracle.v1` proof.
The merged correction makes materialized-file removal fail closed, verifies
those files are absent before reopen, and checks that the recovered stable
frontier covers the final pre-close frontier. Exact query/oracle comparisons
remain in place.

PR #217 head `9221b74e7dd81c350e099ac6b4a034810971d9b0` merged as
`987bf32507af6e1f9cc612385db093b4358996ed`. Hosted Tests, Security Audit,
GenesisRAG17 Linux worker, Performance Audit, and Package Manager Consumer
checks passed. The Linux worker had 26 passes, 0 failures, and 6 skips because
the pinned ONNX model snapshot was absent. The former Clippy issue was fixed
with `std::io::Error::other(...)` and the hosted checks passed.

Closure is limited to the named test proof and CI checks. Physical power-loss,
mobile/device, migration, release, deployment, and production acceptance are
not established by this RCA resolution.

## Version Diff

| From | To | Change |
|---|---|---|
| none | 1.0.0 | Record the G3 WAL-only recovery test-evidence root cause and bounded prevention criteria. |
| 1.0.0 | 1.0.1 | Record the PR #217 Clippy failure, evidence, root cause, and lint-gate prevention. |
| 1.0.1 | 1.0.2 | Record resolution of the bounded G3 test-evidence gap on merged PR #217, including hosted checks and skipped-test limits. |
