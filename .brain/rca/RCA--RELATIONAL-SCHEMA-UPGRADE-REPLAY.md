# RCA: Relational schema upgrade and folded journal replay

## Status / Date

**Status: implemented; verification blocked by C: growth bound and unresolved concurrent workload.** Root cause was reproduced on `fix/relational-replay-schema-lineage` @ `8588aa9` (base main `e99395a`), 2026-11-07 13:07–13:09 ICT, lane `F:\cargo-lanes\genesisblock-issue46` (see "Observed results"). Round 2 implementation started from `a844c61`.

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


## Outcome ? round 3 (2026-11-07)

Disposition: **BLOCKED ? full-suite verification incomplete**. Resume only after the owner establishes and documents a qualified E: concurrent-growth bound and applicable preflight, then remeasure C:/E:/F: after acquiring the lane lock and rerun the full suite and required Clippy. No cleanup or push was performed.

- Initial focused command on `515f8a3`: **7 passed, 1 failed**. `replay_rejects_invalid_schema_version` used an empty-mutation, nonempty-ID event, which is a replay marker under the approved design, not an ordinary row event. The test was corrected to carry a real row mutation; no production code was changed.
- Expanded focused command: **11 passed, 0 failed, 0 ignored**. `budget_same_session_upgrade` now forces sealed history over the budget and asserts a fold horizon before reopen. New tests: `folded_marker_preserves_live_idempotency_conflict`, `folded_journal_rebuilds_fresh_projection_after_upgrade`, and `ordinary_history_rejects_initial_schema_above_v1`. The marker conflict case uses current schema v3 for the changed payload because live version validation precedes identity validation.
- Full `cargo test --no-default-features --locked`: interrupted after **13 completed binaries, 131 passed, 0 failed, 1 ignored**, plus five observed passing cases in an incomplete `concurrency` binary. No conclusion is drawn for its remaining case or the unrun binaries. Raw log: `F:\cargo-lanes\genesisblock-issue46\round3-full.log`. The stop is not a test failure, and no claim of pre-existing failure is made.
- `cargo fmt --check`: passed. Clippy: **NOT_RUN**, blocked by the E: bound; `.github/workflows/test.yml` requires core and default-feature `--all-targets` Clippy. The requested core `--tests` command remains pending. `npm test`: **NOT_RUN**; TESTING.md lists the Node suite but does not specifically require it for relational replay changes, and it requires an addon build.
- Lane preflight and measurements (bytes, ICT): focused initial lock 14:00:53?14:02:22: C 126,210,650,112 ? 126,205,554,688; E 132,369,629,184 ? unchanged; F 2,072,429,080,576 ? 2,072,010,342,400. Expanded lock 14:03:36?14:04:20: C 125,946,523,648 ? 125,905,522,688; E 132,369,629,184 ? unchanged; F 2,072,010,346,496 ? 2,072,167,886,848. Full lock 14:04:33?14:14:21: C 125,878,394,880 ? 125,786,451,968; E 132,369,629,184 ? 132,365,438,976; F 2,072,167,886,848 ? 2,045,919,399,936. Floors: C/E 53,687,091,200; F 107,374,182,400. Full-run E decrease **4,190,208** exceeded the lane's 0-byte E bound, despite remaining above floor; attribution is unknown. Full-run F free-space decrease was 26,248,486,912; observed target size peaked at least 30,101,357,563 bytes and observed temp at least 1,607,198 bytes (samples, not continuous peak instrumentation). All figures are volume free-space or sampled file-size measurements, not proof of isolated workload writes.
- An initial launch at 14:00:31?14:00:32 was interrupted by PowerShell treating Cargo stderr as an exception, before results; no Cargo/rustc process remained and the lock was released. The corrected focused invocation produced the result above.


## Outcome - continuation (2026-11-07)

Disposition: **BLOCKED - verification incomplete**. Resume after the lane owner qualifies the observed concurrent C: and F: growth and active consumers, updates the applicable evidence-backed bounds, and rechecks all volume reserves and lane ownership. Then run the full suite and both CI Clippy commands, `cargo fmt --check`, and the CI Node job with a separately qualified native-addon release build. No push was performed. The earlier round-3 blocked entry is historical; the E: bound was subsequently qualified in `LANE.md`.

