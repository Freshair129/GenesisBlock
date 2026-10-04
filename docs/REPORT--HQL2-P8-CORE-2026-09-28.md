---
doc_id: REPORT--HQL2-P8-CORE-2026-09-28
version: "0.1.63b"
created_at: "2026-09-28T04:35:00+07:00,ATHER,22bc11e"
last_update: "2026-10-04T16:06:00+07:00,ATHER"
status: beta
owner: GenesisBlockDB Engineering
attributes:
  domain: query-execution
  scope: P8 core HQL/IR implementation checkpoint
  risk: HIGH
  complexity: C-3
---

# HQL2 P8 core checkpoint — not full HQL2 completion

## Authority and base

The owner approved P8 typed-boundary 0.1.1b and isolated upstream integration
with `approve` on 2026-09-28. Local merge `22bc11e` joins upstream `8091a56`
with preserved local P6 `fc851e9` and P7 `e20e0e4`. At the 0.1.29b checkpoint,
the primary checkout was not changed. Protected `tests/zz_probe_discriminates.rs`
was not read or changed, and was not part of the HQL2 feature commit. One malformed target-discovery invocation was interrupted after
invoking Cargo without explicit test filters; whether it began compiling the
protected probe is unverified, and it contributes no test result. The guarded
explicit sweep below excludes it; see local RCA `.brain/rca/RCA--HQL2-POWERSHELL-TARGET-SELECTION.md`.
At that checkpoint there had been no fetch, push, main-branch merge, PR,
deployment or user database migration.
Engine version 0.2.6 -> 0.2.9 is inherited from upstream, not a new release here.

On 2026-10-02 the owner approved the P8 completion addendum D1-D6. This
authorizes implementation of bounded text profiles, provenance-preserving
ContextPack, Sequence node/edge properties and contextual literals while
preserving P6 authorization-before-parse. D1-D5 now have local runtime and
focused regression evidence in this isolated worktree. An independent
read-only implementation review found no concrete static defect, and the D5
size-accounting fix passed confirmation review. Broader P8/P13 gates remain
open; this evidence does not close them.
The owner-approved D7 bridge now supports actor-scoped, differential-tested
zero-hop HQL1 ID projections (unlabeled or with one plain-ASCII label) and
bounded unlabeled one-hop projections through `Storage::query_v2`, after P6
`Namespace(Read)` and namespace equality checks. Either zero-hop form may add
one exact same-alias string-ID equality predicate. The 11/11 adapter, 2/2
ordering and 2/2 labeled-filter targets cover legacy/HQL2 parity, match/miss,
wrong-label exclusion, a backslash/Unicode ID, parallel-edge multiplicity,
directions and wildcard relations, endpoint-ID filters/order, authorization
and namespace checks before parse, malformed syntax, fail-closed unlisted
forms and pre-parse resource rejection. The explicit 35-target HQL2 sweep
passes 394/0/1; selected P6 peer tests pass 45/0/0 across seven targets.
Existing v1 transports and all other HQL1 forms remain unchanged. Independent
read-only review found no runtime parity or authorization defect. Broader D7
review and full shared-runtime/P8/P13 acceptance remain open.

PR #208, “HQL2: lower HQL1 zero-hop ID equality,” was merged normally on
2026-10-04; no administrative bypass was used. Its hosted checks finished
with 40 success, 5 failures and 5 skipped. Four failures were worker bootstrap
jobs across Ubuntu, Windows and macOS, including the rebuilt Linux addon job;
their logs report missing markerless database identity. The fifth was the
Windows Rust suite failing
`annotation_lookup_uses_the_selected_transaction_frontier` with
`QUERY_BUDGET_EXCEEDED`. An isolated rerun of that test on Windows passed
1/1 in 0.31 seconds, so the full-suite failure is not reproduced in isolation;
its cause remains unconfirmed. This does not make hosted CI green or close the
worker issue. The local HQL2 sweep was reported as 393/0/1 at that checkpoint;
a fresh complete rerun at the subsequent checkpoint measured 391/0/1. The
run including the ChangeScan cursor-window test at that checkpoint measured
392/0/1.
P6/ACL/schema/transport contracts were not changed, and broad
shared-runtime/P8/P13/review gates remain open.

