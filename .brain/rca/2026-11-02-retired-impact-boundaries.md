# Retired importance policy and authenticated compatibility boundaries

Risk: HIGH / C-3. Scope: the owner-approved K-Impact retirement and GKS ontology pilot. These defects were found while validating the uncommitted retirement candidate, before release.

## Symptoms and evidence

Removing the public `impact` field exposed three boundary failures in intermediate candidates:

- Public DTO serialization could make two authenticated historical payloads with different numeric impact values compare equal. An overly broad revision comparator also accepted a projection that dropped the historical value.
- A genuine old lean snapshot keeps `props:null` in `nodes.bin`, with application properties in SQLite. Normalizing before SQL hydration lost application properties; a matching projection marker skipped replay. Swallowed hydration errors could also turn conflicting or missing historical properties into an apparently valid empty view.
- The first NAPI conversion macro forwarded a Rust `ty` fragment into `#[napi(object)]`. NAPI did not recognize the opaque `Option` type and rejected a current search request with `Missing field alpha`. A Rust compile check had passed; the real Node retirement regression failed.

## Root causes

Authenticated wire representation, hydrated storage projection, and current public DTO have different contracts. Reusing the public DTO as an old-wire comparator erased authenticated field presence/value information. Migrating lean metadata as if it were complete storage data ignored the existing split between snapshot metadata and verified SQL properties. Macro type fragments hid syntax that NAPI's object parser needs to identify optional properties.

## Corrections

Use explicit `LegacyImpactWire::{Absent,Null,Number}` and exact legacy serializers for signature/hash/proof verification. Accept normalized revision equivalents only when they preserve the historical value. Normalize application views after verified SQL hydration, preserve existing properties, and reject conflicts or missing required projection rows at open. Preserve raw protected SQL revision payloads. Use explicit concrete NAPI DTO declarations; the conversion macro handles field names only.

## Why earlier checks missed the failures

Newly generated snapshots did not reproduce the old lean format. Presence-only projection checks did not prove complete hydrated properties. Tests using the public DTO as an oracle could miss the same value loss as the implementation. Compile-only NAPI checks did not exercise generated JavaScript argument conversion.

## Prevention and verification

`src/legacy_kimpact_tests.rs` covers absent/null/numeric signed wire round trips, tampered proof/revision values, original SQL payload preservation, old lean snapshots with matching markers, conflict/missing-row rejection, replay/save/reopen, and non-object/conflicting property migration. `__test__/db.test.mjs` exercises the newly built local addon with an omitted alpha, explicit retired inputs, public field removal, and equal-vector relevance unaffected by generic legacy metadata. Core and NAPI configurations must both pass Clippy; test results and qualification limits are recorded with the delivery evidence.

## Follow-up: schema 5 historical traversal consulted schema 6 revisions

The final no-fail-fast Rust suite found one additional schema-boundary regression in `tests/wave_b_correctness_tests.rs::transaction_and_consensus_rebind_share_current_and_historical_endpoints`. Its schema-5 fixture has valid `node_versions` history but no `hql2_record_revisions` table. Historical traversal called `legacy_node_version_props_from_revision` unconditionally and turned SQLite's missing-table error into `RECOVERY_REQUIRED` for node `B`.

`Storage::legacy_node_version_props_from_revision` now checks `storage_schema_version` at the migration boundary. Schema 5 returns `Ok(None)` because it has no HQL2 revision payloads to migrate. Schema 6 still executes the lookup and propagates missing-table/database errors, preserving fail-closed behavior rather than treating projection corruption as empty history.

The focused Wave B test and the new `legacy_revision_migration_is_schema_gated_and_v6_errors_stay_fail_closed` unit test pass. The unit test verifies both that schema 5 skips a missing revision table and that the same missing table in schema 6 remains an error. The final full no-fail-fast Windows rerun passed 1,320 tests with zero failures and six existing ignored diagnostics. The test profile used optimization level 1 with debug assertions and overflow checks enabled. Independent review found no weakening of schema-6 error handling. This local result is separate from hosted/platform/release qualification and the independently reproduced pre-existing intermittent P6 compaction failure.