- Initial edits were committed as `b17dd496` on `515f8a3`. The first continuation full suite failed `projection_replay_recovery_tests::failed_incremental_replay_does_not_mutate_the_live_projection`: its empty-mutation event became a marker under the approved contract. Baseline `e99395ae510e87e2f82ad79cc1cfd9461e61e6be` passed the same test (1/1), so the obsolete test fixture was changed to contain a real row mutation while preserving the atomicity assertions. Candidate focused rerun passed all 6 projection tests. This is a branch-induced test contract mismatch, not a demonstrated production failure.
- A subsequent full run failed three `hql2_execution_tests` with `QUERY_BUDGET_EXCEEDED` / `query_limit`. Baseline passed that binary 15/15 and candidate focused rerun passed 15/15; a third full attempt also passed that binary 15/15. The intermittent failure remains unclassified as to root cause; one baseline pass does not establish whether the underlying susceptibility pre-existed. No HQL fix was made.
- The third full attempt was stopped after observed C: free-space decrease exceeded the 2,147,483,648-byte lane bound. Completed binaries: **139**, **1,257 passed, 0 failed, 6 ignored**; `tx_as_of_wp22_tests` was in progress and remaining binaries and doc tests were not run. Its exit 1 reflects our stop, not a completed test failure. `relational_schema_upgrade_replay_tests` passed 11/11 and `projection_replay_recovery_tests` passed 6/6 in this attempt. Logs: `round3-resume-full.log`, `round3-final-full.log`, `round3-full-attempt3.log`, `round3-baseline.log`, `round3-baseline-hql.log`, and `round3-focused-followup.log` under `F:\cargo-lanes\genesisblock-issue46`.
- CI Clippy commands `cargo clippy --no-default-features --all-targets -- -D warnings` and `cargo clippy --all-targets -- -D warnings`: **NOT_RUN** after bound breach; `cargo fmt --check`: **NOT_RUN** on the final fixture edit (a prior round passed before that edit). `npm test`: **NOT_RUN**. Contrary to the earlier entry, `.github/workflows/test.yml` runs the Node job for every PR to main, requiring `npm ci --ignore-scripts`, `npm run build`, then `npm test`; the release addon build and concurrent growth need separate admission.
- Full attempt 1 lock 14:17:15-15:04:42 ICT: C 125,772,234,752 -> 124,414,615,552; E 132,365,434,880 -> 132,360,876,032; F 2,045,919,371,264 -> 2,040,790,831,104 bytes. Baseline projection lock 15:06:09-15:10:11: C 124,414,607,360 -> 124,391,755,776; E 132,360,855,552 -> 132,360,843,264; F 2,039,830,650,880 -> 2,035,865,776,128. Full attempt 2 lock 15:10:56-15:57:50: C 124,390,727,680 -> 125,819,224,064; E 132,360,843,264 -> 132,365,340,672; F 2,036,939,898,880 -> 2,032,384,487,424. Baseline HQL lock 15:58:46-16:02:51: C 125,818,695,680 -> 125,815,799,808; E 132,365,340,672 -> 132,365,340,672; F 2,033,298,489,344 -> 2,032,407,101,440. Candidate focused lock 16:03:23-16:05:17: C 125,815,799,808 -> 125,815,721,984; E 132,365,340,672 -> 132,365,340,672; F 2,032,755,048,448 -> 2,032,765,149,184. Full attempt 3 lock 16:05:36-17:23:49: C 125,815,697,408 -> 123,444,121,600; E 132,365,340,672 -> 132,365,291,520; F 2,032,760,430,592 -> 2,016,472,113,152. The triggering sampled C: value was 123,446,005,760 (decrease 2,369,691,648). Concurrent Cargo jobs outside this lane were observed; attribution and peak writes are unknown. Bounds per run C 2,147,483,648, E 1,073,741,824, F target 42,949,672,960 plus temp 4,294,967,296; floors C/E 53,687,091,200, F 107,374,182,400 bytes. Measurements are free-space snapshots, not isolated workload write totals. The exact temporary baseline worktree was clean, removed after each comparison, and its absence verified.