PR #210, “test(hql2): add source-scan P7 differentials,” merged normally at
`e9fba8c` on 2026-10-04. Its [hosted checks](https://github.com/Freshair129/GenesisBlock/actions/runs/37179964772)
finished with 11 success, 4 failures and 1 skipped. Ubuntu/macOS/Windows Rust,
regular npm, documentation, version consistency, fmt/clippy, RustSec and
Scientific Performance checks passed. The four failures were worker jobs
reporting missing markerless database identity; the scheduled advisory check
was skipped. This does not close the worker issue or establish checks for the
current change; no administrative bypass was used.

PR #212, “test(hql2): verify ChangeScan cursor windows against P7,” is merged
normally. Its hosted checks finish with 10 pass, 5 failures and 1 skipped. Four
failures are worker bootstrap jobs reporting missing markerless database
identity; the Windows Cargo test job failed after 15m13s, with the exact
failure detail unconfirmed. The relevant isolated Windows query-budget test
passed locally 1/1, but that does not resolve the suite failure or make hosted
checks green. PR #213 code commit `1af723e` later finished with 40 passing
checks, four worker failures, five skipped and one Windows Cargo suite canceled
at the configured 15-minute job limit. Worker check annotations report only
exit code 1, so their causes remain unconfirmed. Ubuntu/macOS full Cargo tests
passed; Windows full Cargo is incomplete, while the local Windows HQL2 sweep
passes 394/0/1. No administrative bypass was used.

The approved H2-D11 R6b recovery contract is now implemented in the isolated
worktree: signed local schema-v6 activation is preflighted before WAL replay,
and migration authority survives fold and markerless projection rebuild.
Crash recovery passes 17/17, schema-v6 migration passes 19/19, and the selected
HQL2/durability/authority aggregate passes all 40 targets (one parser probe is
intentionally ignored). Verification uses temporary fixtures only; no user
database was migrated. Full P8/P13, shared-runtime and independent-review gates
remain open.

At the prior checkpoint, the locked/offline no-default-features Rust suite
passed with `--jobs 4 --no-fail-fast -- --skip probe_vs_recall`; default and
no-default strict all-target Clippy also passed. For this ACL/budget change,
the focused suites and explicit HQL2/P6 sweeps recorded below pass; a fresh
full-suite rerun excluding `probe_vs_recall` has no completion result recorded
in this checkpoint. An earlier
unrestricted attempt entered that protected probe and was interrupted after it
ran for over 60 seconds; no result is claimed for the probe. Existing ignored
soak/parser-child cases remain ignored. `cargo fmt --check`, `git diff --check`,
and `npm run docs:validate` passed (240 files, 0 violations). This remains
fixture/local evidence only: independent review, full shared-runtime/P8/P13,
release and consumer qualification are not claimed.

## Implemented scope

- `src/query/hql2/`: full pinned grammar parser and spanned source tree; typed
  scalar lowering; all 23 closed operator configurations; iterative DAG checks;
  semantic scalar binder, deterministic physical plan, native typed kernels;
  safe errors and closed result encoding. No oracle imports in production.
- Exact runtime operators: Values, Filter, Project, Distinct, Sort, Take,
  Offset, UnionAll, Aggregate, Join (inner/left/semi/anti). Scalar/list parameter
  values are explicitly typed; I64 decimal-string encoding is lossless.
- `Storage::query_v2`: authorization before parsing, one continuous commit
  lock across catalog/bind/plan/pin/execute/final validation. EXPLAIN is catalog
  only and never invokes generation publication. Execution uses one real P6
  lease; snapshot identity derives only from the public verifying key.
- `ReadView::hql2_catalog` is private and lease-validated. Its borrowed result
  cannot escape the guard or expose Storage. Node/Edge/Row/Annotation source
  scans, annotation lookup, and typed selective hydration now execute under
  the same validated P6 lease; see the current source-target evidence below.
- Conservative query-local memory reservations, deadline, result-row/byte and
  collect bounds; exact execution fails on exhaustion, never returns partial
  aggregates. ANALYZE emits measured row/time counters only; unknown counters,
  costs and index frontiers remain absent.
- Sequence ID, conjunctive labels and D4 node/edge property predicates bind
  and execute through HQL and typed IR under the same P6 graph snapshot.
  Property values use exact JSON equality; missing values do not match, explicit
  JSON null matches only explicit null, and filtering precedes SHORTEST. Compact
  remains closed. Addendum D1-D5 also has focused evidence for pre-parse ACL,
  registered lexical/context profiles, exact ContextPack evidence and
  contextual literals; the focused P8 completion target passes 10/10,
  including edge-property filtering before SHORTEST.

These are Rust core changes only. REST/NAPI/FFI/SDK/MCP remain unchanged on
their existing compatibility contracts; P13 parity is deliberately not claimed.

## Verification record

Platform: local Windows MSVC, Rust 1.97.1, `--locked --offline
--no-default-features`, target directory
`D:\CodexBuilds\GenesisBlock-HQL2-2026-10-02\preserved-target-hql2-execution`.
The latest 35-target HQL2 run was executed in the isolated G: worktree with
explicit test targets; the protected probe target was not selected.

| Gate | Measured result | Limit |
|---|---|---|
| Upstream integration prerequisites | 41 passed: G0/P6 36 + index coverage 1 + relational schema limits 4 | Local merge baseline only |
| P8 first integrated revision | 78 passed: storage 10, parser 13, request 5, scalar 27, value 5, wire 18 | Before last resource-review corrections; not final acceptance |
| Compatibility and preserved prerequisites | 306 passed: legacy HQL/IR 141 + G0/P6 36 + P7 129 | Exact scoped targets, not full suite |
| Final focused P8 scalar revision | 98 passed: storage 12, parser 18, lower 12, request 6, scalar 27, value 5, wire 18 | Final Error.reason source included; not full P8 |
| Independent review | No outstanding finding in reviewed scalar/resource boundary | No full P8 approval inferred |
| First full native Rust attempt | 142 passed, 1 failed before fail-fast stopped at epoch_e2 | Legacy snapshot fixture retained a stale P6 manifest; not a full-suite pass |
| Default-feature library check | cargo check --locked --offline --lib --jobs 1 passed | Final Error.reason field compiles with NAPI; no addon runtime claim |
| Final P8/P6 and corrected fixture rerun | 141 passed: P8 98 + P6 31 + epoch/metadata 12 | Default test profile; zero failures, allocator child entrypoint handled as described below |
| Full no-fail-fast native rerun | 913 passed, 1 failed, 4 ignored across 118 harness results; exit 1 | Sole failure: wal_tail_replay legacy fixture retained P6 while removing its required frontier; see correction below |
| Final WAL fixture/P6 rerun | 48 passed, 0 failed, 0 ignored; exit 0 | WAL 5 + epoch/metadata 12 + P6 31; production code unchanged since the full run; not a second full-suite invocation |
| Exact vector operator target | 12 passed, 0 failed | HQL/IR exact-KNN parity, Original Rerank, missing-original behavior, typed Score, collection metric/fingerprint and dimension/space validation, candidate budget, zero-cosine rejection, and approximate-KNN fail-closed behavior; fixture-backed core evidence only |
| H2-D11 focused regression sweep | 62 passed across 8 targets: durable revisions 15, annotation ACL 8, annotation lookup 4, annotation source 3, key codec 3, storage sources 8, vector revisions 2, schema-v6 migration 19 | Separate from the 154-test P8 core matrix; isolated fixtures only, no user-database migration, and not P8 operator closure |
| Blueprint semantic oracle | 26 Python fixture tests passed, including distinct annotation target/evidence validation | Independent fixture oracle only; it does not execute the Rust HQL2 runtime |
| Explicit all-HQL2 target sweep (prior checkpoint) | 294 passed, 3 failed, 1 ignored across 21 named integration targets | All three failures are `query_v2` KNN/Rerank requests rejected as `operator_unavailable` at bind; vector-revision (2) and wire (18) targets were run separately after Cargo stopped at the failing vector target. Oracle targets are included, so this is regression evidence, not P8 acceptance. Protected probe was not selected. The two focused source tests below were added/run afterward and are not included in this count. |
| Explicit all-HQL2 target sweep (after RowScan fix; historical) | 296 passed, 3 failed, 1 ignored across all 21 root `hql2_*` integration targets | At that checkpoint the RowScan HQL/IR regression was green and three vector operator tests were rejected at bind as `operator_unavailable`. Superseded by the later 307/0/1 sweep. Parser's ignored child entrypoint was exercised by its parent. Protected probe was excluded. |
| Explicit all-HQL2 target sweep (after exact vector execution; historical) | 307 passed, 0 failed, 1 ignored across all 21 root `hql2_*` integration targets | Superseded by the final 22-target sweep below. Offline, locked, no-default-features, jobs=1; protected probe excluded. Oracle targets are included, so this is regression evidence, not P8 acceptance. |
| HistoryScan/ChangeScan focused target | 9 passed, 0 failed | HQL/IR parity, exact historical hydration, tombstones, source floors, vector-owner events, recursive annotation/edge ACL, unknown families, future bounds, and fail-closed budgets. |
| Recursive annotation-source ACL target | 6 passed, 0 failed | Nested annotation references and Edge endpoints are re-authorized; corrupt/inaccessible references fail closed. |
| Final explicit all-HQL2 target sweep (before Match anchors) | 318 passed, 0 failed, 1 ignored across all 22 root `hql2_*_tests.rs` targets | Offline, locked, no-default-features, jobs=1; the ignored parser child entrypoint is exercised by its parent. Protected probe excluded. Oracle targets are included, so this is regression evidence, not P8 acceptance. |
| Typed-IR root Match anchors | 12 passed, 0 failed in `hql2_expand_tests`; 6 passed, 0 failed in `hql2_value_tests` | Exact full RecordRef comparison, stale/foreign no-row behavior, edge-alias/type rejection, Entity parameter scope restriction, internal-node anchor before SHORTEST deduplication. |
| Post-anchor explicit all-HQL2 target sweep | 319 passed, 0 failed, 1 ignored across 22 root `hql2_*_tests.rs` targets | Locked/offline/no-default-features/jobs=1; ignored parser child entrypoint is exercised by its parent; protected probe excluded. Regression evidence, not P8 acceptance. |
| Structural unsigned HQL parameters | 50 passed across four targets: lower 14, execution 13, History/Change 10, vector operators 13 | Exact DecimalU64/no-cast binding for TAKE/SKIP/CHANGE/KNN/RERANK; missing, I64, nullable and u32 ranking overflow fail closed. |
| Final explicit all-HQL2 target sweep (after unsigned parameters) | 323 passed, 0 failed, 1 ignored across 22 root `hql2_*_tests.rs` targets | Locked/offline/no-default-features/jobs=1; ignored parser child entrypoint is exercised by its parent; protected probe excluded. Regression evidence, not P8 acceptance. |
| HQL2 implicit null-order default | 58 passed across three targets: lower 15, execution 14, scalar execution 29 | Omitted HQL null placement lowers to NULLS LAST for ASC/DESC; explicit FIRST is preserved; typed IR remains unchanged. |
| Final explicit all-HQL2 target sweep (after null-order default) | 325 passed, 0 failed, 1 ignored across 22 root `hql2_*_tests.rs` targets | Locked/offline/no-default-features/jobs=1; ignored parser child entrypoint is exercised by its parent; protected probe excluded. Regression evidence, not P8 acceptance. |
| Checked HQL remainder | 79 passed across four targets: lower 16, wire 18, scalar execution 30, storage-backed execution 15 | HQL `%` lowers to typed-IR `rem`; matching I64/F64 only, NULL propagation, signed float remainder, `+0.0`/`-0.0` and integer-zero errors, `MIN % -1` overflow, wrong-type rejection, independent test-only reference comparison, fixture-schema checksum synchronized. |
| Explicit all-HQL2 target sweep (before oracle differential) | 328 passed, 0 failed, 1 ignored across 22 root `hql2_*_tests.rs` targets | Locked/offline/no-default-features/jobs=1; ignored parser child entrypoint is exercised by its parent; protected probe excluded. Historical regression checkpoint, not P8 acceptance. |
| Exact-KNN and Original Rerank P7 oracle differential | 2 passed, 0 failed | HQL and typed IR both match the independent exact-L2 oracle for KNN and Original Rerank; coverage includes equal-distance identity ordering, missing-KNN-original exclusion, and top-k reranking; two test-only fixtures, not broad P8 oracle acceptance. |
| All-root-HQL2 sweep after KNN/Original Rerank oracle differential (historical) | 330 passed, 0 failed, 1 ignored across 23 root `hql2_*_tests.rs` targets | Locked/offline/no-default-features/jobs=1; ignored parser child entrypoint is exercised by its parent; protected probe excluded. Regression evidence, not P8 acceptance. |
| Root `MATCH` P7 oracle differential | 1 passed, 0 failed | HQL root `MATCH` matches the independent P7 bag for parallel `LINK` edges and excludes a non-matching relation; test-only fixture. |
| All-root-HQL2 target sweep after root MATCH differential (historical) | 331 passed, 0 failed, 1 ignored across 24 root targets | Locked/offline/no-default-features/jobs=1; ignored parser child entrypoint is exercised by its parent; superseded by the current 30-target sweep. Regression evidence, not P8 acceptance. |
| Approved P8 addendum D1-D6 focused target | 10 passed, 0 failed | Registered lexical BM25 profile and independent P7 rank parity; ContextPack scalar offsets, exact source hash and independent oracle parity; Sequence node/edge property exactness and no-partial budget behavior; contextual NULL/list/JSON boundaries; authorization-before-parse fixture. |
| D7 actor-scoped HQL1 adapter | 11 passed, 0 failed | Differential parity for unlabeled zero-hop ID projections with optional one exact same-alias string equality predicate, single-label zero-hop projections without predicates, and bounded unlabeled one-hop ID projections across edge directions, relation/wildcard forms, endpoint projections, one endpoint-ID exact string filter and parallel-edge multiplicity; Namespace(Read)/namespace checks before parse; malformed/unlisted behavior; broad legacy pattern fails resource preflight before AST construction. Existing v1 transports are unchanged. |
| HQL1 parser-resource RCA | Confirmed and corrected | Legacy Pest pairs were materialized before the D7 allowlist without parser-specific heap reservation. The adapter now runs allocation-free parser preflight and reserves the conservative estimate before legacy AST construction; see local RCA `.brain/rca/RCA--HQL1-ADAPTER-PARSER-RESERVATION.md`. No production crash or HQL1 allocator peak is claimed. |
| Storage-backed HQL/typed-IR scalar P7 differential | 2 passed, 0 failed; 81 four-value nullable bags plus 6 scalar-pipeline fixtures; 174 Storage executions | Existing Values/Distinct/Sort bags plus projection/arithmetic/sort/offset/take and filter/project/distinct/sort/take compositions; each frontend independently matches P7 and each other. Test-only evidence, not broad P8 acceptance. |
| Storage-backed HQL/typed-IR aggregate P7 differential | 1 passed, 0 failed across 121 nullable bags; 242 Storage executions | Covers empty through four-row bags over NULL/-1/2 and count-all/count/sum/avg/min/max/collect; each frontend matches P7 and each other. Test-only aggregate evidence, not broad P8 acceptance. |
| Storage-backed HQL/typed-IR `Values`/`UnionAll` P7 differential | 1 passed, 0 failed across 169 nullable bag pairs; 338 Storage executions | All pairs from 13 empty/length-one/two bags over NULL/-1/2 match the independent P7 plan; explicit null-last ordering preserves duplicate and NULL multiplicity. Test-only evidence, not broad P8 acceptance. |
| Storage-backed HQL/typed-IR Join P7 differential | 1 passed, 0 failed; four join kinds plus bare JOIN | HQL and typed IR match P7 result bags for Inner/Left/Semi/Anti (5/7/3/2), duplicate keys, missing-property NULL and JSON null; bare JOIN defaults to Inner, and Semi/Anti hide the right scope. A test sentinel preserves JSON-null distinction in P7. Test-only evidence, not broad P8 acceptance. |
| Storage-backed HQL/typed-IR AnnotationLookup P7 differential | 1 passed, 0 failed; focused target 6/6 | Frozen-target match, evidence-only nonmatch, optional NULL extension and duplicate-input multiplicity match independent P7 for HQL and typed IR. Test-only evidence; no runtime/contract/schema/P6/transport change, not broad P8 acceptance. |
| Storage-backed HQL/typed-IR Sequence-property P7 differential | 3 passed, 0 failed; focused pattern target 11/11 | Positive exact UTF-8 node-name and integer edge-weight constraints, optional edge-property miss, and nested JSON node/edge properties match independent P7 for HQL and typed IR; wrong-valued candidates are excluded, and all four optional input rows survive with endpoint/edge aliases NULL-extended. Nested object/array values, including an embedded JSON null, compare exactly. Test-only; no runtime/contract/schema/P6/transport change. |
| Storage-backed HQL/typed-IR Node/Edge/Row source-scan P7 differential | `hql2_storage_source_tests`: 10 passed, 0 failed | Node label and edge relation nonmatches are excluded, two relational rows retain their bag multiplicity, and HQL and typed IR each match the independent P7 scan bag. Row comparison uses the declared row-key property; durable `r.id` remains the UUIDv4 revision. Test-only; no runtime/contract/schema/P6/transport change. |
| Approved H2-D11 Vector HistoryScan extension | Implemented; local differential passed | HQL and typed IR read exact revisions using compact JSON `(owner_id, collection_id)` identity under the P6 lease and owner-node ACL; the vector source floor fails closed before access. Its result bags match independent P7 from WAL-derived revision facts and the captured frontier. The expanded five-kind HistoryScan differential passes; HistoryScan/ChangeScan passes 17/17, P7 graph target 39/39 and combined P7 130/130. |
| HQL/typed-IR Row HistoryScan and ChangeScan parity | 1 passed, 0 failed | Retained insert/update revisions hydrate exact row properties; HQL and typed IR agree on history values, change operations and durable Row identity. Test-only regression coverage; no schema migration. |
| HQL2/IR transaction-time snapshot (`tx_as_of`) | Implemented; local verification passed | One selected frontier S controls revision-backed scans, graph/vector/annotation operators, source-floor checks, property hydration and `Snapshot.tx`; the validated P6 generation/catalog/current policy remain pinned and no path falls back to current state. Five focused targets pass 56/56. |
| Latest explicit root-HQL2 target sweep | 394 passed, 0 failed, 1 ignored across 35 root targets | All 35 root `hql2_*.rs` integration targets explicitly selected with `--locked --offline --no-default-features --jobs 1`; includes the new labeled-filter differential, Node/Edge/Row source-scan and ChangeScan cursor-window P7 differentials, and the ignored parser child exercised by its parent. Regression evidence, not P8 acceptance. |
| Current selected P6 peer regression set | 45 passed, 0 failed, 0 ignored across seven named targets | `governance_tests`, `p6_generation_tests`, `p6_lease_tests`, `p6_visibility_tests`, `wave_a_commit_tests`, `temporal_queries_tests`, `tx_as_of_wp22_tests`; focused current rerun, not the broader 11-target group. |
| Separate P6/schema-v6/compatibility sweep | 194 passed, 0 failed, 0 ignored across 11 named targets | `p6_generation_tests`, `p6_lease_tests`, `p6_visibility_tests`, `p6_peer_authority_tests`, `schema6_migration_tests`, `query_ir_tests`, `hql_p0_tests`, `hql_filter_tests`, `hql_cypher_tests`, `napi_rest_parity_tests`, `rest_api_tests`; previously measured, not rerun in this continuation. |
| Prior explicit HQL2/P6/schema-v6/compatibility sweep | 528 passed, 0 failed, 1 ignored across 37 named targets | Pre-edge-regression checkpoint; protected probe excluded. Superseded for current HQL2 root-target count by 394/0/1 across 35 targets; not full P8/P13 acceptance. |
| Native library check | `cargo check --locked --offline --no-default-features --jobs 1 --target-dir target/hql2-execution` passed | Local core compile only; no NAPI addon runtime, release or cross-platform claim. |
| Documentation and source hygiene | docs validation: 0 violations/240 files; agent registry: 6 agents/12 routes; rustfmt check and `git diff --check` passed | Local structural checks only. |
| HQL/IR Node/Edge/Row source hydration differential | 1 passed, 0 failed | Node/Edge properties and Row `prop(r, "id")` match across HQL and IR; `r.id` remains the UUIDv4 durable revision. Row properties resolve from H2-D11 `after_image`; see local RCA `.brain/rca/RCA--HQL2-ROW-SOURCE-PROPERTY-HYDRATION.md`. |
| HQL2 annotation ACL and ChangeScan budget regression | 11 passed, 0 failed | Namespace-only policy denies annotation subjects; explicit Annotation(Read) permits them. Three hidden Annotation revisions are excluded before caller-budget charging while a readable Node event still returns under a one-node limit. Same-lease target/evidence checks remain covered. No timing noninterference claim. |
| Approved ChangeScan budget policy | Implemented; threshold regression passed | Preserve Namespace(Read)-authorized non-annotation subjects; exclude Annotation revisions without Annotation(Read) before caller-budget accounting/materialization. The ACL target passes 11/11; History/Change passes 17/17. Independent review and hosted CI of this source revision remain open. |
| HQL/IR annotation source and recursive ACL parity | 7 passed, 0 failed, 0 ignored | Separate `targets`/`evidence` properties, nested annotation references, and Edge endpoint access are covered; fixture evidence is independent of P8 operator acceptance. |
| Long-running soak | Three soak_tests cases remain ignored by the normal suite | No soak qualification inferred |
| Storage-backed ChangeScan P7 differentials | 3 passed, 0 failed | Node upsert/correct/retract plus Edge/Row/Vector/Annotation revisions match independent P7 by sequence, operation and exact subject identity; HQL and typed IR agree. A separate five-window test verifies exclusive `after`, inclusive `through`, equal empty bounds and selected `tx_as_of` frontiers. Recursive Edge endpoint, Vector owner and Annotation target/evidence dependencies are represented. Broader ChangeScan semantics and full acceptance remain open. |
| PR #194 final state at head `4f4e298` | [Run 37104811672](https://github.com/Freshair129/GenesisBlock/actions/runs/37104811672): 48 checks passed, five failed and five skipped; three worker OS tests and rebuilt Linux addon/worker fail with `RECOVERY_REQUIRED: markerless database identity is missing`. The Android GitHub Packages check failed during checkout after the PR ref disappeared. | PR #194 was merged at `4f02d6b` on 2026-10-03 despite the five reported failures; no admin bypass was used and reviewDecision is empty. No package publication is inferred from the failed checkout. Worker correction remains approval-gated; broad P8/P13 remains open. |
| Hosted PR #196 at head `8ac07f6` | [Tests run 37114718266](https://github.com/Freshair129/GenesisBlock/actions/runs/37114718266): 10 displayed checks pass, five fail, one skips. Worker npm tests fail on Linux/macOS/Windows with `RECOVERY_REQUIRED: markerless database identity is missing`; [rebuilt addon/worker run 37114718286](https://github.com/Freshair129/GenesisBlock/actions/runs/37114718286) has the same failure. Windows `cargo test` fails at 15m16; detailed log is unavailable, so its cause is unconfirmed. | PR #196 is merged at `fb7085a` with no review decision recorded. Worker startup and Windows job-time corrections remain open; broad P8/P13 and independent review remain open. |
| Hosted PR #194 CI at head 31168524 | [Tests run 37073380412](https://github.com/Freshair129/GenesisBlock/actions/runs/37073380412): Rust, standard Node, docs, fmt/clippy and non-worker distribution checks pass; worker npm tests fail on Linux/macOS/Windows with RECOVERY_REQUIRED: markerless database identity is missing. [Rebuilt addon/worker run 37073380336](https://github.com/Freshair129/GenesisBlock/actions/runs/37073380336) fails with the same error. | Four worker checks remain red; correction is approval-gated and no worker code changed. |
| Hosted PR #194 CI at docs-only head 43cc6e8 (engine sources match 31168524) | [Tests run 37083654705](https://github.com/Freshair129/GenesisBlock/actions/runs/37083654705): Linux/macOS Rust, standard Node on all three OSes, docs, fmt/clippy and version consistency pass. Windows Rust fails `storage_hql_join_kinds_match_typed_ir_and_independent_p7` at `tests/hql2_storage_hql_join_oracle_differential_tests.rs:281` with `QUERY_BUDGET_EXCEEDED` / `query_limit`. [Rebuilt-addon run 37083654689](https://github.com/Freshair129/GenesisBlock/actions/runs/37083654689) and worker npm jobs on Linux/macOS/Windows fail with `RECOVERY_REQUIRED: markerless database identity is missing`. | Five CI checks fail. The Join differential target passes locally 5/5 at 0.21–0.22s; the generic error does not identify the budget dimension, so the CI cause remains suspected. No implementation source changed at 43cc6e8. |
| PR #194 state after run 37083654705 | GitHub reports OPEN, UNSTABLE at head 43cc6e8; 48 checks pass, five fail, five skip; review submissions remain empty. | Not merged; no failed checks were bypassed. Broad P8/P13 acceptance remains open. |
| Hosted PR #194 CI at head c4f5729 (before this ACL patch) | [Tests run 37085618470](https://github.com/Freshair129/GenesisBlock/actions/runs/37085618470): 48 checks pass, four fail, five skip and one is cancelled. Worker npm tests fail on Ubuntu/Windows/macOS, and the Windows Rust job is cancelled; [rebuilt addon/worker run 37085618419](https://github.com/Freshair129/GenesisBlock/actions/runs/37085618419) also fails. | These runs do not include the ACL patch. Updated code is pending hosted CI and review; do not merge with current failures/cancellation. |
| PR #194 state at head c4f5729 | GitHub reports OPEN, UNSTABLE; 48 checks pass, four fail, five skip, one cancelled; review submissions remain empty. | Not merged; failed worker checks and cancelled Windows Rust job remain unresolved; updated ACL code has not run in hosted CI. |
| Windows Join differential local reproduction | `cargo test --locked --offline --no-default-features --test hql2_storage_hql_join_oracle_differential_tests --target-dir target/hql2-execution`, repeated five times: 5/5 pass, each 0.21–0.22s. | Does not prove which default budget dimension fired on GitHub Actions; do not infer a runtime fix from the local pass. |
| Hosted Rust/core CI, PR #194 head `9344b71` | [Run 37029503800](https://github.com/Freshair129/GenesisBlock/actions/runs/37029503800): Linux/macOS `cargo test`, all three standard `npm test` targets, `fmt + clippy`, docs validation and version consistency passed. Windows `cargo test` reached its 15-minute job limit while `tests/zz_probe_discriminates.rs::probe_vs_recall` was still running; runner canceled the job. | Windows full Rust suite remains incomplete; protected probe source was not changed. |
| Hosted Rust/core CI, PR #194 head `484916b` (parent of this local test/docs-only change) | [Run 37049245946](https://github.com/Freshair129/GenesisBlock/actions/runs/37049245946): Linux/macOS/Windows `cargo test`, standard `npm test` on all three OSes, `fmt + clippy`, docs validation and version consistency passed; worker test jobs failed on all three OSes. The Join differential in this local revision is not included. | Core checks passed; hosted worker checks remain red. |
| Hosted worker CI, PR #194 head `9344b71` | [Run 37029503800](https://github.com/Freshair129/GenesisBlock/actions/runs/37029503800) and [rebuilt-addon run 37029503808](https://github.com/Freshair129/GenesisBlock/actions/runs/37029503808): worker tests failed on Linux/macOS/Windows and rebuilt Linux addon with `RECOVERY_REQUIRED: markerless database identity is missing`. | Worker startup-order correction remains documentation/approval-gated; no worker code changed. |
| Hosted worker CI, PR #194 head `484916b` (parent of this local test/docs-only change) | Worker tests fail on Linux/macOS/Windows in [run 37049245946](https://github.com/Freshair129/GenesisBlock/actions/runs/37049245946); rebuilt Linux addon also fails in [run 37049246023](https://github.com/Freshair129/GenesisBlock/actions/runs/37049246023) with `RECOVERY_REQUIRED: markerless database identity is missing`. | Worker startup-order correction remains approval-gated; no worker code changed. |
| Hosted mobile/consumer CI, PR #194 head `9344b71` | [Mobile run 37029503804](https://github.com/Freshair129/GenesisBlock/actions/runs/37029503804): host mobile build/tests, iOS/Android builds and acceptance, C header and SDK checks passed. RustSec, Go/Python consumers and distribution checks passed in their respective runs. | Hosted green jobs do not close P8/P13, security-review, soak or release qualification. |
| PR #194 merge state at head `484916b` | PR is open and GitHub reports mergeable; checks fail in worker jobs and no review submissions exist. | Not merged; CI/review conditions remain unmet. |

Parser tests include all 35 positive and 6 negative vendored examples. The
parser harness reports one ignored child entrypoint, explicitly executed twice
by its passing parent (default/detailed diagnostic allocator probes). It is not
unrun coverage. The storage-backed scalar differential enumerates all 81
four-value bags over NULL, -1 and 2; HQL and typed IR each execute 81 requests
and match independent P7 results for distinct/order semantics. The new
storage-backed aggregate differential enumerates all 121 bags from lengths 0-4
and runs both frontends against P7 for seven aggregate functions (242 Storage
executions). Oracle-only graph/rank tests remain fixture evidence, not
storage-backed operator acceptance.

The existing REST integration tests run in this native suite. Their passing
results preserve the old surface only: no HQL2 transport is implemented or
qualified by those tests. No Node addon runtime test was run.

EXPLAIN storage tests compare signed journal, SQLite database/WAL and snapshot
bytes before/after fresh and changed stores, plus stable frontier and absent
actual counters. Windows `genesis.lock` and SQLite volatile `projection.sqlite-shm`
have presence/size checks instead of byte-content comparisons; neither is an
authoritative data/WAL artifact. Test mismatch diagnostics print names only.

Reproducible focused command from the isolated checkout:

```powershell
cargo test --locked --offline --no-default-features --jobs 1 --test hql2_execution_tests --test hql2_parser_tests --test hql2_lower_tests --test hql2_request_tests --test hql2_scalar_execution_tests --test hql2_value_tests --test hql2_wire_tests --target-dir G:/GenesisBlock_Dev/GenesisBlock/target/hql2-execution
```

Post-anchor explicit 22-target regression command (protected probe excluded):

```powershell
cargo test --locked --offline --no-default-features --jobs 1 --test hql2_annotation_acl_tests --test hql2_annotation_lookup_tests --test hql2_annotation_source_tests --test hql2_durable_revision_tests --test hql2_execution_tests --test hql2_expand_sequence_tests --test hql2_expand_tests --test hql2_graph_oracle_tests --test hql2_history_change_tests --test hql2_key_codec_tests --test hql2_lower_tests --test hql2_oracle_tests --test hql2_parser_tests --test hql2_pipeline_oracle_tests --test hql2_rank_oracle_tests --test hql2_request_tests --test hql2_scalar_execution_tests --test hql2_storage_source_tests --test hql2_value_tests --test hql2_vector_operator_tests --test hql2_vector_revision_tests --test hql2_wire_tests
```

Latest guarded 33-target regression command (only the named HQL2 targets are selected):

```powershell
cargo test --locked --offline --no-default-features --jobs 1 --quiet --test hql2_annotation_acl_tests --test hql2_annotation_lookup_tests --test hql2_annotation_source_tests --test hql2_durable_revision_tests --test hql2_execution_tests --test hql2_expand_sequence_tests --test hql2_expand_tests --test hql2_graph_oracle_tests --test hql2_history_change_tests --test hql2_hql_reference_differential_tests --test hql2_hql1_adapter_tests --test hql2_key_codec_tests --test hql2_lower_tests --test hql2_oracle_tests --test hql2_p8_completion_tests --test hql2_parser_tests --test hql2_pattern_constraint_oracle_tests --test hql2_pipeline_oracle_tests --test hql2_rank_oracle_tests --test hql2_request_tests --test hql2_root_match_oracle_differential_tests --test hql2_scalar_execution_tests --test hql2_storage_aggregate_oracle_differential_tests --test hql2_storage_set_oracle_differential_tests --test hql2_storage_hql_join_oracle_differential_tests --test hql2_storage_join_oracle_differential_tests --test hql2_storage_source_tests --test hql2_value_tests --test hql2_vector_operator_tests --test hql2_vector_oracle_differential_tests --test hql2_vector_revision_tests --test hql2_wire_tests --test hql2_row_history_tests --target-dir target/hql2-execution
```

First full-suite command (failed at the legacy fixture described above):

```powershell
cargo test --locked --offline --no-default-features --jobs 1 --target-dir G:/GenesisBlock_Dev/GenesisBlock/target/hql2-execution
```

No-fail-fast full-suite command (only this checkout, protected probe absent):

```powershell
cargo test --locked --offline --no-default-features --no-fail-fast --jobs 2 --config profile.test.debug=0 --config profile.test.debug-assertions=true --config profile.test.overflow-checks=true --target-dir G:/GenesisBlock_Dev/GenesisBlock/target/hql2-qualification
```

This separate qualification target disables debug symbols only for this test
invocation, retains assertions and overflow checks, and does not edit Cargo
profiles or global configuration. It is not release-mode or performance evidence.

Final correction verification, with the same profile and target directory:

```powershell
cargo test --locked --offline --no-default-features --jobs 2 --config profile.test.debug=0 --config profile.test.debug-assertions=true --config profile.test.overflow-checks=true --test wal_tail_replay_tests --test epoch_e2_tests --test meta_format_migration_tests --test p6_generation_tests --test p6_lease_tests --test p6_visibility_tests --test p6_peer_authority_tests --target-dir G:/GenesisBlock_Dev/GenesisBlock/target/hql2-qualification
```

Do not rewrite the preceding full run as a green full-suite invocation: its
failure was recorded before the final test-only correction. The lib-unit and
doc-test harnesses have zero tests; they are not extra acceptance evidence.

Documentation: Python 3.13 `-B scripts/validate_doc_status.py docs` reports zero
violations in 237 files. Scoped rustfmt and `git diff --check` pass. No transport
file changes and no HQL2/query_v2 route/binding in router/main/FFI/index.d.ts/MCP;
Rust internal-only surface selection is deliberate under the P8 contract.

## Review corrections and provenance

RCA files are local under `.brain/rca/` (ignored by default); this committed
report retains the causes and distinguishing evidence:

1. Plain HQL was mistaken for explicit explain=None; envelope Plan/Analyze
   incorrectly conflicted. Preserve omission and compare explicit modifiers.
2. Initial disk evidence collection tried reading the Windows ownership lock;
   OS error 33 preceded query execution. Volatile SQLite read marks also changed
   legitimately. Narrow only these two synchronization artifacts as above.
3. Early refusal of deeply constructed Rust JSON could recursively drop input.
   Retain ownership and dispose it iteratively; test 10000 nested arrays.
4. Collect evaded its element row bound; List equality borrowed an unregistered
   Rust comparator. Check cardinality and reject unsupported comparison at bind.
5. AVG initially bypassed checked same-type accumulation. The P7 discriminator
   AVG([i64::MAX,i64::MAX]) requires overflow, not a finite running-mean answer.
6. Recursive bind/eval/lower frames could exhaust a Windows-sized stack despite
   a logical depth cap. Use iterative traversals, with small-stack regressions.
7. Propagated aliases and per-expression type clones were uncharged. A 16KiB
   alias over 100 nodes exceeded a 1MiB reservation; deep types referenced 4000
   times exceeded a 3MiB reservation. Reserve these copies before binding.
8. G0 cloned long IDs before planning charges. Use borrow-only envelope checks
   and the mandatory closed IR decoder; reserve ID copies before validation.
9. Parser work and nested BETWEEN expansion require pre-allocation bounds, not
   a post-allocation node count. An iterative lexical preflight shares public
   parser/query-boundary limits: 262144 Unicode scalars, at most 2048 lexical
   units, and heap plus worker-stack reservation within 64 MiB. Some valid
   wide queries deliberately fail QUERY_BUDGET_EXCEEDED. The grammar/Pest-specific
   conservative bound is not an RSS measurement or a global allocator limit.
   Independent parser verification passed 18 tests, including two explicit
   child allocator probes. A 200033-byte wide rejection fell from 97518360
   to 709 incremental live allocation bytes. Lowering checks projected BETWEEN
   node count/depth before construction: depth25 refuses; small depth10 retains
   6139 wire expressions. Lowering passed 12 tests on the reviewed source.
10. A healthy read-only handle without a reusable current generation returned
    DATA_CORRUPTION because the P6 publisher's exact `read-only` reason hit the
    generic mapper. The runtime RED reproduced this classification after
    successful EXPLAIN. Map only that exact reason to CAPABILITY_UNSUPPORTED;
    do not prohibit existing-generation reuse or authorize publication writes.
    Use the shared Error.reason field: napi's Display adds a status prefix,
    unlike the native Error shim. Default-feature compilation is checked
    separately; no addon runtime or transport test is inferred from it.
11. Full-suite integration exposed old migration fixtures overwriting metadata
    while retaining the new P6 digest manifest. Epoch GBP1 migration failed;
    malformed metadata unexpectedly recovered through WAL, and the bincode
    test's search-only success was a false positive for decoder coverage.
    Correct only the test fixtures to schema4/no-P6 after asserting fresh
    Disabled revision0/no generation. Retain zero-epoch decoder evidence and
    add a modern manifest-corruption case requiring WAL recovery. Production
    snapshot validation is unchanged. Local RCA:
    `.brain/rca/RCA--HQL2-UPSTREAM-LEGACY-SNAPSHOT-FIXTURES.md`.
12. The no-fail-fast full run found the same fixture mismatch in WAL tail replay:
    `snapshot_without_frontier_still_loads` removed journal/wal_frontier but
    retained schema5/P6. Required frontier validation correctly rejected the
    snapshot; WAL replay produced 10 arena rows, not the expected legacy 20.
    Construct schema4/no-P6 only for the guarded legacy fixture, preserving its
    20-row assertion. Add a separate modern missing-frontier case requiring
    exactly 10 arena rows, all nodes and 10 search results after WAL recovery.
    Independent source review passed; production validation/recovery unchanged.
    Final targeted rerun passed all 48 tests, including both legacy and modern
    missing-frontier discriminators. No known failing tested case remains;
    the full suite was not rerun after this final test-only fixture change.

## Current source checkpoint and open work — do not promote P8/R1

The local P8 kernel now executes Values, Filter, Project, Distinct, Sort, Take,
Offset, UnionAll, Aggregate, Join, NodeScan, EdgeScan, RowScan, AnnotationScan,
AnnotationLookup and structural root Match. The four source scans return
database-bound revision refs under one P6 lease, use bounded keyset pagination
and validate their source filters;
RowScan binds the authorized table catalog. Annotation scans check normalized
target/evidence references. Identity fields and `prop`/`has_prop` work on these
sources, including typed nulls after a LEFT JOIN, with query-budget reservation
before source payload reads. AnnotationLookup now matches normalized targets
but not evidence, preserves frozen/live revision semantics at the leased S,V,
applies reference ACL checks, and supports optional (typed-null preservation)
and required (inner) lookup. HQL syntax and query-ir.v2 execute through the
same binder/kernel; a focused test compares their results and duplicate input
paths retain multiplicity. HistoryScan and ChangeScan read exact retained
revisions; Vector HistoryScan uses the compact H2-D11 `(owner_id,
collection_id)` tuple under the P6 lease and vector floor. Compact and ordered multi-segment Sequence Expand
execute through both frontends against exact node/edge revisions in the same
P6 lease. Per-step direction/relation filters and bounded hops compose with
global Walk/Trail/Simple uniqueness. Each step can bind node and edge aliases;
optional null extension, one typed path, deterministic ordering and measured
EXPLAIN ANALYZE counters are covered. Structural root Match executes compact
and Sequence patterns through HQL and typed IR; SHORTEST returns one stable
minimum-hop path per endpoint pair. HQL/IR parity, parallel-edge tie ordering,
sequence results and fail-closed graph-budget exhaustion pass. Typed-IR Match
anchors compare validated non-null RecordRefs; supported node ID/label/property
constraints execute for HQL and typed IR, with unmatched or unsupported shapes
failing closed. At the earlier pre-vector
checkpoint, ten focused and regression targets passed 154 tests: Expand 11/11, Sequence 8/8, scalar 28/28,
lowering 13/13, parser 18 passed (one child entrypoint is ignored directly and
exercised by its parent), query boundary 12/12, wire 18/18, P7 graph oracle
38/38, values 6/6, and vector 2/5. This historical count is superseded by the
historical 21-target sweep below; these local fixture results do not
qualify P8.

A separate H2-D11 regression sweep passed 62/62 over eight explicit offline
targets: durable revisions, annotation ACL/lookup/source, row key codec,
storage-backed scans, vector revision identity, and schema-v6 fixture migration.
These passes were recorded before exact vector execution and do not qualify
history reads, the remaining operators, or P8.
The Blueprint semantic-oracle suite also passed 26/26 Python tests after the
AnnotationPut example/oracle switched to a separate `evidence` field; this is
fixture-level evidence, not Rust runtime verification.

On 2026-09-29, every named `hql2_*` integration target was re-run explicitly:
294 passed, 3 failed and 1 ignored across 21 targets. The parser's ignored
child entrypoint is exercised by its parent. All three failures are the two
exact-KNN cases and one original-Rerank case, which at that checkpoint received
`CAPABILITY_UNSUPPORTED/operator_unavailable` at bind; the two vector
parameter mismatch tests pass. Cargo stopped at the failing vector target, so
the vector-revision (2) and wire (18) targets were then run separately and
passed. This sweep includes independent oracle targets and is not a P8
acceptance count; all targets were named explicitly and the protected probe
was not compiled. This is a historical checkpoint, superseded by the latest
307/0/1 all-target result below, later superseded by the final 318/0/1 sweep.

`FieldIdV2`/`ExecBatchV2` uses typed columns, stable row keys, dynamic
row-dependent names, and no payload-backed entity rows. The source-level
differential confirms Node/Edge and Row property behavior; RowScan reads
`after_image` while retaining the UUIDv4 entity identity. Annotation
target/evidence checks remain before body hydration.

Exact KNN and Original Rerank now execute through the same HQL/IR kernel and
P6 ReadView. The HQL `space_id` resolves to the H2-D11 collection fingerprint
(SHA-256 with domain `genesis.hql2.vector-space.v1:` over the compact ordered
collection-name/model/dimension/metric/quantization tuple); typed IR vector
parameters must declare the same space and dimension. The ReadView validates
namespace/node access and owner revision at the pinned S,V, then reads only the
matching original schema-v6 vector revision. It does not consult HNSW,
quantized vectors or sidecars. Exact L2 is squared Euclidean distance; Cosine
uses the collection metric. Lower distance wins, with deterministic owner-key
then input-order ties. KNN skips candidates without an original; Original
Rerank fails closed if any candidate lacks one. Candidate/distance budgets do
not return partial rows. Scores are typed; `.value` is canonical and `.distance`
is an HQL alias. Approximate KNN remains fail-closed.

HistoryScan and ChangeScan now execute over exact retained schema-v6 revisions
under one P6 ReadView. The focused History/Change target passes 17/17, covering
HQL/IR parity, closed-revision hydration, Vector tuple identity and floor
rejection, retract tombstones, exclusive source floors, vector-owner events,
recursive Annotation and Edge-reference ACL, future/unknown-family rejection,
no-partial budget exhaustion, and five P7 cursor windows with selected
transaction frontiers. The pre-anchor explicit sweep of all 22 root
`hql2_*_tests.rs` targets passed
318/0/1. After root Match anchors, the explicit sweep passed 319/0/1; after
structural unsigned parameters, it passed 323/0/1; after implicit NULLS LAST,
it passed 325/0/1; after checked remainder, it passes 328/0/1. The ignored parser child entrypoint is exercised by its
parent. The focused anchor target passes 12/12 and the Entity-value target
passes 6/6; four structural-parameter targets pass 50/50, three null-order
targets pass 58/58, and four checked-remainder targets pass 79/79. The latest
all-root-HQL2 sweep includes the KNN, Original Rerank and root `MATCH` oracle
differentials plus the Sequence pattern oracle, D7 adapter, row-history and
AnnotationLookup and positive Sequence-property differential targets, and now
passes 394/0/1 across all 35 root targets on a fresh explicit rerun. The D7 zero-hop exact-ID adapter differential
also passes within the adapter target. A test-only scalar
pipeline differential adds six deterministic P7 cases (12 Storage executions)
for both HQL and typed IR across projection/arithmetic/order/offset/take and
filter/project/distinct/order/take compositions. Both frontends match P7 and
each other; broad exact-oracle and P8 acceptance remain open. The storage-backed
HQL/typed-IR `Values`/`UnionAll` differential matches P7 for 169 nullable bag
pairs (338 Storage executions), preserving NULL and duplicate multiplicity.
The AnnotationLookup differential target passes 6/6, comparing HQL and typed
IR bags against P7 for a frozen target, evidence-only nonmatch, optional NULL
extension and duplicate-input multiplicity. A Sequence-property differential
also matches P7 for UTF-8 node names and integer edge weights, excluding
wrong-value candidates. An optional edge-property miss also matches P7,
preserving all four input rows and NULL-extending endpoint/edge aliases. The
nested-JSON Sequence differential compares exact object/array properties on
both nodes and the edge through HQL and typed IR against P7; mismatching nested
array candidates are excluded, including when another endpoint matches. The
focused pattern target passes 11/11. No runtime, contract, schema, P6 or
transport behavior changed.
Storage-backed HQL and typed-IR HistoryScan and ChangeScan bags for Node, Edge, Row, Vector and Annotation match P7
catalogs assembled from signed-WAL revisions and captured frontiers/valid-time
windows; checks include Edge endpoint ACL, Vector tuple/owner ACL and Annotation
target/evidence references. Artifact HistoryScan remains unsupported. The
10/10 addendum target covers registered lexical/context profiles,
P7 parity, Sequence node/edge properties, contextual literals and fail-closed
budget behavior. A separate explicit 11-target P6/schema-v6/compatibility
group passes 194/0/0. HQL2/IR `tx_as_of` now selects one historical frontier S
across source scans, graph/vector/annotation operators, hydration and
`Snapshot.tx`. Five focused targets pass 56/56; HistoryScan/ChangeScan passes
17/17 and Annotation source passes 7/7. The new ChangeScan P7 differential
checks five exclusive-after/inclusive-through windows, including empty bounds
and selected transaction frontiers. Per-source history floors fail closed before source access, while the
validated P6 generation/catalog/current policy remain pinned. These remain
local regression results; the prior 37-target combined sweep passed 528/0/1 before
the edge-property regression was added. Under current P6 policy an ACL-hidden-ID query-result
fixture is not meaningful; the exact-record-only actor is instead verified to
receive `FORBIDDEN/authorize` before HQL/IR parsing. Compact-pattern constraints
remain unavailable. The interrupted unfiltered command remains NOT_RUN; the
guarded explicit sweeps are the only current results.

HQL2 now has contextual NULL/list/JSON lowering. The differential-tested D7
zero-hop actor-scoped HQL1 form (unlabeled or single-label) and bounded
unlabeled one-hop form are lowered through
`query_v2`; other legacy HQL forms remain on their old runtime and shared
HQL1/v2/IR binding is not claimed. Checked HQL remainder,
NULLS LAST defaults and exact DecimalU64 structural parameters remain as recorded
above. The Blueprint AnnotationPut example keeps separate `evidence` references.
The ChangeScan P7 differential now covers all five supported subject kinds,
checking sequence, operation and exact subject identity for HQL and typed IR.
Broader ChangeScan error/semantic cases, cancellation, exact-oracle coverage,
spill/scheduling/index lifecycle, transport parity,
independent review, security qualification, soak/crash/platform acceptance and
the full 190-obligation ledger remain open.
No production release, user-database migration or external readiness is inferred
from these local tests.

## Version diff

| Artifact | Before | After |
|---|---|---|
| Registry | 0.5.75+draft | 0.5.76+draft |
| P8 completion addendum | 0.1.12b | 0.1.13b (accepted D7 evidence) |
| P6 generations/leases/ACL | 0.5.32b | 0.5.33b (evidence only) |
| P8 typed boundary | 0.2.59b | 0.2.60b (evidence only) |
| P7 oracle report | 0.1.11b | unchanged |
| Orchestration plan | 0.8.69b | 0.8.70b |
| C4 architecture index | 0.1.68b | 0.1.69b |
| Master specification | 2.3.40b | 2.3.41b |
| This report | 0.1.62b | 0.1.63b |

Version diff 0.1.62b -> 0.1.63b: extend the approved D7 allowlist after
legacy/HQL2 differential evidence to one exact same-alias string ID predicate
on labeled zero-hop scans. Match, miss, wrong-label exclusion and a
backslash/Unicode ID are covered; adapter/order/labeled-filter targets pass
11/11, 2/2 and 2/2. The full 35-target HQL2 sweep passes 394/0/1 and selected
P6 peer tests pass 45/0/0 across seven targets. The independent reviewer found
no runtime parity or authorization defect and identified the stale D7 text,
which this version synchronizes. Record PR #212 baseline checks (10 pass, five
fail, one skipped) and PR #213 code commit results (40 pass, four worker
failures with cause unconfirmed, five skipped, one Windows Cargo cancellation
at the 15-minute job limit). Broad P8/P13 and shared-runtime gates remain open.

