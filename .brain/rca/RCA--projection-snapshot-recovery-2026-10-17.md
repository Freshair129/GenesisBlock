# RCA: Untrusted Projection Snapshot Skipped Recovery

## Status / Date

Follow-up fix in PR #239; exact-head CI pending / 2026-10-17.

## Risk

`HIGH` — this changes persistence recovery behavior and SQLite projection publication.

## Symptom

The mobile-host Rust suite failed in `edge_projection_tests::an_existing_database_is_backfilled_on_open`. After the test removed rows from `projection.sqlite`, reopening rejected the P6 snapshot because its recorded projection digest no longer matched. The database then served an empty `edges` projection (`0` rows instead of `3`).

## Evidence

- GitHub Actions run `38962281366`, job `117180050222`, at PR head `90e2f24ed830e0565ea788357dedde7f17ce7138` logged `SNAPSHOT_COMPONENT_INVALID: byte count or digest mismatch for projection.sqlite` before the assertion failed.
- `projection_journal_fingerprint` hashes the journal inputs, not SQLite contents. The stored marker therefore remained equal after the test changed SQLite.
- `try_load_state` rejected a snapshot with an invalid component before loading the otherwise-valid `edges.bin` state.
- `projection_sync_on_open` used the journal fingerprint marker as its repair shortcut. It had no signal that snapshot validation had rejected `projection.sqlite`, so it returned without replay or edge backfill.
- The earlier mobile-host run ended at the job timeout without a completed suite result; the later longer run produced the first recorded CI failure for this path.

## Root Cause

Snapshot integrity validation and projection replay used independent validity signals. Snapshot validation could reject `projection.sqlite`, another P6 component, or the snapshot itself, while projection synchronization only checked whether the journal changed. An unchanged cache marker could therefore suppress repair after a rejected snapshot. Legacy snapshots without a P6 projection digest also looked valid to the open path even though their projection integrity was unverified. Skipped snapshots (stale frontier or sidecar recovery) lost the same signal.

The read-only review found the mixed-component and legacy paths after the initial fix. Those paths are now treated as untrusted even when the snapshot loader returns before reporting projection status.

Exact-head macOS CI exposed two test-fixture issues. First, two metadata migration tests created schema-6 journal events and later rewrote only `state.json` to schema 4. A recovery attempt correctly rejected schema-6 record-revision events under the declared schema-4 contract. The fixtures now start from a schema-4 state before writing events, so they exercise a real legacy journal and snapshot instead of an inconsistent downgrade. Second, `old_transaction_projection_adds_frontier_kind_with_legacy_true_default` inserted a transaction receipt directly into an unverified legacy SQLite projection, with no corresponding journal transaction, then expected open to preserve that cache-only row. The recovery fix correctly discarded it during journal rebuild. The test now asserts the security contract: the fabricated receipt does not survive, and the rebuilt projection has the current column layout.

## Why Detection Failed

Earlier mobile-host CI did not complete the full suite, so those runs provided no completed signal for this path. Local and remote verification also needed to exercise the exact mobile-host feature set because snapshot validation and projection recovery meet during open.

## Prevention

- Preserve an untrusted-snapshot signal when another P6 component fails, the snapshot is skipped, or the snapshot has no P6 projection digest. Only a fully loaded snapshot with a verified projection digest can use the unchanged journal marker as a skip signal.
- Rebuild an untrusted projection into a fresh SQLite candidate from the journal, then backfill from loaded native state where the existing recovery path permits it. Do not copy the rejected SQLite file or treat the journal fingerprint as a projection integrity proof.
- Return `RECOVERY_REQUIRED` for read-only recovery and when a rejected, unloadable snapshot has recovery material but no journal events. An event-presence check is only a negative guard; finding an event does not prove journal completeness. Candidate replay and validation must still succeed before publication.
- Cover projection-only and mixed-component corruption, legacy snapshots without a digest, read-only rejection, and missing-journal failure. Keep the mobile-host suite timeout long enough to reach the Rust integration tests.
- Build legacy migration fixtures from the legacy schema at creation time; do not convert schema-6 journals into schema-4 fixtures by editing only `state.json`.
- Never treat projection-only receipts as authoritative during recovery. Tests of recovery must assert replay from the journal; schema migration fixtures must preserve the journal as the source of truth.

## Outcome

Local focused validation on Windows passed after the follow-up fix:

- `cargo test --no-default-features --features "mobile ffi android-jni" --test edge_projection_tests -- --test-threads=1` — 10 passed, 0 failed.
- `cargo test --no-default-features --features "mobile ffi android-jni" --test wal_tail_replay_tests -- --test-threads=1` — 5 passed, 0 failed.
- `cargo test --no-default-features --features "mobile ffi android-jni" --test projection_replay_recovery_tests -- --test-threads=1` — 6 passed, 0 failed.
- `cargo test --no-default-features --features "mobile ffi android-jni" --test epoch_e2_tests -- --test-threads=1` — 8 passed, 0 failed.
- `cargo test --no-default-features --features "mobile ffi android-jni" --test meta_format_migration_tests -- --test-threads=1` — 4 passed, 0 failed.
- `cargo test --no-default-features --features "mobile ffi android-jni" --test schema6_migration_tests -- --test-threads=1` — 19 passed, 0 failed after aligning the stale-projection fixture with journal-authoritative recovery.
- `cargo build --no-default-features --features "mobile ffi android-jni"` — passed.
- `cargo fmt --all -- --check` and `git diff --check` — passed.

Exact-head GitHub Actions validation and merge remain pending. On PR head `283e52e6736daaa02c837f32d62b0893b78474e2`, macOS job `117212797568` in run `38972137113` caught the cache-only receipt expectation above. The corrected `schema6_migration_tests` suite passes locally on Windows (19 passed, 0 failed); exact-head CI must rerun after the follow-up is pushed.
