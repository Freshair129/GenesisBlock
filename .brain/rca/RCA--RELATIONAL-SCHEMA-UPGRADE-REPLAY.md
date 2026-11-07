# RCA: Relational schema upgrade and folded journal replay

## Status / Date

**Status: implemented and compile-checked; functional test round pending.** Root cause was reproduced on `fix/relational-replay-schema-lineage` @ `8588aa9` (base main `e99395a`), 2026-11-07 13:07–13:09 ICT, lane `F:\cargo-lanes\genesisblock-issue46` (see "Observed results"). Round 2 implementation started from `a844c61`.

**Owner approval (2026-11-07):** The human owner approved the replay-only marker handling, verified base-segment bootstrap rule, and listed documentation deltas. Approval was relayed by the lead. This supersedes round-1 agent approval as authorization for the production change. Risk R8: **HIGH**; complexity R10: **C-3**.

## Symptom and evidence

The issue reports `projection sync failed (REL_SCHEMA_VERSION_CONFLICT: mutation schema version is not current); fresh WAL rebuild failed: REL_SCHEMA_VERSION_CONFLICT: initial schema must be version 1`, on dependency rev `14de32a` with frontier-only retention after v1 rows and v2/v3 upgrades. Round 1 reproduced both failures in `relational_schema_upgrade_replay_tests` and printed the signed folded journal before reopen. The explicit `save_state` exposes the journal; the control still checks two subsequent reopens. The `budget:1` case does not demonstrate that a fold occurred.

## Observed results (lead run, 2026-11-07)

`cargo test --no-default-features --locked --test relational_schema_upgrade_replay_tests -- --nocapture`: **6 passed, 2 failed**.

| Test | Result |
|---|---|
| `frontier_same_session_upgrade` | FAILED — `projection sync failed (REL_SCHEMA_VERSION_CONFLICT: mutation schema version is not current); fresh WAL rebuild failed: REL_SCHEMA_VERSION_CONFLICT: initial schema must be version 1` |
| `frontier_later_session_upgrade` | FAILED — same error |
| `full_same_session_upgrade`, `full_later_session_upgrade` | ok |
| `budget_same_session_upgrade` | ok (fold not demonstrated for `budget:1`) |
| `frontier_rows_after_upgrade_control` | ok |
| `live_batch_rejects_invalid_schema_version`, `replay_rejects_invalid_schema_version` | ok |

Printed folded journal (frontier, same session): current `RelationalSchema` v3 → `RelationalRows` v3 with empty mutation identity and live row upserts → `RelationalRows` v1 with `mutation_id: 00000000-0000-4000-8000-000000000100`, non-empty `payload_hash`, `affected_rows: 1`, and **empty `mutations`** (the idempotency marker). This confirms the defect is specific to folds that retain a marker from a batch applied under an older schema version; both full-retention controls passed.

## Root cause (confirmed)

On open, the pre-fix `projection_sync_on_open` used the fingerprint and schema preflight to choose an existing candidate or fresh candidate (`src/lib.rs@8588aa9:14775`, `:14795`, `:14805`). Existing-candidate replay failure retried from a blank candidate (`:14835`-`:14875`). Both replayed the journal (`:14520`-`:14545`).

A fold serializes relational state (`src/lib.rs:30969`-`:30976`). `relational_snapshot_events` reads current registry packages and emits each package followed by canonical live row upserts tagged with the current schema (`:15846`-`:15924`), then emits empty-mutation idempotency markers with their original recorded schema versions (`:15927`-`:15947`). The measured dump confirmed the v3 package, v3 live row and v1 marker for a v1 batch upgraded to v3. The marker is distinct from a historical row payload.

In an existing v3 candidate, replaying a v1 marker failed the current-version check (`src/lib.rs@8588aa9:14001`-`:14015`) before the marker identity check; fresh replay of a folded v3 package into an empty registry failed the initial-v1 requirement (`:12611`-`:12625`). The equivalent checks at `:15075`-`:15090` and `:15134`-`:15163` governed **live** schema registration and batch application. Round 1 runtime results confirmed the failures and the version-99 rejection guards; the live and replay guards must remain strict for non-marker events.