Version diff 0.1.61b -> 0.1.62b: add a test-only HQL/typed-IR ChangeScan
P7 differential over five `(after, through]` windows, including equal empty
bounds and explicit `tx_as_of` frontiers; the focused History/Change target
passes 17/17. The fresh 34-target HQL2 sweep passes 392/0/1. Record merged
PR #210 hosted status (11 pass, four worker markerless-identity failures, one
scheduled check skipped). No runtime, contract, schema, P6 or transport change;
broad P8/P13 and independent-review gates remain open.

Version diff 0.1.60b -> 0.1.61b: add a storage-backed P7 bag differential
for HQL and typed-IR NodeScan, EdgeScan and RowScan; exclude wrong label and
relation candidates and preserve two row keys. The focused source target passes
10/10. A fresh locked/offline explicit sweep over all 34 root HQL2 targets
passes 391/0/1; this measured total supersedes the prior 393/0/1 report, which
was not reproduced by the complete command. No production runtime, contract,
schema, P6 or transport change; broad P8/P13 and independent-review gates remain
open.

Version diff 0.1.58b -> 0.1.59b: implement the conditionally approved D7
zero-hop exact string-ID equality extension after a legacy/HQL2 match/no-match
differential. Adapter tests pass 11/11, ordering tests 2/2, and the explicit
HQL2 sweep passes 393/0/1 across 34 targets. Labeled predicates, zero-hop
ordering and other unproven forms remain fail-closed. Synchronize the accepted
addendum, typed boundary, plan, C4, Master and registry. No P6 grant/lease,
schema/migration or transport behavior changed; independent D7 review and broad
P8/P13 acceptance remain open.

