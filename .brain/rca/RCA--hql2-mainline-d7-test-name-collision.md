# RCA: HQL1 D7 Mainline Test Name Collision

## Status / Date

Root cause confirmed; correction verified / 2026-10-05.

## Symptom

The merged HQL1 adapter integration test target fails to compile with Rust
`E0428`: `actor_scoped_hql1_zero_hop_id_equality_matches_legacy_and_hql2` is
defined twice in `tests/hql2_hql1_adapter_tests.rs`.

## Evidence

- The `origin/main` test file already contains the unlabelled zero-hop ID-equality
  differential under that function name.
- The incoming D7 extension changes the test body to add labeled cases but keeps
  the same function name.
- Cherry-pick conflict resolution retained the mainline test and appended the
  incoming labeled test, producing two definitions. The compiler points to the
  definitions at lines 180 and 315.

## Root Cause

The incoming commit treated the existing test as a replacement, while the
mainline version was preserved as an independent regression test. Keeping both
behaviors requires distinct test names; the automatic conflict resolution
preserved both bodies without reconciling their shared name.

## Why the Issue Escaped Detection

The feature branch compiled only its replacement test. The duplicate arose
only when its patch was replayed onto the newer mainline file, and the adapter
target had not yet been run after that replay.

## Prevention Applied

Keep the richer Unicode/missing-ID differential and the mainline basic-ID
differential as distinct tests, preserve the separate labeled-filter target,
and run the adapter target after every conflict-resolution/replay step.

## Outcome

RED reproduced by `cargo test --locked --offline --no-default-features --test
hql2_hql1_adapter_tests` after replay (`E0428`). Renaming the preserved mainline
basic-ID differential removed the duplicate; the adapter target now passes
13/13, and the explicit root HQL2 sweep passes 404/0/1 across 39 targets.