`full` avoids automatic checkpoint folds (`:27123`-`:27135`), and both full-retention upgrade controls passed in the measured round-1 run. Explicit compact can still fold under full. The existing `projection_replay_recovery_tests::upgraded_relational_schema_reopens_and_preserves_rows` uses `full` and removes a marker, so it does not cover the folded marker case.

## Why detection failed / prevention

The previous upgrade coverage exercised retained history but not a fold containing current materialized rows and an old-version marker. Prevention is the dedicated `tests/relational_schema_upgrade_replay_tests.rs` suite: retain the same-session/later-session frontier regressions, full-retention controls, after-upgrade-only control, and version-99 live/replay guards. The round-1 journal dump was reviewed and confirmed the marker shape. The `budget:1` case still does not prove a budget fold; round 3 must run the suite and report that limitation separately.

## Approved fix

Risk R8: **HIGH**; complexity R10: **C-3**. The owner approved the following replay-only design on 2026-11-07, relayed by the lead:

1. A `RelationalRows` event with empty `mutations` and non-empty `mutation_id` is an idempotency marker during projection replay. Preserve its original schema version, payload hash, mutation ID and affected-row count; skip only the current-schema-version check. Keep namespace lookup and the existing same-ID/different-hash conflict check. Non-marker replay rows retain the strict version check.
2. A first `RelationalSchema` package may initialize a namespace at version greater than 1 only when the verified journal origin says the event is from a folded base segment. Do not infer ancestry from version numbers and do not change the journal format.
3. Live `register_relational_schema` remains v1-first and upgrades advance once; `apply_relational_batch` still requires the current version with existing error strings.

Approved documentation deltas: U2 §6.1, §7.1, and replay/idempotency text; journal format §8; journal-history ADR D3/D5; review C4 and Master Spec and edit only if affected; add a changelog entry per repository convention. C4 and Master Spec were reviewed and left unchanged because no component, public API or architecture boundary changes.

## Implementation summary

Round 2 wires `projection_replay_wal` (`src/lib.rs:14706`) to `scan_journal_with_origin` (`:31562`), passing the origin into `projection_apply_event` (`:13975`) for both existing-candidate synchronization and fresh-candidate rebuild (`:14548`, `:14895`). The journal visitor reports `Base` only for a frame read from a base-kind segment (`:31682`); the sealed body is read through `read_segment_body` (`:5791`), which verifies its hash and CRC before replay. `projection_apply_schema_conn` (`:12612`) allows version >1 only for that origin and only when the namespace has no package yet. It does not infer ancestry from version numbers.

Replay markers are identified by empty `mutations` plus non-empty `mutation_id` (`src/lib.rs:14023`). Namespace lookup and same-ID/different-hash checks remain before insertion; marker schema version, payload hash, identity and affected-row count are persisted from the event. Non-markers keep the strict schema check. Live projection call sites and the two internal helper callers pass `None` (`src/lib.rs:17828`, `:26120`; `src/legacy_kimpact_tests.rs:492`, `:902`), leaving live registration and batch rules unchanged.

Documentation updated: U2 §§6.1, 7.1, 9.1 and 14.1; journal format §8; journal-history ADR D3/D5; and the Unreleased Fixed section. C4 and Master Spec were reviewed and left unchanged because no component, public API or architecture boundary changed.

## Verification and disposition

- Final `cargo fmt --check`: **passed** (2026-11-07 13:46 ICT).
- Final `cargo check --no-default-features --locked --tests`: **passed** (2026-11-07 13:48–13:49 ICT); this compiled test targets but did not execute them.
- The first compile-only check found two internal helper call sites using the old private signature. Both now pass `None`; the final compile-only check passed.
- `cargo test` was not run in this round by design. Round 3 owns execution of the relational replay regression suite and the remaining functional acceptance.
- Local commit and exact lane/lock measurements are in the round-2 handoff. No push or cleanup is authorized.