Version diff 0.1.57b -> 0.1.58b: add the test-only storage-backed HQL/typed-IR
Sequence nested-JSON property differential against independent P7, covering
exact node and edge objects/arrays (including nested JSON null) and excluding
near-match candidates. The focused pattern target passes 11/11 and HQL2
392/0/1 across 34 targets. Synchronize the P7 report, accepted addendum, plan,
C4, Master and registry. No runtime, contract, schema, P6 or transport change;
broad P8/P13 acceptance remains open.

Version diff 0.1.54b -> 0.1.55b: add the storage-backed HQL/typed-IR
AnnotationLookup differential against independent P7 for frozen-target match,
evidence-only nonmatch, optional NULL extension and duplicate-input
multiplicity; focused target 6/6 and HQL2 386/0/1 across 34 targets. Refresh
the D7 summary to distinguish the closed source-alias test gap from broader
review still open. No runtime/contract/schema/P6/transport change; broad
oracle and P8/P13 acceptance remain open.

Version diff 0.1.53b -> 0.1.54b: add storage-backed HQL/typed-IR scalar
pipeline differential evidence against independent P7 for six deterministic
fixtures and 12 Storage executions; retain the existing 81-bag scalar test.
Record 2/2 tests in the focused target and 383/0/1 across 33 HQL2 targets.
No runtime, contract, schema or transport behavior changed; broad oracle,
shared-runtime, P8/P13, independent review and qualification gates remain open.