### Attempt 3 per-binary results

Counts are pass / fail / ignored. Incomplete and not-run binaries are not included in the totals.

| Binary | Passed | Failed | Ignored | Status |
|---|---:|---:|---:|---|
| `src\lib` | 72 | 0 | 1 | complete |
| `add_vector_tests` | 6 | 0 | 0 | complete |
| `anti_entropy_tests` | 7 | 0 | 0 | complete |
| `async_indexing_tests` | 4 | 0 | 0 | complete |
| `backup_restore_u9_tests` | 8 | 0 | 0 | complete |
| `batch_atomicity_tests` | 1 | 0 | 0 | complete |
| `binary_quant_tests` | 3 | 0 | 0 | complete |
| `bitemporal` | 10 | 0 | 0 | complete |
| `bitemporal_matrix_wp31_tests` | 7 | 0 | 0 | complete |
| `bq_centering_tests` | 5 | 0 | 0 | complete |
| `collection_ef_default_tests` | 4 | 0 | 0 | complete |
| `collection_name_validation_tests` | 3 | 0 | 0 | complete |
| `compaction_tests` | 1 | 0 | 0 | complete |
| `concurrency` | 6 | 0 | 0 | complete |
| `concurrent_writers_tests` | 2 | 0 | 0 | complete |
| `consensus_checkpoint_race_tests` | 2 | 0 | 0 | complete |
| `consensus_commit_tests` | 5 | 0 | 0 | complete |
| `consensus_vote_sig_tests` | 6 | 0 | 0 | complete |
| `core_engine_tests` | 5 | 0 | 0 | complete |
| `crash_simulation_tests` | 17 | 0 | 0 | complete |
| `crdt_sync_tests` | 4 | 0 | 0 | complete |
| `durability_slice0_tests` | 4 | 0 | 0 | complete |
| `durability_slice1_tests` | 5 | 0 | 0 | complete |
| `edge_interning_tests` | 10 | 0 | 0 | complete |
| `edge_projection_tests` | 10 | 0 | 0 | complete |
| `edge_u128_tests` | 2 | 0 | 0 | complete |
| `ef_search_tests` | 3 | 0 | 0 | complete |
| `ephemeral_nodes_tests` | 1 | 0 | 0 | complete |
| `epoch_e1_tests` | 5 | 0 | 0 | complete |
| `epoch_e2_tests` | 8 | 0 | 0 | complete |
| `f16_quant_tests` | 4 | 0 | 0 | complete |
| `g3_oracle_differential_tests` | 3 | 0 | 0 | complete |
| `gossip_membership_security_tests` | 30 | 0 | 0 | complete |
| `governance_tests` | 1 | 0 | 0 | complete |
| `graph_traversal` | 15 | 0 | 0 | complete |
| `grl_retrieval_tests` | 10 | 0 | 0 | complete |
| `hardening_tests` | 3 | 0 | 0 | complete |
| `hnsw_capacity_tests` | 2 | 0 | 0 | complete |
| `hnsw_recall_floor_tests` | 5 | 0 | 0 | complete |
| `hql` | 11 | 0 | 0 | complete |
| `hql2_annotation_acl_tests` | 12 | 0 | 0 | complete |
| `hql2_annotation_lookup_tests` | 7 | 0 | 0 | complete |
| `hql2_annotation_source_tests` | 7 | 0 | 0 | complete |
| `hql2_durable_revision_tests` | 15 | 0 | 0 | complete |
| `hql2_execution_tests` | 15 | 0 | 0 | complete |
| `hql2_expand_sequence_tests` | 11 | 0 | 0 | complete |
| `hql2_expand_tests` | 13 | 0 | 0 | complete |
| `hql2_explain_source_tests` | 5 | 0 | 0 | complete |
| `hql2_g4_contract_tests` | 2 | 0 | 0 | complete |
| `hql2_g4_plan_identity_tests` | 6 | 0 | 0 | complete |
| `hql2_g4_provenance_completeness_tests` | 5 | 0 | 0 | complete |
| `hql2_g4_scalar_counter_tests` | 22 | 0 | 0 | complete |
| `hql2_g4_source_counter_tests` | 4 | 0 | 0 | complete |
| `hql2_graph_oracle_tests` | 39 | 0 | 0 | complete |
| `hql2_history_change_tests` | 22 | 0 | 0 | complete |
| `hql2_hql1_adapter_tests` | 13 | 0 | 0 | complete |
| `hql2_hql1_labeled_filter_tests` | 2 | 0 | 0 | complete |
| `hql2_hql1_ordering_tests` | 2 | 0 | 0 | complete |
| `hql2_hql1_property_projection_tests` | 3 | 0 | 0 | complete |
| `hql2_hql_reference_differential_tests` | 2 | 0 | 0 | complete |
| `hql2_key_codec_tests` | 3 | 0 | 0 | complete |
| `hql2_lower_tests` | 19 | 0 | 0 | complete |
| `hql2_oracle_tests` | 29 | 0 | 0 | complete |
| `hql2_p8_completion_tests` | 14 | 0 | 0 | complete |
| `hql2_parser_tests` | 18 | 0 | 1 | complete |
| `hql2_pattern_constraint_oracle_tests` | 11 | 0 | 0 | complete |
| `hql2_pipeline_oracle_tests` | 25 | 0 | 0 | complete |
| `hql2_rank_oracle_tests` | 37 | 0 | 0 | complete |
| `hql2_request_tests` | 6 | 0 | 0 | complete |
| `hql2_restore_generation_tests` | 3 | 0 | 0 | complete |
| `hql2_revision_backup_restore_tests` | 1 | 0 | 0 | complete |
| `hql2_root_match_oracle_differential_tests` | 1 | 0 | 0 | complete |
| `hql2_row_history_tests` | 1 | 0 | 0 | complete |
| `hql2_scalar_execution_tests` | 31 | 0 | 0 | complete |
| `hql2_storage_aggregate_oracle_differential_tests` | 3 | 0 | 0 | complete |
| `hql2_storage_hql_join_oracle_differential_tests` | 3 | 0 | 0 | complete |
| `hql2_storage_join_oracle_differential_tests` | 1 | 0 | 0 | complete |
| `hql2_storage_set_oracle_differential_tests` | 3 | 0 | 0 | complete |
| `hql2_storage_source_tests` | 10 | 0 | 0 | complete |
| `hql2_value_tests` | 6 | 0 | 0 | complete |
| `hql2_vector_operator_tests` | 18 | 0 | 0 | complete |
| `hql2_vector_oracle_differential_tests` | 2 | 0 | 0 | complete |
| `hql2_vector_revision_tests` | 2 | 0 | 0 | complete |
| `hql2_wire_tests` | 18 | 0 | 0 | complete |
| `hql_collection_tests` | 6 | 0 | 0 | complete |
| `hql_cypher_tests` | 18 | 0 | 0 | complete |
| `hql_filter_tests` | 30 | 0 | 0 | complete |
| `hql_fuzz_tests` | 40 | 0 | 0 | complete |
| `hql_p0_tests` | 30 | 0 | 0 | complete |
| `index_coverage_tests` | 1 | 0 | 0 | complete |
| `jit_chunk_schema` | 7 | 0 | 0 | complete |
| `journal_format_tests` | 10 | 0 | 0 | complete |
| `journal_migration_tests` | 4 | 0 | 0 | complete |
| `journal_recovery_p4_tests` | 2 | 0 | 0 | complete |
| `merkle_convergence_tests` | 6 | 0 | 0 | complete |
| `meta_format_migration_tests` | 4 | 0 | 0 | complete |
| `multi_collection_tests` | 6 | 0 | 0 | complete |
| `napi_rest_parity_tests` | 10 | 0 | 0 | complete |
| `neighbors_direction_rels_tests` | 9 | 0 | 0 | complete |
| `node_interning_tests` | 4 | 0 | 0 | complete |
| `node_meta_a2_tests` | 2 | 0 | 0 | complete |
| `node_versions_wp21_tests` | 5 | 0 | 0 | complete |
| `p6_generation_tests` | 6 | 0 | 0 | complete |
| `p6_lease_tests` | 10 | 0 | 0 | complete |
| `p6_peer_authority_tests` | 15 | 0 | 0 | complete |
| `p6_snapshot_authority_tests` | 6 | 0 | 0 | complete |
| `p6_visibility_tests` | 15 | 0 | 0 | complete |
| `persistence_tests` | 1 | 0 | 0 | complete |
| `projection_reopen_tests` | 1 | 0 | 0 | complete |
| `projection_replay_recovery_tests` | 6 | 0 | 0 | complete |
| `quantization_tests` | 4 | 0 | 0 | complete |
| `query_ir_tests` | 11 | 0 | 0 | complete |
| `readonly_sql_surface_tests` | 11 | 0 | 0 | complete |
| `rebuild_from_truth_tests` | 10 | 0 | 0 | complete |
| `relational_schema_resource_limit_tests` | 4 | 0 | 0 | complete |
| `relational_schema_upgrade_replay_tests` | 11 | 0 | 0 | complete |
| `relational_u2_contract_tests` | 5 | 0 | 0 | complete |
| `relational_u2_tests` | 5 | 0 | 0 | complete |
| `rerank_tests` | 6 | 0 | 0 | complete |
| `rest_api` | 13 | 0 | 0 | complete |
| `rest_api_tests` | 48 | 0 | 0 | complete |
| `rest_index_coverage_tests` | 2 | 0 | 0 | complete |
| `retention_wp13_tests` | 4 | 0 | 0 | complete |
| `retract_edge_tests` | 5 | 0 | 0 | complete |
| `robustness` | 12 | 0 | 0 | complete |
| `schema6_migration_tests` | 19 | 0 | 0 | complete |
| `schema_version_tests` | 3 | 0 | 0 | complete |
| `sidecar_migration_tests` | 2 | 0 | 0 | complete |
| `sidecar_ondisk_tests` | 6 | 0 | 0 | complete |
| `sidecar_reader_tests` | 2 | 0 | 0 | complete |
| `soak_tests` | 0 | 0 | 3 | complete |
| `sq8_calibration_tests` | 4 | 0 | 0 | complete |
| `sqlite_substrate_s0_tests` | 8 | 0 | 0 | complete |
| `state_transition_tests` | 1 | 0 | 0 | complete |
| `storage_lock_tests` | 3 | 0 | 0 | complete |
| `storage_reliability` | 13 | 0 | 0 | complete |
| `studio_contract_tests` | 6 | 0 | 1 | complete |
| `temporal_queries_tests` | 2 | 0 | 0 | complete |
| `thai_fuzzy_tests` | 1 | 0 | 0 | complete |
| `tx_as_of_wp22_tests` | - | - | - | interrupted |
| `uee_g0_contract_tests` | - | - | - | NOT_RUN |
| `unified_transaction_p5_tests` | - | - | - | NOT_RUN |
| `unified_transaction_u3_tests` | - | - | - | NOT_RUN |
| `vector_collections` | - | - | - | NOT_RUN |
| `vector_memory_metrics_tests` | - | - | - | NOT_RUN |
| `wal_compaction_tests` | - | - | - | NOT_RUN |
| `wal_tail_replay_tests` | - | - | - | NOT_RUN |
| `wave_a_commit_tests` | - | - | - | NOT_RUN |
| `wave_b_collection_lifecycle_tests` | - | - | - | NOT_RUN |
| `wave_b_correctness_tests` | - | - | - | NOT_RUN |
| `wave_b_failure_tests` | - | - | - | NOT_RUN |
| `wave_b_sync_tests` | - | - | - | NOT_RUN |
| `wave_c_query_correctness_tests` | - | - | - | NOT_RUN |
| `wave_d_budget_tests` | - | - | - | NOT_RUN |
| `wave_d_quality_artifact_tests` | - | - | - | NOT_RUN |
| `wave_d_rest_tests` | - | - | - | NOT_RUN |
| `wave_e_context_tests` | - | - | - | NOT_RUN |
| `wave_e_rest_tests` | - | - | - | NOT_RUN |
| `wp23_semantics_tests` | - | - | - | NOT_RUN |
| `zz_probe_discriminates` | - | - | - | NOT_RUN |
| `doc tests` | - | - | - | NOT_RUN |
| **Completed total (139 binaries)** | **1257** | **0** | **6** | **incomplete suite** |
