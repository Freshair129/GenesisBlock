# RCA: Relational schema upgrade and folded journal replay

## Status / Date

Suspected from source inspection; runtime confirmation on GenesisBlock main is blocked by Cargo admission. 2026-11-07. Base `e99395ae510e87e2f82ad79cc1cfd9461e61e6be`.

Round 1 RCA and regression tests were approved by Claude, the lead agent acting on the owner's instruction to fix Freshair129/Lalin-AI #46. This is **agent approval, not human approval**. Production changes and contract deltas remain pending owner approval.

## Symptom and evidence

The issue reports `projection sync failed (REL_SCHEMA_VERSION_CONFLICT: mutation schema version is not current); fresh WAL rebuild failed: REL_SCHEMA_VERSION_CONFLICT: initial schema must be version 1`, on dependency rev `14de32a` with frontier-only retention after v1 rows and v2/v3 upgrades. This is external prior evidence, not a run at the current base. The new `relational_schema_upgrade_replay_tests` contains same-session, later-session, full, budget, after-upgrade-only controls and version-99 guards. Its checkpoint prints the actual signed journal events before reopen. None has been compiled or run; the dump and per-case outcomes remain unobserved. The explicit `save_state` is an extra checkpoint before drop to expose the journal; the control still checks two subsequent reopens. The `budget:1` case does not yet demonstrate that a fold occurred.

## Root cause (source-supported hypothesis)

On open, `projection_sync_on_open` uses the fingerprint and schema preflight to choose an existing candidate or fresh candidate (`src/lib.rs:14775`, `:14795`, `:14805`). Existing candidate replay failure is retried from a blank candidate (`:14835`-`:14875`). Both replay the journal (`:14520`-`:14545`).

A fold serializes relational state (`src/lib.rs:30969`-`:30976`). `relational_snapshot_events` reads only current registry packages and emits each current package followed by canonical live row upserts tagged with the current schema (`:15846`-`:15924`), then emits separate empty-mutation idempotency markers with their original recorded schema versions (`:15927`-`:15947`). Thus source predicts a v3 package, v3 live row and v1 marker for a v1 batch upgraded to v3. This is not a measured journal dump. The marker is the crucial distinction from a historical row payload.

In an existing v3 candidate, replaying a v1 marker fails the current-version check (`src/lib.rs:14001`-`:14015`) before the marker identity check; fresh replay of a folded v3 package into an empty registry fails the initial-v1 requirement (`:12611`-`:12625`). The equivalent checks at `:15075`-`:15090` and `:15134`-`:15163` govern **live** schema registration and batch application, respectively, and must not be conflated with replay. Whether these branches and exact combined error occur in this checkout requires the blocked test run.

`full` avoids checkpoint folds (`:27123`-`:27135`), but this alone does not establish whether the replay implementation handles retained v1→v3 history; pending runtime result. Explicit compact can fold even under full. The existing `projection_replay_recovery_tests::upgraded_relational_schema_reopens_and_preserves_rows` uses `full` and removes a marker, but its historical result is not current validation and it does not cover this folded marker case.

## Why detection failed / prevention

The existing upgrade test covers full retained schema history, not a fold containing both materialized current rows and an old-version idempotency marker. Add the new cases to regular integration coverage after fixing and validate fresh journal-only recovery, repeat reopen, marker idempotency/conflicts and rejection of invalid versions, including both full and actual budget-fold cases. The test's journal print requires review before claiming observed folded contents.

## Proposed fix (pending approval)

Risk R8: **HIGH**, because the change affects recovery and durable journal interpretation. Complexity R10: **C-3**, because it crosses the U2 relational and journal/fold recovery contracts and requires architecture review. Round 1 diagnostic work is C-2; no production design has been approved.

Propose a replay-only distinction between authenticated folded-base materializations and ordinary history. Permit a validated current package as a fresh base bootstrap only when the journal's verified base context proves it is a base materialization; never loosen live initial registration. Treat folded empty-mutation idempotency markers as markers, preserve their original version/hash/identity and apply them without treating them as row mutations, with collision and namespace checks intact. Continue strict live version checks and reject arbitrary unrecognized versions in ordinary history. For retained history use ordered fresh-candidate replay where needed rather than assuming version numbers prove lineage; verify previous-version chain and signatures at the applicable boundary. Evaluate preexisting folded data and base provenance, missing/corrupt package, marker collision and mixed base/history before choosing implementation. Do not infer that a smaller number is an ancestor. Avoid a journal format change if the existing authenticated segment context suffices; otherwise return for a specific scope decision.

Proposed documentation deltas, not approved or applied: U2 `docs/SPEC--GENESISDB-RELATIONAL-APPLICATION-CONTRACT-U2.md` sections 6.1, 7.1 and replay/idempotency: specify separate base-bootstrap and marker semantics while retaining live v1 and upgrade validation. `docs/SPEC--GENESISDB-JOURNAL-FORMAT-V1.md` section 8 and `docs/adr/ADR--GENESISDB-JOURNAL-HISTORY.md` D3/D5: define proof and ordering for base materializations, existing base compatibility, and independent journal reconstruction. Review `docs/C4--GENESISDB-ARCHITECTURE.md` and `docs/MASTER-SPEC--GENESIS-DB.md` for actual impact; update only if warranted. Owner approval is required before implementation or contract edits.

## Outcome and admission

No fix or post-fix result. Focused tests are unrun and not claimed passing or failing. The lane's free-byte measurements, lock status and unresolved §J admission inputs are recorded in the handoff; resolve those inputs, rerun admission immediately before Cargo, then execute the tests and capture the printed journal and errors. No cleanup is authorized.