Version diff 0.1.52b -> 0.1.53b: synchronize D7's differential-proven
single plain-ASCII label on zero-hop HQL1 node-ID scans and record the 10/10
focused adapter result. HQL2 remains 382/0/1 across 33 targets; the separate
P6/schema-v6/compatibility group remains 194/0/0. This is a narrow adapter
extension; other legacy forms, shared-runtime, independent review, broad P8/P13
and qualification gates remain open.

Version diff 0.1.51b -> 0.1.52b: add the storage-backed HQL/typed-IR
`Values`/`UnionAll` differential against P7 for 169 nullable bag pairs and 338
Storage executions; explicit null-last ordering preserves NULL and duplicate
multiplicity. The latest HQL2 sweep is 382/0/1 across 33 targets, with the
separate P6/schema-v6/compatibility sweep at 194/0/0. No runtime, contract,
schema or transport behavior changed. Broad exact-oracle, ChangeScan semantics,
P8/P13, independent review and qualification gates remain open.

Version diff 0.1.50b -> 0.1.51b: synchronize PR #196 hosted evidence at
head `8ac07f6`, merged as `fb7085a`. Ten displayed checks pass, five fail and
one is skipped; worker failures reproduce across three OSes and rebuilt
Linux, while the Windows Rust job failed at 15m16 with its cause unverified.
Local HQL2 380/0/1 and P6/schema-v6/compatibility 194/0/0 are unchanged.
Worker startup correction, Windows job-time disposition, broad P8/P13 and
independent review remain open.

