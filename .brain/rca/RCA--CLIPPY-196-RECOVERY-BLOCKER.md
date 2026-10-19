# RCA: existing main code blocks strict Clippy during recovery

## Symptom and evidence

Strict core Clippy 0.1.96 fails both the G4 candidate and clean main `5eb2fa4`
at `src/router.rs:1097`, `src/lib.rs:28752` and `src/lib.rs:30311`.
The source files are unchanged by the archived G4 counter patch. The errors
are two `nonminimal_bool` expressions and one `drop_non_drop` closure drop.

## Root cause

The existing equivalent-but-negated Option predicates and an explicit closure
drop conflict with the installed strict lint set. `#![deny(clippy::all)]`
makes these diagnostics hard errors. They are a baseline integration blocker,
not evidence that the recovered counter calculations are incorrect.

## Why detection failed

Prior green CI belongs to older revisions/toolchains. It cannot substitute
for a fresh strict lint run on the recovery candidate.

## Remediation and prevention

Use the direct `is_none_or` predicates suggested by Clippy, preserving the
empty-property and expiry boundaries. Remove the redundant explicit closure
drop; Rust's non-lexical lifetimes end the borrow at its last use. Keep the
strict lint settings and rerun both configurations plus relevant REST,
gossip-membership and journal-recovery regression tests.

## Validation

Baseline failure reproduced locally on main and the G4 candidate. Fresh
post-fix strict lint, build and regression evidence is pending.