Version diff 0.1.49b -> 0.1.50b: extend the P7 ChangeScan differential to
Edge, Row, Vector and Annotation alongside Node; seven events match on sequence,
operation and exact subject identity, with HQL/typed-IR parity. Record
History/Change 16/16 and HQL2 380/0/1 across 32 targets. Keep broader
ChangeScan semantics, worker correction, independent review and P8/P13 gates
open.

Version diff 0.1.47b -> 0.1.48b: add WAL-derived P7 HistoryScan differentials
for Node, Edge, Row and Annotation alongside the existing Vector profile;
record History/Change 15/15, Annotation source 7/7, HQL2 379/0/1, P7 130/130
and P6/schema-v6/compatibility 194/0/0. Artifact HistoryScan, broad ChangeScan
oracle coverage, hosted checks/review and P8/P13 acceptance remain open.

Version diff 0.1.45b -> 0.1.46b: implement and verify the owner-approved narrow
ChangeScan budget decision across H2-D11/P6/P8/plan: Namespace(Read) continues
to expose other readable kinds, while Annotation subjects without explicit
Annotation(Read) are excluded before caller-budget accounting/materialization.
The threshold regression passes; ACL 11/11, History/Change 14/14, HQL2 376/0/1
and selected P6/schema-v6 43/0/0. This source revision still needs hosted CI
and independent review; broad P8/P13 gates remain open. No schema/migration
change.

Version diff 0.1.43b -> 0.1.44b: synchronize P6/C4/master/H2-D11/plan/
registry versions and record completed PR #194 run 37083654705 at docs-only
head 43cc6e8. Four worker checks fail at markerless schema-v6 bootstrap and
Windows Rust fails the Join differential with `query_limit`; the exact budget
dimension is unconfirmed and local reproduction passes 5/5. No H2-D11 or P6
contract/schema change. PR remains OPEN/UNSTABLE and unmerged; broad P8/P13,
independent review and worker-correction approval gates remain open.

Version diff 0.1.42b -> 0.1.43b: synchronize current P6/C4/master/H2-D11/
plan/registry versions with the 374/0/1 HQL2 regression sweep across 32
targets. Refresh hosted PR #194 evidence at head 31168524: core CI passes,
three worker OS jobs and rebuilt Linux addon/worker fail at fresh schema-v6
bootstrap. The worker correction remains approval-gated; PR is OPEN/UNSTABLE
and unmerged. No H2-D11 contract or schema change; broad P8/P13/review gates
remain open.
Version diff `0.1.41b -> 0.1.42b`: record storage-backed HQL/typed-IR Row
HistoryScan/ChangeScan parity and exact retained row-property hydration (1/1);
record the explicit HQL2 regression sweep at 374/0/1 across 32 targets. No
schema or user-database migration is involved; hosted CI, independent review
and broad P8/P13 qualification remain open.

Version diff `0.1.40b -> 0.1.41b`: record implementation and local
verification of HQL2/IR `tx_as_of` as one no-fallback frontier across sources,
operators, source floors, hydration and `Snapshot.tx`; record focused 56/56,
History/Change 14/14, HQL2 373/0/1 and P6/compatibility 194/0/0. Hosted CI,
independent review and broad P8/P13 qualification remain open.

Version diff `0.1.39b -> 0.1.40b`: synchronize the accepted P8/P6
transaction-time snapshot contract across the H2-D11 ADR, plan and registry.
The evidence-backed runtime gap is recorded: normalization rejects explicit
`tx_as_of`, and revision-backed reads/results otherwise use the current
frontier. Implementation and verification remain pending; hosted CI, review
and broader P8/P13 gates remain open.

Version diff `0.1.38b -> 0.1.39b`: implement approved Vector HistoryScan in
HQL/typed IR with P6 vector-floor failure and the existing same-lease owner-node
ACL; record 12/12 focused tests, 367/0/1 across 31 HQL2 targets and 194/0/0
across 11 P6/compatibility targets. `tx_as_of`, hosted CI and broader P8/P13
acceptance remain open.

Version diff `0.1.36b -> 0.1.37b`: implement HQL `JOIN TABLE` lowering through
the approved two-input Join contract for Inner/Left/Semi/Anti, default bare
JOIN to Inner, and add a storage-backed HQL/typed-IR differential against
independent P7; record 365/0/1 across 31 HQL2 targets. Hosted CI for this local
revision has not run yet. At parent head `484916b`, Rust/core, standard
Node, fmt/clippy, docs and version checks pass, but worker tests fail across
OSes with `RECOVERY_REQUIRED: markerless database identity is missing`; no PR
reviews exist, so #194 remains open and unmerged. The new test is not in hosted
CI yet. Broad exact-oracle, shared-runtime, resource/cancellation and P8/P13
gates remain open.

Version diff `0.1.34b -> 0.1.35b`: add and pass a storage-backed HQL/typed-IR
aggregate differential against independent P7 for all 121 nullable bags of
length 0-4 (242 executions); record 363/0/1 across 29 HQL2 targets. Hosted
Rust/core checks pass on all three OSes at prior head `d4bc870`; worker checks
still fail across OSes and review remains pending, so PR #194 stays unmerged.
Broad exact-oracle, shared-runtime, resource/cancellation and P8/P13 gates
remain open.

Version diff `0.1.33b -> 0.1.34b`: add and pass a storage-backed HQL/typed-IR
scalar differential against independent P7 over 81 nullable bags (162 query
executions); record 362/0/1 across 28 HQL2 targets and 190/0/0 across 11
P6/schema-v6/compatibility targets. Update hosted evidence at PR head `cef747a`:
worker checks fail across platforms and Windows Rust times out at 15m15s; the
current differential is not included in that CI run. Keep merge, worker-fix,
Windows-timeout, broad P8/P13, shared-runtime and independent-review gates open.

Version diff `0.1.32b -> 0.1.33b`: synchronize hosted PR #194 results at
`cdfb90a`; record that the Windows Rust test step passed, including the
protected probe, but the 15-minute job timeout canceled cache finalization.
Record repeated cross-platform worker bootstrap failures and retain merge,
shared-runtime, broad P8/P13, independent-review, security, soak/crash/platform
and release gates.

Version diff `0.1.30b -> 0.1.31b`: record final full locked/offline Rust-suite
and default/no-default strict Clippy results, schema-v6 consensus revision
envelope signing order, and explicit `probe_vs_recall` NOT_RUN boundary;
synchronize H2-D11, P6, P8, plan, master and registry versions. Broader P8/P13,
independent-review, security, soak/crash/platform and release gates remain open.

Version diff `0.1.29b -> 0.1.30b`: record implementation and fixture
verification of H2-D11 R6b markerless schema-v6 WAL recovery; crash 17/17,
migration 19/19 and the selected 40-target HQL2/durability/authority aggregate
pass. Integrate Query IR V1 1.0.3 linear `match_path` from upstream; full
P8/P13, shared-runtime, independent-review and release gates remain open.

Version diff `0.1.28b -> 0.1.29b`: extend D7 only with the differential-proven
one-hop endpoint-ID exact string filter, reserve parser resources before legacy
AST construction, and record 9/9 focused adapter tests, 361/0/1 across 27 root
HQL2 targets and 190/0/0 across 11 separate P6/schema-v6/compatibility targets;
retain all shared-runtime/P8/P13 and independent-review gates.

Version diff `0.1.22b -> 0.1.23b`: implement Sequence node ID/labels for HQL
and typed IR under the P6 snapshot; record 7 focused passes and 338/0/1 across
25 root HQL2 targets, including the independent P7 pattern oracle; document
remaining ACL-hidden fixture, property/Compact, operator, transport, review,
P8 and release gates.

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.63b | 2026-10-04 | beta | Extend D7 after labeled-zero-hop match/miss/wrong-label and backslash/Unicode differential; adapter 11/11, ordering 2/2, labeled-filter 2/2; HQL2 394/0/1 across 35 targets and selected P6 peers 45/0/0 across 7; PR #213 code commit: 40 pass, 4 worker failures with unconfirmed cause, 5 skipped, 1 Windows Cargo cancellation at 15-minute job limit; no P6 contract/schema/transport change; broad P8/P13/shared-runtime gates remain open | working-tree | ATHER |
| 0.1.62b | 2026-10-04 | beta | Add test-only HQL/typed-IR ChangeScan P7 differential for five exclusive-after/inclusive-through windows and tx_as_of frontiers; History/Change 17/17 and HQL2 392/0/1 across 34 targets; PR #210 hosted checks 11 pass, four worker markerless-identity failures, one skipped; no runtime/contract/schema/P6/transport change; broad P8/P13 and independent review remain open | working-tree | ATHER |
| 0.1.61b | 2026-10-04 | beta | Add HQL/typed-IR Node/Edge/Row source-scan P7 bag differential; focused target 10/10; fresh explicit 34-target HQL2 sweep 391/0/1 (prior 393 count not reproduced); no runtime/contract/schema/P6/transport change; broad P8/P13 and independent review remain open | working-tree | ATHER |
| 0.1.60b | 2026-10-04 | beta | Synchronize merged PR #208 hosted status: 40 success, 5 failures and 5 skipped; four worker markerless-identity bootstrap failures and one Windows annotation `QUERY_BUDGET_EXCEEDED` failure; isolated Windows rerun passes 1/1 with cause unconfirmed; local HQL2 was reported 393/0/1 at that checkpoint and broad P8/P13/review gates stay open | working-tree | ATHER |
| 0.1.59b | 2026-10-04 | beta | Extend D7 with differential-proven same-alias exact string ID equality on unlabeled zero-hop HQL1 scans; adapter 11/11, ordering 2/2, HQL2 393/0/1 across 34 targets; other forms remain fail-closed; no P6 grant/lease/schema/migration or transport change; independent review and broad P8/P13 gates remain open | working-tree | ATHER |
| 0.1.58b | 2026-10-04 | beta | Add test-only HQL/typed-IR Sequence nested-JSON property differential against P7 with exact node/edge object/array equality and near-match exclusion; focused pattern 11/11, HQL2 392/0/1 across 34 targets; no runtime/contract/schema/P6/transport change; broad P8/P13 gates remain open | working-tree | ATHER |
| 0.1.57b | 2026-10-04 | beta | Add test-only HQL/typed-IR optional Sequence edge-property-miss P7 differential; preserve four input rows and NULL-extend endpoint/edge aliases; focused pattern target 10/10, HQL2 389/0/1 across 34 targets; synchronize accepted addendum, P7 report, plan, C4, Master and registry; no runtime/contract/schema/P6/transport change; broad P8/P13 gates remain open | working-tree | ATHER |
| 0.1.56b | 2026-10-04 | beta | Add test-only HQL/typed-IR Sequence node/edge exact-property P7 differential for positive UTF-8/integer values and mismatching candidates; focused pattern target 8/8, HQL2 387/0/1 across 34 targets; no runtime/contract/schema/P6/transport change; broad P8/P13 gates remain open | working-tree | ATHER |
| 0.1.55b | 2026-10-04 | beta | Add test-only HQL/typed-IR AnnotationLookup P7 differential (frozen target, evidence-only nonmatch, optional NULL, duplicate bag); target 6/6, HQL2 386/0/1 across 34 targets; clarify focused D7 ordering review/source-alias gap closure; no runtime/contract/schema/P6/transport change; broader P8/P13 gates remain open | working-tree | ATHER |
| 0.1.54b | 2026-10-03 | beta | Add test-only HQL/typed-IR scalar pipeline differential against P7 for six fixtures and 12 Storage executions; scalar target 2/2 and HQL2 383/0/1 across 33 targets; no runtime/contract/schema/transport change; broad oracle and P8/P13 gates remain open | working-tree | ATHER |
| 0.1.53b | 2026-10-03 | beta | Extend D7 with differential-proven single-label zero-hop HQL1 ID projection; adapter 10/10, HQL2 382/0/1 across 33 targets and P6/compatibility 194/0/0; other legacy forms and broad P8/P13 gates remain open | working-tree | ATHER |
| 0.1.52b | 2026-10-03 | beta | Add test-only HQL/typed-IR Values/UnionAll differential against P7 for 169 nullable bag pairs and 338 Storage executions; HQL2 382/0/1 across 33 targets, P6/compatibility 194/0/0; no runtime/schema/transport change; broad semantics and P8/P13 gates remain open | working-tree | ATHER |
| 0.1.51b | 2026-10-03 | beta | Synchronize merged PR #196 hosted status: 10 displayed checks pass, five fail, one skips; worker bootstrap failures across OSes and rebuilt Linux addon; Windows Rust failure at 15m16 has unverified cause; local History/Change 16/16 and HQL2 380/0/1 unchanged; worker approval and broad P8/P13 gates remain open | working-tree | ATHER |
| 0.1.50b | 2026-10-03 | beta | Extend storage-backed P7 ChangeScan differential to Edge/Row/Vector/Annotation alongside Node; seven event revisions match sequence, operation and identity; History/Change 16/16 and HQL2 380/0/1 across 32 targets; broader semantics and P8/P13 remain open | working-tree | ATHER |
| 0.1.49b | 2026-10-03 | beta | Add storage-backed P7 ChangeScan Node-revision differential; History/Change 15/15 and HQL2 379/0/1 across 32 targets; record PR #194 merged at 4f02d6b with 48 checks passing, five failing and five skipped; other event kinds, worker correction and broad P8/P13 remain open | working-tree | ATHER |
| 0.1.48b | 2026-10-03 | beta | Add WAL-derived P7 HistoryScan differentials for Node/Edge/Row/Annotation alongside Vector; History/Change 15/15, Annotation source 7/7, P7 130/130, HQL2 379/0/1, P6/schema-v6/compatibility 194/0/0; Artifact, broad ChangeScan, hosted review/checks and P8/P13 remain open | working-tree | ATHER |
| 0.1.47b | 2026-10-03 | beta | Add P7 Vector HistoryScan differential from WAL-derived retained revision facts; P7 130/130, History/Change 14/14, HQL2 377/0/1 and P6/schema-v6/compatibility 194/0/0; prior hosted head 9ada855 has five failed checks and no reviews; broad P8/P13 remains open | working-tree | ATHER |
| 0.1.45b | 2026-10-03 | beta | Implement and verify H2-D11 R4/P6 annotation ACL conformance for AnnotationScan and ChangeScan subjects; regression passes, HQL2 375/0/1 and current selected P6/schema-v6 43/0/0; prior hosted head c4f5729 retains four worker failures and cancelled Windows Rust; no schema/migration change, hosted/review and broad P8/P13 gates open | working-tree | ATHER |
| 0.1.44b | 2026-10-03 | beta | Synchronize current P6/C4/master/H2-D11/plan/registry and record PR #194 run 37083654705 at docs-only head 43cc6e8: four worker bootstrap failures plus one Windows Join budget failure; local Join target 5/5, exact budget dimension unconfirmed; no contract/schema change; PR unmerged, broad gates open | working-tree | ATHER |
| 0.1.43b | 2026-10-03 | beta | Synchronize P6/C4/master/plan/H2-D11/registry with the current 374/0/1 HQL2 regression result; record PR #194 head 31168524 and four worker-CI failures; retain worker-fix approval, unmerged, review and broad P8/P13 gates | working-tree | ATHER |
| 0.1.42b | 2026-10-03 | beta | Verify HQL/typed-IR Row HistoryScan/ChangeScan parity and exact row-property hydration (1/1); record HQL2 regression 374/0/1 across 32 targets; retain hosted CI/review/broad P8/P13 gates | working-tree | ATHER |
| 0.1.41b | 2026-10-03 | beta | Implement and locally verify HQL2/IR `tx_as_of` through one no-fallback frontier; focused 56/56, History/Change 14/14, HQL2 373/0/1 and P6/compatibility 194/0/0; retain hosted CI/review/broad P8/P13 gates | working-tree | ATHER |
| 0.1.40b | 2026-10-03 | beta | Synchronize accepted HQL2 `tx_as_of` snapshot semantics and document the runtime gap (normalizer rejection/current-frontier reads); implementation was pending at that checkpoint; retain hosted CI/review/broad P8/P13 gates | working-tree | ATHER |
| 0.1.39b | 2026-10-03 | beta | Implement approved HQL/typed-IR Vector HistoryScan under the P6 lease; focused history/change 12/12, HQL2 367/0/1 and P6/compatibility 194/0/0; correct stale current checkpoint wording and keep tx_as_of/hosted CI/P8/P13 gates open | working-tree | ATHER |
| 0.1.38b | 2026-10-03 | beta | Synchronize approved H2-D11 Vector HistoryScan identity, P6 vector floor/owner ACL and P8/plan scope; runtime and HQL/IR parity pending | working-tree | ATHER |
| 0.1.37b | 2026-10-03 | beta | Implement HQL Join lowering through the existing RowScan/Join contract for four kinds and bare JOIN default; HQL and typed IR match independent P7; record 365/0/1 across 31 targets; hosted CI for this local revision pending; retain merge/P8/P13/review gates | working-tree | ATHER |
| 0.1.36b | 2026-10-03 | beta | Add and pass storage-backed typed-IR Join P7 differential for all four kinds; record 364/0/1 across 30 HQL2 targets without expanding HQL JOIN; at parent head 484916b core checks passed but worker checks failed across OSes and no PR review exists; retain merge/P8/P13/review gates | working-tree | ATHER |
| 0.1.35b | 2026-10-03 | beta | Add and pass storage-backed HQL/typed-IR P7 aggregate differential over 121 nullable bags; record 363/0/1 across 29 HQL2 targets; hosted Rust/core checks pass on Linux/macOS/Windows, worker checks fail across OSes and review remains pending; retain merge/P8/P13/review gates | working-tree | ATHER |
| 0.1.34b | 2026-10-03 | beta | Add and pass storage-backed HQL/typed-IR P7 scalar differential over 81 nullable bags; record 362/0/1 across 28 HQL2 targets and 190/0/0 across 11 compatibility targets; current hosted worker checks fail; retain merge/P8/P13/review gates | working-tree | ATHER |
| 0.1.33b | 2026-10-02 | beta | Update hosted PR #194 evidence at cdfb90a: Windows Rust tests including protected probe passed but job timeout canceled cache save; worker tests fail across OS at markerless schema-v6 bootstrap; preserve probe and keep merge/P8/P13/review gates open | working-tree | ATHER |
| 0.1.32b | 2026-10-02 | beta | Synchronize hosted PR #194 evidence: Linux/macOS Rust and standard Node pass; Windows Rust hits 15-minute timeout in protected probe; worker tests fail on all OSes at markerless schema-v6 bootstrap; PR remains unmerged with review/P8/P13 gates open | working-tree | ATHER |
| 0.1.31b | 2026-10-02 | beta | Record full locked/offline Rust suite and both strict Clippy configurations; schema-v6 consensus revision-envelope signing order verified; `probe_vs_recall` remains NOT_RUN and broader P8/P13/review/release gates remain open | working-tree | ATHER |
| 0.1.30b | 2026-10-02 | beta | Implement and verify approved H2-D11 R6b markerless WAL recovery; crash 17/17, migration 19/19, selected 40-target aggregate pass; integrate upstream Query IR match_path; no user DB migration, full P8/P13/review gates remain open | 0135c29 | ATHER |
| 0.1.29b | 2026-10-02 | beta | Extend D7 with one-hop endpoint-ID exact string filter after legacy/HQL2 differential; record 9/9 focused tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 compatibility targets; retain shared-runtime, independent review and broad P8/P13 gates | working-tree | ATHER |
| 0.1.28b | 2026-10-02 | beta | Extend D7 with differential-proven one-hop forms and pre-parse resource reservation; record 8/8 focused tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 compatibility targets; retain shared-runtime, independent review and broad P8/P13 gates | working-tree | ATHER |
| 0.1.27b | 2026-10-02 | beta | Implement D7's initial actor-scoped HQL1 allowlist; record 5/5 focused tests and 354/0/1 across 27 HQL2 targets; retain other HQL1 forms, independent review and broad P8/P13 gates | working-tree | ATHER |
| 0.1.26b | 2026-10-02 | beta | Record edge-property-before-SHORTEST regression, 10/10 completion tests and 349/0/1 across 26 HQL2 targets plus separate 190/0/0 across 11 P6/schema-v6/compatibility targets; independent static review found no concrete defect; retain broad P8/P13 gates | working-tree | ATHER |
| 0.1.25b | 2026-10-02 | beta | Implement approved P8 D1-D5; record 9 focused passes and 528/0/1 across 37 explicit HQL2/P6/schema-v6/compatibility targets; retain independent review and broad P8/P13 gates | working-tree | ATHER |
| 0.1.24b | 2026-10-02 | beta | Record owner approval of P8 addendum D1-D6; authorize remaining bounded operator/literal implementation and corrected ACL fixture; no new runtime or acceptance evidence inferred | working-tree | ATHER |
| 0.1.23b | 2026-09-30 | beta | Implement lease-bound HQL/typed-IR Sequence node ID and labels; record 7 focused passes and 338/0/1 across 25 HQL2 targets; retain ACL-hidden fixture, properties/Compact and full P8 gates | working-tree | ATHER |
| 0.1.22b | 2026-09-30 | beta | Add test-only HQL root `MATCH` differential against independent P7 graph bag; record 1/1 focused and 331/0/1 across 24 root targets; document unfiltered interrupted command as NOT_RUN; retain broad exact-oracle, remaining operator, transport, review and P8 gates | working-tree | ATHER |
| 0.1.21b | 2026-09-30 | beta | Add HQL/typed-IR Original Rerank differential against independent P7 ranking; record 2/2 oracle tests and 330/0/1 across 23 root targets; retain broad exact-oracle, remaining operator, transport, independent-review and P8 gates | working-tree | ATHER |
| 0.1.20b | 2026-09-30 | beta | Add passing HQL/typed-IR exact-KNN differential against the independent P7 oracle for tie order and missing originals; record 329/0/1 across 23 root targets; retain broad exact-oracle, remaining operator, transport, independent-review and P8 gates | working-tree | ATHER |
| 0.1.19b | 2026-09-30 | beta | Implement checked same-type HQL `%`/typed-IR `rem`, NULL propagation and deterministic zero/overflow errors; record 79 focused passes and 328/0/1 across 22 targets; retain remaining pattern, operator, transport, independent-review and P8 gates | working-tree | ATHER |
| 0.1.18b | 2026-09-30 | beta | Default omitted HQL2 null ordering to NULLS LAST in either direction while preserving explicit NULLS FIRST and typed-IR requirements; record 58 focused passes and 325/0/1 across 22 targets; retain remaining pattern, operator, transport, independent-review and P8 gates | working-tree | ATHER |
| 0.1.17b | 2026-09-30 | beta | Implement structural unsigned HQL parameters from exact DecimalU64 values with no casts; record 50 focused passes and 323/0/1 across 22 targets; retain remaining pattern, operator, transport, independent-review and P8 gates | working-tree | ATHER |
| 0.1.16b | 2026-09-30 | beta | Implement typed-IR root Match anchors with exact RecordRef semantics, no direct lookup, pre-SHORTEST filtering, 18 focused passes and 319/0/1 across 22 HQL2 targets; retain remaining pattern, operator, transport, independent-review and P8 gates | working-tree | ATHER |
| 0.1.15b | 2026-09-30 | beta | Synchronize P6 spec with locally verified lease-bound HistoryScan/ChangeScan (9/9 focused; 318/0/1 across 22 HQL2 targets); retain transport, independent-review and full P8 gates | working-tree | ATHER |
| 0.1.14b | 2026-09-30 | beta | Implement HistoryScan/ChangeScan over exact retained revisions with recursive current ACL and bounded fail-closed behavior; record 9/9 focused and 318/0/1 across 22 HQL2 targets; retain lexical/context, constrained-pattern, transport, review and P8 gates | working-tree | ATHER |
| 0.1.13b | 2026-09-29 | beta | Implement exact HQL/IR KNN and Original Rerank on lease-bound original vectors; record 307/0/1 across 21 targets and retain History/Change, transport, review and P8 gates | working-tree | ATHER |
| 0.1.12b | 2026-09-29 | beta | Fix RowScan after_image hydration; the then-current 21-target sweep reported 296 passes, three vector operator failures and one ignored | working-tree | ATHER |
| 0.1.11b | 2026-09-29 | beta | Record focused RowScan property-hydration RED and RCA, annotate prior sweep scope, and retain approval/contract gates | working-tree | ATHER |
| 0.1.10b | 2026-09-29 | beta | Record the explicit 21-target HQL2 sweep (294 pass, three KNN/Rerank bind failures); keep oracle counts separate from P8 acceptance and retain open contract gates | working-tree | ATHER |
| 0.1.9b | 2026-09-29 | beta | Record 62 passing H2-D11 tests and 26 passing Blueprint fixture tests; retain unavailable operators and P8 gates | working-tree | ATHER |
| 0.1.8b | 2026-09-29 | beta | Record separate 62/62 H2-D11 fixture verification without mixing it into the P8 core count; retain HQL2 and P8 gates | working-tree | ATHER |
| 0.1.7b | 2026-09-29 | beta | Record structural HQL/IR root Match, shortest tie rule and 154 passing tests across ten focused targets; retain anchors/constraints, three unavailable vector executions and P8 gates | working-tree | ATHER |
| 0.1.6b | 2026-09-29 | beta | Record IR typed vector-parameter validation and 2 green tests; retain 3 red Knn/Rerank tests, unresolved public space_id mapping and P8 gates | working-tree | ATHER |
| 0.1.5b | 2026-09-29 | beta | Record structural HQL/IR Sequence Expand, optional/mode/budget/Analyze evidence and 206 passing tests across 21 targets; retain pattern-constraint and P8 gates | working-tree | ATHER |
| 0.1.4b | 2026-09-29 | beta | Record bounded HQL/IR Expand, separate AnnotationPut evidence fixture/oracle, and 199 passing tests across 20 targets; retain unsupported patterns and P8 gates | working-tree | ATHER |
| 0.1.3b | 2026-09-29 | beta | Record HQL/IR AnnotationLookup, frozen/live and optional/required tests; update matrix to 194 passing tests across 19 targets and retain remaining P8 gates | working-tree | ATHER |
| 0.1.2b | 2026-09-29 | beta | Record FieldIdV2/ExecBatchV2 selective hydration, dynamic-name regression and 188 passing tests across 18 HQL2/P6 targets; retain remaining P8 gates | working-tree | ATHER |
| 0.1.1b | 2026-09-29 | beta | Record four lease-bound source scans, annotation reference ACL, budgeted payload access and nullable identity/property results; keep selective hydration and full P8/HQL2 acceptance open | working-tree | ATHER |
| 0.1.0b | 2026-09-28 | beta | Record approved integration, reviewed scalar/core checkpoint, full-run failure and green targeted corrections; retain incomplete P8/HQL2 gates | working-tree | ATHER |
