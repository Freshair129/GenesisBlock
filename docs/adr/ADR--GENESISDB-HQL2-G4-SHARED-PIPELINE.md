---
doc_id: ADR--GENESISDB-HQL2-G4-SHARED-PIPELINE
version: "0.1.31b"
created_at: "2026-10-04T13:08:29+07:00,Codex,2be63a7"
last_update: "2026-10-29T09:43:02+07:00,Codex"
status: active
superseded_by: null
owner: "Boss (Founder / Product Authority)"
attributes:
  doc_type: architecture-decision
  domain: query-execution
  scope: UEE-HQL2 G4 shared pipeline, deterministic planning and truthful diagnostics
  complexity: C-3
  risk: HIGH
  amends: ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY
---

## Current Phase C2 composed scalar pipeline qualification (2026-10-29)

The owner-approved Phase C2 host qualification adds one test to the existing scalar-counter target and reconciles six existing governance documents. It exercises paired HQL2 and typed-IR `Values -> Filter -> Project -> Distinct -> Sort -> Offset -> Take -> Project` requests for nullable I64, Bool, DecimalU64, F64Finite, and Utf8. Literal output rows/schema and all operator row counters are the expected values. P7 independently checks the four represented value families; DecimalU64 is literal-only.

The five fixtures contain nulls and repeated inputs. Filter removes nulls and Distinct leaves unique single-key Sort values, so this evidence makes no claim about multi-key ordering or full-key ties. Across 80 requests, 30 Plans/Normal/Analyze requests succeed, 30 namespace controls deny, and 20 byte-budget controls fail closed. Plan precedes result queries and explicit publication; observed state is compared around Plan and error controls. The focused test passed 1/1 and the scalar-counter target 20/20 on one Windows author host. Strict core/default all-target Clippy, formatting, docs validation (0/244), version validation (0.2.10), and diff checks passed. See the [C2 P8 ledger](../REPORT--HQL2-P8-CORE-2026-09-28.md#current-phase-c2-composed-scalar-pipeline-qualification-2026-10-29) for the exact bounded matrix and raw evidence location.

This is test evidence only: it changes no G4 contract, runtime, API, schema, P6 authority, P7 interpreter, or error taxonomy. Broad type/source/order/composition/provenance, universal no-write and ordinary-success-state evidence, broad P6, P9-P16, independent runtime/performance, device and release gates remain open or NOT_RUN. Preserve P8 -> P9-P12 -> P13/P14 -> P15 -> P16.

## Historical Phase C1b nullable scalar ordering qualification (2026-10-28)

This owner-approved host-only tests/docs candidate extends the selected C1 ordering evidence to Nullable<Bool>, Nullable<DecimalU64>, Nullable<F64Finite> and Nullable<Utf8>; Nullable<I64> remains the C1 control. It adds one test to tests/hql2_g4_scalar_counter_tests.rs and reconciles these six existing governance documents. Runtime, API and normative contract behavior are unchanged.

The governing anchors are docs/SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY.md:390-393 (HQL omission defaults to NULLS LAST for ASC and DESC; typed IR requires explicit NullOrder), src/query/hql2/bind.rs::ordered and the named C1b test. The matrix issues 96 paired explicit-placement requests (four types x two directions x NULLS FIRST/LAST x HQL/IR x Normal/Plan/Analyze) plus 24 HQL-only omitted-placement requests (four types x two directions x three modes), 120 total. Literal ordered rows and root schema are asserted. The C1b-only helper checks Plan before Normal result queries, then Analyze; the original C1 helper and assertions are unchanged. P7 is used for Bool, F64Finite and Utf8, including default controls; DecimalU64 is literal-only. Repeated equal values assert multiplicity, not tie identity.

Author evidence on one Windows host: the full tests command reported 158 result blocks, 1,299 passed, 0 failed and 6 ignored; the 44 root HQL2 targets reported 468/0/1, with the scalar-counter target 19/0/0. The whole-suite run used the first C1b test helper and preceded its C1b-only Plan-before-result ordering refactor, plus rustfmt normalization; it is not a whole-suite run of the final source. The corrected formatted scalar-counter target passed 19/19, and strict core/default all-target Clippy and fmt checks passed after that refactor. Docs validation (0 violations in 244 files), version validation (0.2.10) and diff check passed for the reconciled documents. PR exact-head hosted workflows remain a separate gate. Raw commands/logs and the D-volume linker tooling RCA are retained in D:/GenesisBlock_Dev/local-cleanup-backups/20261019T030925Z-genesisblock/branch-review/g4-p8-c1b-nullable-ordering-20261028. This is author host evidence, not hosted CI, independent runtime or cross-device acceptance.

C1b is finite evidence, not broad G4/P8 closure. It does not establish exhaustive types/sources/nullability, multi-key order or full-key ties, composition, complete provenance, universal no-write, or ordinary successful-result state outside the selected guards. Broad P6 and P9-P16, independent runtime/performance, device and release gates remain OPEN or NOT_RUN. Preserve P8 -> P9-P12 -> P13/P14 -> P15 -> P16.

## Post-PR273 Phase C1 ordering qualification status (2026-10-27)

PR #273 merged normally: PR head 774c3aa428dec5d5562c21f7efef30fe80e80b72, merge 445a07942edb941893f7e0ab68288f5abcfb9538, exact tree 694ecf0796d40bf3e36a00acc094167678fa7078; fetched origin/main matches that tree. Its hosted rollup had 15 SUCCESS checks plus one genuine schedule-only security-advisory SKIPPED check, and Rust Ubuntu/Windows/macOS suites passed. GitHub recorded no formal PR review event (blank reviewDecision and empty reviews/reviewRequests); no reviewer runtime rerun is claimed.

The selected local suite was 1,298/0/6 across 158 result blocks, including 44 root HQL2 targets at 467/0/1. That full run preceded only a mechanical test-helper Clippy cleanup; the focused C1 test (1/1) and shared scalar-counter target (18/18) passed after it. Strict core/default all-target Clippy, formatting/diff, docs validation (0/244) and version validation (0.2.10) passed.

A selected exact-main P6 host run passed 77/0/0 across nine existing targets, with cargo check, fmt, docs validation (0/244) and diff check passing. This is one local Windows author run, not independent P6 Verify/Review/Final or cross-platform/device/release evidence; see the P8 report and P6 specification for limits and raw evidence. No P6 contract or runtime change is implied.

This reconciles hosted status only. C1 does not close G4/P8: full-key tie policy remains unspecified and exhaustive type/source/order/composition/provenance, universal no-write, ordinary-success-state, broad P6, P9-P16, independent runtime/performance, device and release gates remain open or NOT_RUN. Preserve P8 -> P9-P12 -> P13/P14 -> P15 -> P16. See the [current P8 ledger](../REPORT--HQL2-P8-CORE-2026-09-28.md#post-pr273-phase-c1-ordering-status-2026-10-27).

## Historical pre-PR272 Phase B1 scalar type-admissibility candidate (2026-10-26)

Current base is fetched `origin/main` `6bd493e7d8cc58d38b27a4c916b0d16237aeceb0`
(tree `0f9480e3badf0137841e2e7aa3f05789d7bf1f2a`), after PR #271. Owner
approved the tests/docs-only Phase B1 scalar type-admissibility candidate. Its
six new tests are in the existing scalar counter target; the focused target
passed 17/17 with zero failures or ignored tests. Repo-wide
`cargo test --locked --offline --no-default-features --jobs 1 --tests` completed
with exit 0 (158 result blocks: 1,297 passed, 0 failed, 6 ignored); its 44 root
HQL2 targets recorded 466 passed, 0 failed, 1 ignored. Core/default all-target
Clippy, formatting, docs validation (0/244), version check (0.2.10), and diff
checks passed. Final independent source review passed on commit
d6978664ae6c5797887e0b968d1bf06976d6e891, tree
baac58b38fb7a0f27a1f0e055992ce11a05503bb, with 0 Critical, 0 Important and 0
Minor findings; reviewer runtime is NOT_RUN. At this candidate snapshot,
formal hosted review and exact-head CI were pending; PR #272 later merged the
Phase B1 change.

The matrix covers `Values`, `Filter`, `Project`, `Distinct`, Sort key type,
`Take`, and `Offset`/HQL `SKIP`; selected scalar type admission, comparisons,
registered functions, checked arithmetic and DecimalU64 structural pagination.
Sort cases do not qualify ordering semantics. This adds no G4-D01..D10
decision, accepted contract, runtime behavior, shared P7, or P6 authority.
G4/P8 remains partial; complete type/order/composition/provenance, broad P6,
P9-P16 and performance/device/release gates remain open or NOT_RUN. See the
[current source-linked P8 ledger](../REPORT--HQL2-P8-CORE-2026-09-28.md#post-pr271-phase-b1-scalar-type-admissibility-status-2026-10-26).
Preserve P8 -> P9-P12 -> P13/P14 -> P15 -> P16.

## Historical post-PR269 qualification snapshot (2026-10-26)

The following PR269 status was recorded before PR #271 and is retained as a
dated snapshot, not current main status. Its SourceScan section's candidate-time
review/CI wording is historical and does not describe Phase B1.
## Approved G4/P8 SourceScan result metadata qualification - 2026-10-26

Owner approved tests/docs-only seven existing files (LOW, C-3): one appended
SourceScan test with local helpers and these six governing documents. No
production/runtime/API/P6/WAL/hash/schema/ACL/planner/index/grammar/shared-P7 or
error-contract change. Baseline fetched `origin/main`
`c9fbc27f657e89e75f77d415d195a59311fd9d31`, tree
`9d530693b10177ba2facd8e2ba4dd811adfa7c10`.

The selected matrix has 48 requests: 16 pre-execution Plan controls and 32
Normal/Analyze results across Node, Edge, Row and Annotation; populated/empty
fixtures; and paired HQL/typed-IR frontends. Each fixture's Plan requests run
before its Normal/Analyze requests. The appended test is
`tests/hql2_g4_source_counter_tests.rs::source_result_metadata_has_independent_fixture_expectations_in_hql_and_ir`.

Expected metadata is derived from independent literals and fixture setup:
ordered root schema and source lineage, request ID, fixture-key/domain-derived
database ID, explicit publication receipt and selected snapshot fields, one
fixed fixture `valid_at`, four semantics fields, completeness, empty index
frontiers and a cursor key explicitly present with JSON null. Node/Edge/
Annotation identities are checked against fixture literals. Row IDs are UUIDv4
values assigned during fixture publication; the test reads `record_id` from the
fixture revision table before query execution to seed P7. The RowScan comparison
is therefore fixture-bound P7 evidence, not a precomputed literal UUID oracle.
Actual result order is not normalized. Plan asserts
the physical SourceScan, root schema, catalog policy revision/frontier and
absence of actuals/estimates. Analyze checks selected source counters and keeps
elapsed time typed unknown where uninstrumented. Metadata equality across modes
or frontends is not used as the expected oracle.

Host regression for candidate test source blob `cc1fa5dcd35b83424240272c0055124818fd03ab`: 460 passed/0 failed/1 top-level ignored across 44
HQL2 targets, plus 145/0/0 across 11 compatibility targets; 605 unique passing
target/test pairs, with the focused SourceScan test included once. Ignored parser
child invocations are excluded from that unique top-level count. Strict core/default all-target Clippy, formatting, documentation validation
(244 files, zero violations) and version consistency (0.2.10) pass for the
recorded candidate source. Primary task-agent source/evidence/architecture review
of the exact candidate source and seven-file scope completed with zero unresolved
findings; reviewer runtime was NOT_RUN. Required PR review and hosted exact-head
CI remain PENDING. The raw runs, unique-pair inventory and source hashes
are recorded under
`D:/GenesisBlock_Dev/local-cleanup-backups/20261019T030925Z-genesisblock/branch-review/g4-p8-sourcescan-result-metadata-qualification`.

This is selected fixture metadata qualification, not a complete P6 provenance
proof. Existing lock/shm size-only file observations do not prove universal
transient/internal-write absence or ordinary successful-execution state effects.
Broad G4/P8/P9-P16, exhaustive types/order/composition/provenance, independent
runtime/new oracle/other machine, performance, device and release gates remain
open. Preserve P8 -> P9-P12 -> P13/P14 -> P15 -> P16; devices are unavailable.

Current selected evidence: [P8 SourceScan result metadata](../REPORT--HQL2-P8-CORE-2026-09-28.md#approved-g4p8-sourcescan-result-metadata-qualification---2026-10-26).

## Approved G4/P8 Join result metadata qualification - 2026-10-24

Owner approved tests/docs-only7existingfiles (LOW,C-3): one appended Join test
and these six governing documents. No production/runtime/API/P6/WAL/hash/schema/
ACL/planner/index/grammar/shared-P7/error-contract change. Baseline merged PR266
main `9a4e543a75a401cd2afb6f001fa13cc8952583d1`, tree
`95cf91c4a5428edb8aab5c2898c60d873cebc7a0`.

Selected Join metadata matrix:96requests=32prepublicationPlan controls+
64normal/Analyze successes, four Join kinds x four populated/empty-side datasets
x paired HQL/IR. Literal Join schema distinguishes Inner's nonnullable n/r,
Left's nullable right Entity, and Semi/Anti's absent right column. Root prop
columns are literal Nullable<Json>, with right_key only for Inner/Left; empty
results keep the same declared schemas. Nodes are located through root/input
lineage, not generated HQL IDs. SQL NULL and JSON null remain distinct and
duplicate bag multiplicity is checked against literal bags AND unchanged P7;
no ordering guarantee or complete JSON oracle is inferred.

All successful envelopes independently expect ordered root schema, request_id,
five snapshot fields, four non-ranking semantics fields, complete/reason=null/
eligible_count_known=true, empty index_frontiers and an explicitly present null
cursor. Identity comes from fixture public key/domain SHA256, never a discovery
query. Snapshot tx/generation/policy come from explicit publication receipt;
valid_at is captured once after setup before Plan, then reused with selected
tx_as_of. Analyze catalog uses separately captured live frontier/publication_seq,
not selected snapshot data tx. Receipts are not a complete independent P6 verifier.

Each dataset/kind/frontend gets a fresh enforced-policy fixture. Plan before any
publication/query execution expects literal Join/root schemas, catalog policy1/
captured frontier and no actuals/estimates, preserving observed files/frontier
with existing lock/shm size-only exceptions. Analyze preserves typed counters:
source elapsed stays unknown/not_instrumented; Join/Project elapsed is measured.
No timing/performance threshold or universal internal/transient/ordinary-success
no-write claim. Original Join functions, other tests and shared P7 remain unchanged.

Author final regression: 459/0/1 across44HQL2 targets plus145/0/0 across11
compatibility targets = 604 unique passing target/testpairs. Focused Join
results are included once, not added again. Top-level ignored parser child is
explicitly invoked twice by its passing parent; child invocations are excluded
from the main count. Primary strict core/default all-target Clippy,fmt,docs244/0,version0.2.10 and
diff checks passed. Fresh final source review/current-head hosted CI remain
PENDING until exact-head external records complete.

PR266 completed four workflows,15SUCCESS+1genuine schedule-only advisory skip,
including actual Rust Ubuntu/Windows/macOS, before normal merge9a4e543 at
2026-10-24T11:41:17Z (18:41:17+07:00). Reviewed headca9a686/tree95cf91c;
fetched-main/reviewed-tree parity was verified, not a runtime rerun. Final source/
evidence/architecture review0Critical/0Important/0Minor followed the corrected
cursor-presence assertion and full requalification. Prior catalog3/livecatalog4
fixture failure and initial Minor were retained/resolved, not new failures.
Its603author pairs and priorPR265600pair independent same-host reproduction are
separate dated evidence, not execution of this new Join test.

```text
literal Join/root schemas + literal/P7 bags + fixture identity
  -> prepublication Plan (catalog, observed state)
  -> explicit receipt + post-setup valid_at/selected tx
  -> HQL/IR normal/Analyze independent envelope -> host regressions
  -> final source/evidence review -> exact-head hosted CI
P8 -> P9-P12 -> P13/P14 -> P15 -> P16 (device/release remain open)
```

Broad G4/P8/P9-P16, exhaustive JSON/types/order/composition/provenance, universal
no-write/ordinary-success state effects, complete P6, independent new-test runtime/
new oracle/other-machine/performance/device/release acceptance remain open.
Devices unavailable; no repeat device request, release/tag/publication, surface
enablement or user-database migration authorized. Historical pending/NOT_RUN
sections remain dated checkpoints, not retroactively executed evidence.

Current selected evidence: [P8 Join result metadata](../REPORT--HQL2-P8-CORE-2026-09-28.md#approved-g4p8-join-result-metadata-qualification---2026-10-24).



## Approved G4/P8 independent result metadata qualification - 2026-10-24

Owner approved tests/docs-only9existing files (LOW,C-3): three existing test
targets and these six governing documents. No production/runtime/API/P6/WAL/
hash/schema/ACL/planner/index/grammar/shared-oracle/error-taxonomy change.
Baseline merged PR265 main `5bfc475eef6a66aa2cc5905ca43a469e8fc30684`,
tree `c48584599eaff12bcea70ad5c392624aec10a2b7`.

The new selected matrix contains96requests:32Plan controls before publication
and64normal/Analyze successes across16fixtures, paired HQL/IR. Scalar preserves
surviving NULL through Project/Distinct/Sort/Offset/Take/Return; UnionAll adds
selected Distinct/Sort/Project results; global/grouped Aggregate checks seven
existing functions. Nullable duplicates,all-NULL,empty,Take0 and empty branches
are finite partitions. Sorted scalar/set actual rows are compared without
sorting them; unordered aggregate groups remain bags.

Each successful envelope is compared independently to fixture expectations:
ordered column names/type/nullability; request_id; all five snapshot fields;
four non-ranking semantics fields; complete/reason=null/eligible_count_known=true;
empty index_frontiers and an explicitly present null cursor. Rows match literal
values AND unchanged
P7. Expected database identity is SHA256 of fixed domain bytes and the fixture's
public verifying key, never obtained by executing a query. Explicit publication
GenerationInfo supplies tx/generation/policy receipts; fixed valid_at and tx_as_of
are reused. These publication receipts are not an independent proof of all P6
internals. Empty result schema is declared, not inferred from rows.

Each frontend/case has a fresh enforced-policy store. Plan runs before publication,
expects literal root columns and catalog policy_revision1/captured frontier, no
actuals/estimates, and unchanged observed files/frontier (lock/shm size-only
exceptions). Execution snapshot receipts remain distinct from Plan's catalog
stamp. Normal has no explain; Analyze keeps measured readings and typed unknowns.
Existing counter/namespace/budget/error fixtures and all original test functions
remain unchanged. No universal transient/internal-write or ordinary-success
no-effect claim is made.

Author regression: HQL2 458 passed/0failed/1top-level ignored across44targets;
compatibility145/0/0 across11targets; 603 unique passing target/test pairs.
The ignored parser child is invoked in default/detailed processes by its passing
parent and excluded from the unique main count. Focused three targets pass;
focused results are included once in the broad total, not added again.
Primary strict core/default all-target Clippy,fmt,docs244/0,version0.2.10 and
diff checks passed. Fresh final source review/current-head hosted CI remain
PENDING until their exact-head external records are complete.

PR265's separate same-host independent verifier600pairs and sealed-before-author
comparison remain historical evidence for its unchanged inventory, not execution
of these new tests. Its final review0Critical/0Important/0Minor was source-only;
all four hosted workflows passed,15SUCCESS+1genuine schedule-only advisory skip,
including actual Rust Ubuntu/Windows/macOS, before normal merge5bfc475 at
2026-10-24T06:37:04Z (13:37:04+07:00). Fetched/reviewed-tree parity was verified,
not another runtime run. Prior pending/NOT_RUN sections are dated checkpoints.

```text
literal schema/rows + fixture public-key identity -> prepublication Plan control
  -> explicit generation receipt -> HQL/IR normal and Analyze expected envelope
  -> focused + broad host regressions -> final source/evidence review -> full CI
P8 -> P9-P12 -> P13/P14 -> P15 -> P16 (device/release gates remain open)
```

Broad G4/P8/P9-P16, exhaustive types/order/composition/provenance, independent
new implementations/other-machine/performance/device/release acceptance remain
open. Devices unavailable; no further device request, release/tag/publication,
public surface enablement or existing-user database migration is authorized.

Current selected evidence: [P8 result metadata](../REPORT--HQL2-P8-CORE-2026-09-28.md#approved-g4p8-independent-result-metadata-qualification---2026-10-24).



## Approved G4/P8 independent host reproduction - 2026-10-24

Owner approved this six-existing-document qualification lane (LOW, C-3) and
host-only verification; no production/test/runtime/API/P6/WAL/hash/schema/ACL
change. Baseline is merged PR264 main
`f060b387e66d72c21851cc62ed22dd45fbdae2d4`, tree
`57a077736ffef41eb505c44e3198ba892065bf32`.

A fresh verifier context independently executed the existing frozen inventory
from an exact-source snapshot, using a fresh build target and separate snapshot
working directory. All1,041 tracked files matched exact Git blob bytes before
execution. Snapshot `D:/GenesisBlock_Dev/g4ir-raw`, fresh output
`D:/GenesisBlock_Dev/g4ir-target`; shared Cargo registry cache was allowed,
while author test binaries/fixtures were not reused. This is separate-executor
reproduction on the same Windows host, not a new machine, independent engine/
oracle implementation, blind study, device or release qualification.

Verifier HQL2:455 passed/0 failed/1 top-level ignored in44 targets;
compatibility:145/0/0 in11. The600 passing target/test pairs
match the prior author inventory exactly. The ignored parser child is invoked
twice by its passing parent. A separate nocapture parser-parent rerun exposes
two successful child configurations; that rerun/children are excluded from the
main count. Verifier sealed its own raw outcomes/commands/times/hashes before
root compared them with author264 evidence. Complete source hashes remained
unchanged. Relative fixture paths use the separate snapshot; tests retain their
existing fresh TempDir allocation under the shared OS TEMP root. This is not a
filesystem security sandbox. Same-host finite reproduction is now recorded for this inventory;
full G4/P8 correctness or exhaustive type/order/composition/provenance coverage
is not thereby accepted.

PR264 completed four workflows,15 SUCCESS+1 genuine schedule-only advisory skip,
including actual Rust Ubuntu/macOS/Windows, before normal mergef060b38 at
2026-10-23T23:12:58Z (2026-10-24 06:12:58+07:00). Reviewed-tree parity is a
separate check, not another runtime run. Prior author/source-review/hosted and
this verifier's execution remain separate evidence records. Historical pending/
NOT_RUN statements below are dated checkpoints; the current reproduction item
is qualified only to the finite scope above.

```text
exact main + tracked-source hashes -> fresh snapshot/target
  -> independent verifier execution -> seal -> author comparison
  -> six-doc candidate -> final source/evidence review -> current-head CI
P8 -> P9-P12 -> P13 surfaces/P14 recovery -> P15 security -> P16 qualification
```

Primary strict core/default all-target Clippy, fmt, docs validation (244 documents,
0 errors), version consistency (0.2.10) and diff checks passed. This candidate's
final source review and hosted CI remain PENDING until exact-head records are
finalized externally. Broad
G4/P8/P9-P16, independent implementations/exhaustive semantics, universal no-write,
performance, physical-device and release acceptance remain open. Device execution
is NOT_RUN because the owner reports devices unavailable; no device request,
publication, release/tag or existing-user-database migration is authorized.

Current evidence: [P8 independent host reproduction](../REPORT--HQL2-P8-CORE-2026-09-28.md#approved-g4p8-independent-host-reproduction---2026-10-24).

Version transition: `0.1.20b` -> `0.1.21b`; independently executed host evidence and PR264 reconciliation only.

## Approved G4/P8 broad host qualification - 2026-10-24

Owner approved a six-existing-document evidence consolidation (LOW, C-3),
including existing HQL2/P6/compatibility regressions, one fresh whole-scope
source/evidence/architecture review and normal PR delivery after complete CI.
No test/production/runtime/API/P6/WAL/hash/schema/ACL/stream or surface contract
changes are admitted. Host first; owner reports physical devices unavailable.

Current baseline is PR263 merged main `0a5db778e533152d20fdbd8be7dfcc0e03c4fc4b`
with reviewed source tree `ed2ba0fcf3d802f0c5e2f66a66c3a817c774ee56`.
The P8 report's **historical PR264 G4/P8 qualification ledger** maps all eight
G4 criteria, all 19 executable `Kernel` variants, six `BoundSourceV2` partitions,
exact-KNN/Original-Rerank modes and independently expected versus self-consistent
metadata. Every kernel has selected test evidence and explicit residuals; no
whole type/frontend/mode/temporal/ACL/budget cross-product is claimed.

Author HQL2 run:455 passed/0 failed/1 ignored across44 root integration targets,
locked no-default-features/jobs2/serial. The ignored parser child is explicitly
invoked twice by its passing parent (default/detailed diagnostics);455 excludes
those child invocations. Initial recursive count49 included support helpers.
Separate P6/Query IR v1/REST/schema compatibility run:145 passed/0 failed/
0 ignored in11 targets. Combined author execution600 passed/0 failed/1 ignored
in55 targets;600 distinct target/test pairs are not600 independent semantic
obligations. Strict core/default all-target Clippy, fmt/docs244/0/version0.2.10/
diff checks pass. This candidate's fresh review/hosted checks remain PENDING.
Author execution, source review and independent runtime reproduction are
separate; reviewer runtime/device/release are NOT_RUN.

PR263's four workflows completed success with15 SUCCESS+1 genuine schedule-only
advisory skip, including actual Rust Ubuntu/macOS/Windows suites, before normal
merge at0a5db77. Reviewed-tree parity was verified, not a runtime rerun. Historical
candidate pending statements below remain dated checkpoints. This candidate
needs its own exact-head hosted qualification.

```text
G4/P8 finite ledger + host regressions -> independent whole-scope review
P8 -> P9 -> P10 -> P11 -> P12 -> P13 surfaces + P14 recovery
  -> P15 security -> P16 host/device/release/traceability
```

Broad G4/P8 remains PARTIAL. Missing selected proof is MISSING_EVIDENCE;
unsupported v2 stream and future surface/planner behavior need separate
contract approval, not implied test coverage. Existing storage indexes and
core/additive NAPI v2 are not absent merely because broader P9/P13 remain open.
Physical device availability blocks device execution; simulator/emulator/host
results do not replace it. Release publication is not a verification dry run.

Canonical ledger: [current P8 acceptance partitions](../REPORT--HQL2-P8-CORE-2026-09-28.md#current-g4p8-broad-host-qualification-ledger---2026-10-24).

Version transition: `0.1.19b` -> `0.1.20b`; broad qualification evidence/navigation only.

## Approved G4 checked arithmetic frontend diagnostics - 2026-10-24

Owner approved the seven-existing-file tests/docs-only slice at merged main
`2ec5b2e` (LOW, C-3). The existing scalar-counter target adds paired HQL/IR
normal/Plan/Analyze fixtures for checked I64 Add/Div/Rem, preserving all eight
existing tests and the independent shared P7 interpreter. Nine positive cases
compare P7 AND literal ordered rows (no sorting of actual results): duplicate/
NULL bags, signed truncating division/remainder, empty input and all-NULL input
with rhs0. Five error cases independently check P7 INTEGER_OVERFLOW or
DIVISION_BY_ZERO and existing Storage BIND_ERROR/execute with the precise
integer_overflow/division_by_zero detail reason. The emitted error envelope is
qualified as existing behavior; no new error taxonomy is introduced.

The two-node Values/Project lineage has literal fixture row counts, measured
zero, typed unknown availability and monotonic elapsed provenance. Allowed and
denied Plan controls precede first execution/publication, including numeric
operands that fail at execution. Plan catalog policy revision1 and captured
pre-Plan frontier are asserted explicitly. Execution denial and arithmetic Err
controls follow explicit generation publication and observe stable frontier/
file set/bytes; established lock/shm size-only exceptions remain. Earlier valid
or NULL rows in failing fixtures never become a successful partial result.
Metadata mode equality is consistency only; no independent complete schema/
provenance claim or universal internal/transient-write absence follows.

```text
HQL / typed IR -> bind/plan -> Plan before publication -> catalog/state controls
                          -> publish fixture generation once
                          -> normal / Analyze -> checked rows or typed Err
independent P7 + literal rows / P7 error ----------> selected assertions
frontier/files around Plan/denial/arithmetic Err -> observed state boundary
```

Final author runtime passes105/105 unique tests across11 targets, zero failed/
ignored; scalar10/10 includes the focused run once. First focused run passed,
with no fixture correction or RCA. Selected matrix168 requests=64 successes+
84 namespace denials+20 arithmetic errors,132 observed state guards. Source
blob `95b2b49e1a18bd7c85594cc52eb01e859e434646`; original eight scalar tests/helpers/
imports unchanged. Strict core/default all-target Clippy, formatting, docs244/0,
version0.2.10 and diff checks pass. Fresh source/evidence/architecture review is
retained separately; no reviewer runtime rerun is claimed. Hosted qualification
is NOT_RUN for this candidate
until all exact-head CI is observed.

Seven existing tests/docs files only; no runtime/planner/API/P6/WAL/hash/schema/
ACL/index/grammar/shared-oracle changes. F64/nonfinite/signed-zero, exhaustive
numeric/type/order/composition, broad G4/P8/P9-P13 and independent runtime/
provenance/performance/device/release gates remain open. Ordinary successful
execution state is outside the selected guarded controls.

### Integrated PR262 evidence reconciliation

PR262 merged as `2ec5b2e6d6b2f325407dd1bc9f84d5cdc02634b9` after four exact-head
workflows succeeded: Tests39981972283, Security39981972270,
performance39981972408 and worker39981972332. Full rollup15 SUCCESS+1 genuine
schedule-only advisory SKIPPED; actual Rust Ubuntu/Windows/macOS suites passed.
Author local103/103 unique tests across11 targets, scalar8 included once, and
strict static gates passed. Fresh whole-branch source/evidence/architecture
review initially found1 Important missing Plan catalog-stamp assertion; fixture
expectations were added, full qualification rerun and same-reviewer confirmation
reported0 unresolved Critical/Important/Minor. No reviewer runtime rerun.
Fetched merged tree `186d58f2784c01c074c960fcaad965e8c5d3a907` equals reviewed tree;
parity is not new runtime execution. Candidate NOT_RUN/pending statements for
PR262 below are dated checkpoints superseded by this reconciliation.



## Approved G4 scalar frontend diagnostics - 2026-10-23

Owner approved the seven-existing-file tests/docs-only slice at merged main
`564f28f` (LOW, C-3). The existing scalar-counter target adds paired HQL/IR
normal/Plan/Analyze fixtures through Values/Filter/Project/Distinct/Sort/
Offset/Take/final Project. Independent P7 and literal ordered rows qualify
SQL-NULL filtering, duplicates, empty/all-filtered inputs, offset beyond the
result and TAKE0; actual rows are not sorted by the test. Surviving-NULL sorting
remains covered separately by the preserved nullable scalar regression.
Eight-node rows_in/out are independent fixture cardinalities, located by
lineage and logical kind rather than generated IDs. Typed unknown measurements,
measured zero and monotonic elapsed provenance remain distinct.

Allowed and denied Plan controls precede first query execution/publication.
Normal/Analyze denial and byte-budget controls follow explicit generation
publication. Frontier/file observations include the established lock/shm
size-only exceptions; they do not prove universal transient/internal-write
absence or ordinary nonempty successful execution state. Metadata mode equality
is consistency, not independent full provenance. Same-request shape/hash
identity does not imply cross-frontend equal hashes or authority.

```text
Values -> Filter -> Project(y) -> Distinct -> Sort -> Offset -> Take -> Project(return)
HQL/IR -> bind/plan -> Plan before publication -> observed frontier/files
                  -> normal/Analyze after publication -> rows/counters/errors
independent P7 + literal ordered rows -----------------> selected assertions
```

Final author runtime passes103/103 unique tests across11 targets, zero failed/
ignored; final scalar target8/8 is included once. Selected matrix152 requests=
62 successes+42 namespace denials+48 budget errors,124 observed state guards.
Source blob `cb1b93021099f041af124a238c5e99fd7be676b1`; prior seven scalar test/helper
bodies are unchanged. Strict core/default all-target Clippy, formatting, docs244/0,
version0.2.10 and diff checks pass. Independent source/evidence/architecture
review is recorded separately; no reviewer runtime rerun is claimed.
Hosted qualification is NOT_RUN for this
candidate until complete exact-head CI is observed.

Seven existing tests/docs files only; no runtime/planner/API/P6/WAL/hash/schema/
ACL/index/grammar/shared-oracle changes. Broad G4/P8/P9-P13, exhaustive types/
order/composition, independent runtime/provenance, performance/device/release
gates remain open.

### Integrated PR261 evidence reconciliation

PR261 merged as `564f28f6a1c3951701e1158a822c103493c4d7e7` after all four
exact-head workflows completed successfully: Tests39949504050,
Security39949504060, performance39949504095 and worker39949504118.
The full rollup was15 SUCCESS and1 genuinely schedule-only advisory SKIPPED;
applicable Rust suites on Ubuntu/macOS/Windows succeeded. Author local87/87
unique tests across10 targets, focused2 included once, and fresh source/evidence/
architecture review0 Critical/0 Important/0 Minor remain distinct evidence.
Fetched main tree equals reviewed tree `8bccf46d4a4a6bd900c024aefef0aa2fd35175d3`;
this parity is not a new runtime rerun. Earlier candidate NOT_RUN/pending lines
below are historical checkpoints superseded for PR261 by this reconciliation.



## Approved G4 UnionAll frontend diagnostics - 2026-10-23

Owner approved tests/docs-only qualification at merged main `6cf46f9`
(LOW, C-3), extending the existing Storage set-oracle target. The169-pair
normal HQL/IR/P7 semantic oracle is preserved. New selected fixtures compare
both frontends through Values/branch Projects/UnionAll/Sort/final Project,
with independent P7 and literal ordered rows, including duplicates, NULL and
left/right/both-empty inputs. Per-node rows are fixture cardinalities; measured
zero and typed unknown remain distinct. Generated IDs and cross-frontend equal
hashes are not acceptance requirements. Existing repeated IR edge counter and
golden regressions are retained.
Plan controls precede query execution/publication. Execution denial and byte
budget controls use an explicitly published generation; frontier/files are
observed before/after, with only established lock/shared-memory size comparisons.
No universal internal/transient-write claim follows. Metadata mode equality is
consistency, not independent full provenance acceptance. The complete result
envelope counts toward byte limits, including empty results.
Local runtime passes87/87 unique tests across10 targets, zero failed/ignored;
focused2/2 is included once. Strict core/default all-target Clippy, formatting,
docs244/0, version0.2.10 and diff checks pass. Hosted qualification is NOT_RUN for
this candidate. Seven existing tests/docs files only; no runtime/planner/API/
P6/WAL/hash/schema/ACL/index/grammar changes. Broader type/composition,
G4/P8/P9-P13, independent runtime/device/release gates remain open.

PR260 reviewed head `ca7f88402b0bca9c8058164a1ee2492f5e078b29` merged as
`6cf46f90a0b57b081f7d0d3d08833422fbcffe7c` after four successful exact-head
workflows and15 SUCCESS checks plus one genuine schedule-only skip. Tests
`39899519869`, Security `39899519859`, worker `39899519995` and performance
`39899519858` succeeded, with actual Rust Ubuntu/macOS/Windows steps successful.
PR260 local79/79 across8 targets and strict static gates passed; its fresh
whole-branch source/evidence review found Critical0/Important0/Minor0 and did
not rerun runtime suites. Merged tree `edb4047a0ebeecdcf8d4951cafd24f48b755dde5`
equals the reviewed/tested tree, not a new post-merge runtime run. Earlier
hosted NOT_RUN/pending entries below are dated pre-CI checkpoints; historical
fixture failures/RCAs remain preserved, not current uncorrected failures.



## Approved G4 Aggregate frontend diagnostics - 2026-10-23

Owner approved tests/docs-only qualification at merged main `1821da9`
(LOW, C-3). The existing 121-bag normal HQL/IR/P7 oracle is retained.
The added selected matrix pairs HQL/IR global and grouped Aggregate through
Storage: empty input, all NULL, repeated positive values and repeated signed
values. All seven existing aggregate functions are checked against independent
P7 bags; global emits one row even on empty input, grouped outputs are
hand-specified as 0/1/3/3. Values/Aggregate/Project row counters are fixture
cardinalities; other counters keep typed unknowns and explicit units.
Plan requests precede execution/publication; denied and output-budget controls
preserve observed frontier/file state after explicit publication. Lock and
SQLite shared-memory bookkeeping remain size-only; no universal internal or
transient-write claim is made. Snapshot/metadata mode equality is consistency,
not independent provenance acceptance. Same-request plan identity never
implies cross-frontend semantic equality or access authority.
Local runtime passes79/79 unique tests across8 targets, zero failed/ignored;
focused2/2 is included once. Owner explicitly expanded the limit to eight
existing files for the pre-existing Join fixture date correction: capture one
valid_at after setup and reuse before Plan, retaining every row/counter/state
assertion. The original regression failure and RCA remain recorded below.
Strict core/default all-target Clippy, formatting, docs244/0, version0.2.10 and
diff checks pass. Hosted qualification is NOT_RUN for this candidate.
No production/runtime/planner/API/P6/WAL/hash/schema/ACL changes. Broader
operator breadth, G4/P8/P13, independent runtime/device/release gates stay open.

PR259 head `83e35fd6274f8d0982c3f3f071e8c739a7f57365` merged as
`1821da9b89dd7b418ac7b507d2a510c2b799d36e` after four successful
exact-head workflows and 15 SUCCESS checks plus one schedule-only skip.
Tests `39826465866`, Security `39826465863`, worker `39826465865`,
performance `39826466200` completed successfully, including Rust Ubuntu,
Windows and macOS. Merged tree `00ac16259742eb2fe6473bc097cc70c6d22bb2cc`
equals the tested tree; this is not a fresh post-merge runtime rerun.
Earlier pending statements below are historical checkpoints. The deferred
Minor Join temporal RCA pointer is corrected in the P8 report to
`query/hql2/request.rs::normalize`; raw historical logs are retained.



## Approved G4 storage-backed Join diagnostic qualification - 2026-10-23

Owner approved tests/docs-only scope at merged main `e2be296` (LOW, C-3),
extending the existing HQL/IR NodeScan/RowScan Join P7 differential target.
The paired matrix qualifies Inner/Left/Semi/Anti with duplicate keys,
missing-property SQL NULL, JSON null and left/right/both-empty inputs.
Normal and Analyze result bags have independent P7 expectations; counters
are specified from fixture cardinalities, not inferred from reported actuals.
Plan precedes query execution/publication in each fixture and preserves
observed frontier/file state. Error guards use an explicitly published
generation; lock/shared-memory contents retain the established size-only
comparison. This is observation, not internal operator instrumentation.
Local runtime passes78/78 unique tests across8 targets (focused2 included);
strict core/default all-target Clippy and formatting pass; docs244/0 and
version0.2.10 checks pass. Hosted qualification is pending.
No runtime/planner/API/P6/WAL/hash/schema/ACL model changes. UnionAll,
Aggregate and broad G4/P8/P13, independent runtime/device/release gates remain open.

PR258 head `5e5dd30121bd0fd50324421b4fffcd401e0b19ec` merged as
`e2be296e2e2ae2cf3320b19d24220d096b4ff345` after four successful
exact-head workflows and 15 SUCCESS checks plus one schedule-only skip.
Tests run `39787302554` passed Ubuntu, Windows and macOS. Merged tree equals
tested tree `29f05366e2229f722110c60e51bfeff4b482aeb5`; this equality
is not a post-merge test rerun. Historical pending/NOT_RUN records remain
historical checkpoints, not retroactively executed acceptance.

## Approved G4 composed-query state/identity qualification - 2026-10-22

Owner approved tests/docs-only scope at main `2832ad42` (LOW, C-3).
The fresh read-only review of PR256/257 found no Critical/Important defect
and three Minor evidence improvements: full annotation identity, direct
composed-path state guards and dated hosted-evidence reconciliation.
The focused test target now compares all annotation reference fields and
captures frontier/fixture-file state around composed Plan, annotation denial
and budget-error requests; a separate Plan control runs before any query or
explicit generation publication. Lock/shared-memory bookkeeping uses only
the existing size comparison; no new byte-comparison exclusions are added.
Local156/156 across10 targets and strict core/default all-target Clippy,
format/docs/version/diff gates pass; hosted qualification remains pending.
No runtime/API/P6/WAL/hash/schema/ACL model changes.
Broad G4/P8/P13 and independent runtime/device/release acceptance stay open.

### Current hosted evidence reconciliation

PR256 head `3395e75e408163be7ad961070c1dfcf677db70e8` merged as
`e8f1a58815eac36e45c887f3c73f4d23e3c28d5b`; PR257 head
`c73ec15b35087c6d5540fa97635c89e9dedd11bd` merged as
`2832ad421e1260d57475c330f2bf9cc2e48eab7f`. Each had four successful
exact-head workflows and 15 SUCCESS checks plus one schedule-only skip.
PR257 macOS runner-acquisition cancellations in attempts1/2 were retained;
Tests run `39467118959` succeeded on retry3 before merge. PR256 Tests run
`39448311999` succeeded before merge. Earlier pending/NOT_RUN entries are
historical checkpoints, not current CI failures or retroactive local execution.
This source/evidence review covers only the bounded PR256/257 increment;
it is not whole-G4 independent runtime acceptance.


## Approved G4 annotation-authority composition qualification - 2026-10-21

Owner approved tests/docs-only scope at merged main `e8f1a588` (LOW, C-3).
PR256 passed all four exact-head workflows including Rust on three OSes;
its permissive provenance matrix and namespace-denied controls remain valid.
The next target-local test qualifies enforced Namespace(Read) plus
Annotation(namespace,Read) in HQL/IR lookup and exact KNN/original Rerank ->
parallel-edge Expand -> lookup. Policy revisions grant/remove/regrant the
annotation permission between independent requests, each with a freshly
published pinned generation. Plan stays catalog-only; empty input behavior
is qualified separately from nonempty lookup authorization. No eager ACL,
concurrent revocation, runtime/API/P6/WAL/hash/schema contract is introduced.
Local154/154 across10 targets, strict core/default all-target Clippy,
format/docs244/0/version0.2.10/diff gates pass. Hosted qualification pending;
broad G4/P8/P13, independent/device/release gates remain open.



## Approved G4 provenance/completeness qualification - 2026-10-20

Owner approved tests/docs-only qualification at main `d1bc37e` (LOW, C-3).
The new `hql2_g4_provenance_completeness_tests` target independently checks
full snapshot, four semantics fields, completeness reason/eligible count,
index frontiers and cursor for HQL/IR normal and Analyze requests.
Fixtures cover AnnotationLookup and exact KNN/original Rerank followed by
parallel-edge Expand and AnnotationLookup; entity/revision and score owner
references must retain fixture identity. Empty input is checked separately.
No generic provenance field, truncated-query contract, runtime/API/P6/WAL or
shape-hash change is introduced. Local runtime passes 141 unique tests across eight targets (focused2/2),
with zero failed/ignored; core/default strict all-target Clippy and
format/docs/version/diff checks pass. Hosted qualification is pending; broad
G4/P8/P13 and independent/device/release gates remain open.



## G4 evidence reconciliation - 2026-10-20

Owner-approved documentation-only reconciliation at main `919814f`.
PR #249/#250/#251/#252/#253/#254 are merged bounded qualification slices.
For #254, exact head `7c57bda39be64233ef1dfa8122b0cd47b296eee0`
passed all four triggered PR workflows (Tests `39403874324`, Security
`39403874024`, Performance `39403874196`, worker `39403874412`), with
15 successful checks and one schedule-only advisory skip; Ubuntu, Windows
and macOS Rust suites succeeded before merge `919814f7954659bed2c23a9bdc2777e2d067ddf1`.
Local identity qualification remains 221 unique tests across 18 targets.
Historical NOT_RUN evidence (including #251 local Application Control) is
preserved; later hosted success does not retroactively execute a blocked local run.

The acceptance matrix in the [P8 report](../REPORT--HQL2-P8-CORE-2026-09-28.md#g4-acceptance-evidence-matrix)
separates bounded evidence from missing acceptance. G4/P8 remains PARTIAL;
independent review, device/release and P9-P13 gates are not closed.




# G4 shared query pipeline, deterministic planner and truthful diagnostics

## Approved G4 plan identity/golden qualification - 2026-10-20

Owner approved tests/docs-only qualification on main `24d7eed`; LOW risk, C-3.
Independently specify canonical shape/hash goldens, JSON map-order stability,
parameter values versus shape, controlled node/operator/input/column changes,
and per-request normal/EXPLAIN/ANALYZE parity. Keep the existing shape hash,
planner/runtime/API/P6/WAL/schema contracts. Equal hashes do not imply equal
semantics or authorization. Local qualification passes 221/221 across 18 targets;
strict core/default Clippy, format/docs/version/diff gates pass; hosted passed in #254;
broad G4/P8/P13 and independent/device/release acceptance remain open.

PR #253 exact head `768b56a28678904e505bf0a88e149480a0dd561c` completed
four successful PR workflows (15 SUCCESS checks plus one schedule-only skip),
including Rust Ubuntu/macOS/Windows, before normal merge as
`24d7eeddac7fad3503df5d53c282105ce93f0196`. Its local runtime passed 170 tests
across 14 targets; earlier fixture failures/RCA remain historical evidence.


```text
HQL/IR -> typed bind -> explain shape -> canonical identity -> SHA256
                           |                   |
                           +-> Plan/Analyze ---+-> independent golden assertions
fixture/P7 rows -------------------------------> semantic assertions
```

## Approved G4 lexical/context qualification - 2026-10-20

The owner approved the bounded tests/docs-only proposal on merged main
`7c3e5e2`. Risk LOW, complexity C-3. Qualify LexicalMatch/ContextPack exact rows,
three-mode HQL/IR parity, empty/duplicate inputs, typed unknowns, token/citation
provenance, capability/budget errors and observed-no-write EXPLAIN. Preserve
BM25, tokenizer, runtime/API/P6/WAL/schema and planner contracts. ContextPack
emits one package on empty input; token truncation is a successful package.
Local qualification passes 170/170 across 14 targets; strict core/default
all-target Clippy, format/docs/version/diff gates pass. Hosted qualification
for the lexical/context revision passed in #253. Broad G4/P8/P13,
independent/device/release gates remain open.

PR #252 exact head `25f6e4445ad0ceb11471302e0b5ec40b44d451a9` completed
four successful hosted workflows (15 SUCCESS checks plus one schedule-only
skip), including Rust on Ubuntu/macOS/Windows, then merged normally as
`7c3e5e23984e124a4ba480b8f1e22bf218372b0b`. Its local runtime passed 93 unique
tests across 11 targets. The #251 local NOT_RUN remains historical only.


```text
catalog/profile -> bind/plan -> EXPLAIN (no data execution)
                        |
                        +-> P6 visible sources -> lexical/context kernel -> rows/ANALYZE
fixture + independent P7 rank/context package -----------------------> assertions
```

The bounded matrix counts candidate input/output bags, including duplicates;
it does not report BM25 corpus size or physical scans as measured work. Empty
ContextPack consumes zero rows and emits one package. Source elapsed stays
unknown while lexical/context kernel elapsed is measured without a timing
threshold. Unknown byte/work/memory counters retain units and reasons. Token
truncation retains a successful package; execution-budget failure is an error.
Normal and ANALYZE pair data errors; catalog-only EXPLAIN keeps bind validation.

## Approved G4 history/change diagnostic qualification - 2026-10-20

The owner approved the bounded test/documentation matrix on main `a3e2d4d`.
HistoryScan/ChangeScan keep existing P6/H2-D11 source, temporal/floor/ACL and
budget contracts. Qualify exact emitted rows, empty measured zero, typed unknown
source timing, HQL/IR result parity and normal/EXPLAIN/ANALYZE plan identity.
EXPLAIN remains catalog-only; data-dependent errors are compared only between
normal execution and ANALYZE. No runtime/API/WAL/schema/planner change.
Risk LOW, complexity C-3. Local qualification passes 93 tests across 11 targets;
hosted qualification passed in #252. G4/P8/P13, lexical/context,
independent review and device/release gates remain open.

PR #251 qualified head `098a71e` completed four successful hosted workflows,
15 successful checks and one schedule-only skip before merge as `a3e2d4d`.
Its three OS Rust suites passed; local runtime was NOT_RUN due to Application
Control 4551. The failed parent and fixture-grant RCA remain historical evidence.


```text
catalog -> bind/plan -> EXPLAIN (no data execution)
              |
              +-> P6 lease -> history/change batches -> SourceScan -> rows/ANALYZE
fixture revision log + independent P7 windows ---------------------> assertions
```

## Approved G4 source and annotation diagnostics - 2026-10-20

The owner approved this bounded test/documentation slice at main
`8b18a2c24ac1dfab5bb87036661a80de8973bdca`. Risk LOW, complexity C-3.
PR #249 and #250 each completed four successful hosted PR workflows, with
15 successful checks and one schedule-only conditional skip, before merge.
G4/P8/P13, independent review and device/release acceptance remain open.

Qualify exact source rows for Node/Edge/Row/Annotation, including empty output;
compare ordinary execution, EXPLAIN and ANALYZE with fixture/P7 semantic bags.
Source rows_out describes emitted rows, not physical scan effort. Source
elapsed remains unknown/not_instrumented because materialization precedes
kernel execution. Bytes/work/memory unknowns must never become measured zero.
AnnotationLookup covers required/optional matches and misses, duplicate and
empty bags, existing frozen/live references and explicit actor authority.
Extend EXPLAIN no-effect checks and normal/ANALYZE source/payload/output-budget
errors. The test-only matrix must preserve one P6 lease, typed metadata and
shape identity. No runtime/API/WAL/schema/planner change is authorized.

```text
authorized catalog -> bind/plan -> EXPLAIN (no data execution)
                         |
                         +-> P6 lease -> source batches -> kernels -> rows/ANALYZE
fixture/P7 bag expectations -----------------------------------------> assertions
```

Source/annotation hosted qualification is recorded in the P8 checkpoint at #251.
The history/change extension passes locally; lexical/context and broader frontend
matrices remain separate. No broader acceptance is inferred.

## Approved graph and vector counter qualification — 2026-10-19

The owner approved the next test-only G4 diagnostic slice and explicitly allowed
local work on top of `c61e06d` while PR #249 waits for GitHub Actions. Integration
still follows scalar qualification: merge #249 only after its CI passes, then
base the graph/vector PR on that mainline result. Risk LOW, complexity C-3.

The matrix pins charged graph work for Sequence Expand (parallel paths,
duplicate input bags, optional misses and empty inputs) and root Sequence Match,
including SHORTEST. Manually enumerated fixture steps are the expected counts;
observed counters must not become the expected values. Graph work counts every
charged expansion, including repeated work, rather than unique record IDs.

Exact KNN and Original Rerank must expose exact input/output rows and actual
distance counts for duplicate bags, empty inputs and missing originals under
their existing skip/fail contracts. Executed empty graph/vector kernels report
measured zero for applicable work, distinct from unknown/not_applicable on
other kernels. Ordinary execution, EXPLAIN and ANALYZE must agree on results,
columns, completeness, semantics and plan-shape identity; typed unknown reasons,
source timing provenance and clock metadata remain unchanged.

Graph/candidate/distance budget failures must return errors without successful
partial results in both execution modes. Source/snapshot materialization also
uses query budget before operator execution; its work must not be misattributed
to a per-operator delta or ignored when constructing boundary-budget fixtures.
The initial two-target graph/vector run passes 29/29 tests. Expanded verification
passes 153/153 across twelve explicit targets, including P7 graph/rank and
HQL/IR differentials; strict core/default all-target Clippy also passes.
Seventeen three-mode typed-IR fixtures, six exact-budget successes, twelve
budget errors and two missing-original errors qualify this bounded slice.
No production runtime, API, P6, WAL/schema, migration,
planner/hash or approximation change is authorized by this slice.

```text
existing Storage/P6 materialization -> existing graph/vector kernel
                                    -> ordinary rows / ANALYZE readings
manual fixture work enumeration -----+--> test-only contract comparison
P7/existing result differential -----+
```

## Approved scalar counter qualification — 2026-10-19

The owner approved the bounded G4 scalar and multi-input qualification slice
after the mainline gap review at `6195e85943d519a5080cfda8bf17dda6c695a85f`.
This is verification work under G4-D06/D07, with no new runtime contract.
Risk is LOW; complexity remains C-3 for documentation and architecture review.

The acceptance matrix covers Values, Filter, Project, Distinct, Sort, Offset,
Take, UnionAll, Aggregate and Inner/Left/Semi/Anti Join. Explicit fixture counts
must pin each node's typed `rows_in` and `rows_out`, including duplicate bags,
NULLs, empty inputs, TAKE 0 and multi-input fan-in. Multi-input `rows_in`
counts input rows from each declared input, not predicate comparisons or
distinct identities. Existing independent P7 oracles remain result authorities.

Ordinary execution and ANALYZE must return equal typed rows, columns, semantics
and completeness. EXPLAIN must omit actuals and share the shape-based plan
identity. Non-applicable graph/distance readings and uninstrumented byte/work/
memory readings retain their explicit unknown reasons. Elapsed readings must
use measured nonnegative nanoseconds without wall-time thresholds. Budget
exhaustion in ordinary execution and ANALYZE must return errors without a
successful partial result. Local verification passes 7/7 tests in the new
`hql2_g4_scalar_counter_tests` target and 77/77 across ten explicit regression
targets, with zero failures or ignored cases. Strict all-target Clippy passes
in core and default/NAPI configurations. The P8 checkpoint records commands,
the tested source blob and the bounded architecture review.

```text
explicit v2 request -> existing authorization/binder/planner/P6 lease
                    -> ordinary rows -----+
                    -> ANALYZE + counters +--> fixture counts / P7 results
                    -> EXPLAIN shape -----+    (test-only comparison)
```

The accepted P8 contract defines `plan_hash` as a plan-shape identity, not a
semantic query or result-cache key. This slice preserves it. Broader graph/
vector counter qualification, shared-runtime breadth, independent review and
P8/P13/device/release acceptance remain open. The existing additive NAPI
`queryV2Contract` documented in `API_REFERENCE.md` is implemented separately;
its existence does not close the P13 cross-surface matrix.

## Status and decision

The owner approved this exact G4 contract on 2026-10-04. The approval freezes
the architectural boundary and authorizes bounded source implementation inside
this ADR. It does not authorize public transport exposure, user-database
migration, merge, release or deployment.

The approved decision is one explicit-v2 pipeline for the
parity-proven HQL1 compatibility adapter, HQL2 and typed Query IR v2, with a
deterministic first planner and truthful `EXPLAIN`/`ANALYZE` semantics.
Existing HQL compatibility, `/v1`, and `query-ir.v1` behavior remain stable;
only an explicitly selected and differentially verified HQL1 subset may use
the shared pipeline, and it must not silently fall through to v2.

The current integration candidate combines the P8 typed runtime with the
bounded G4 plan-identity slice. HQL2 text and closed Query IR v2 bind into one
`BoundQueryV2`/`PhysicalPlanV2` path under the serialized Storage/P6 boundary;
the explicitly allowlisted HQL1 adapter lowers into that same path. H2-D11
source adapters preserve revision, retention and ACL checks. Stable contract
and planner metadata plus a canonical plan hash are shared by `EXPLAIN` and
`ANALYZE`.

G4 remains `PARTIAL`, not complete. The integrated tree passed the refreshed
42-target HQL2/REST sweep and strict Clippy in both feature configurations;
full per-operator exact-oracle qualification, independent review, fresh hosted
checks, and device/release qualification remain separate gates.
Missing or skipped evidence remains `NOT_RUN`; public transport and
user-database migration are outside this ADR's implementation scope.

## G4 diagnostics recovery candidate — 2026-10-19

The recovery restores the already approved counter vocabulary under the explicit
v2 core boundary. `ActualCountersV2` uses typed readings containing a unit and
a measured value or an explicit unknown reason (`not_instrumented`,
`not_applicable`, or `overflow`). Checked accounting does not turn overflow
into a saturated measured value. Graph expansions count node and edge expansion
work units charged by the operator, including repeated expansion work.

ANALYZE emits all contracted readings with complete sampling, a monotonic
Instant clock source, and operator-execution elapsed scope. Source scans are
materialized before the execution timer, so their elapsed reading is unknown
with `not_instrumented`; it does not claim to time source I/O. Accounted budget
bytes are not a measured operator peak, and remain unknown in diagnostics.

The diagnostic JSON changes from the preliminary optional numeric fields
(`input_rows`, `output_rows`, `expanded_nodes`, `expanded_edges`, and
`elapsed_micros`) to the ADR's typed `rows_in`, `rows_out`,
`graph_expansions`, and `elapsed_ns` readings. Core v2 diagnostic consumers
must use the typed readings. Plan identity is computed before actual readings;
normal result rows, request versions, planner version and v1 behavior do not
change. No new transport route or user-database migration is introduced.

The initial recovered counter patch passed the fresh 40-target HQL2 sweep
(411 passed, 0 failed, 1 existing ignored case) and 42 library unit tests.
Strict core all-target Clippy passed after equivalent baseline lint cleanup;
subsequent verification is recorded in the PR. The broader G4/P8/P13 and
independent qualification gates remain open; archived branch test totals are
not fresh acceptance evidence.

## Parent and peer evidence

- [HQL2 execution boundary ADR](ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY.md)
  already selects explicit v2 dispatch, one typed binder, a deterministic
  initial planner and truthful `EXPLAIN`; this document supplies the G4
  contract detail that governs implementation.
- [Staged adoption ADR](ADR--GENESISDB-UEE-HQL2-STAGED-ADOPTION.md) keeps the
  current direct-dispatch HQL boundary intact until an explicit superseding
  decision is approved.
- [Typed Query IR agent boundary](ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY.md)
  makes typed IR the machine boundary and keeps HQL as a compatibility
  frontend; arbitrary JSON and SQL are not engine contracts.
- [Typed Query IR v1 specification](../SPEC--GENESISDB-TYPED-QUERY-IR-V1.md)
  remains the v1 compatibility contract. This ADR does not rewrite it.
- [P8 typed boundary](../SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY.md) and
  [H2-D11 durable revision/annotation ADR](ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS.md)
  define the current lease, revision, source and ACL obligations that a G4
  read must preserve.
- [P7 oracle report](../REPORT--HQL2-P7-ORACLE-2026-09-28.md) is the source of
  exact differential semantics. Local oracle evidence is prerequisite
  evidence, not production acceptance.
- [UEE-HQL2 orchestration plan](../IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22.md)
  places G4 at P8 and keeps P9 planner optimization, P10 composition, P13
  surface parity and P16 qualification gated.
- [C4 architecture map](../C4--GENESISDB-ARCHITECTURE.md) is the parent
  architecture index and must remain synchronized with this draft.

Current evidence is bounded local evidence only. Hosted checks, independent
review, mobile/device qualification, release qualification and production
acceptance are not claimed by this ADR.

## Context and problem

The engine currently contains several query frontends and storage-backed
execution slices, but the complete contract from source text or typed IR to a
bound logical representation, physical plan and diagnostic result is not yet
one documented boundary. Direct dispatch and frontend-specific lowering can
therefore drift in type checks, authorization order, temporal binding,
ordering, error codes and result metadata.

G4 must establish a narrow contract that can be implemented and differentially
verified without changing the legacy surface. The contract must also prevent
`EXPLAIN` from reporting guessed access paths or counters, and prevent
`ANALYZE` from returning fabricated actuals.

### Goals

1. Make the parity-proven HQL1 compatibility subset, HQL2 and typed Query IR
   v2 converge on one closed typed binder and one bound logical DAG.
2. Produce a deterministic physical plan before execution, with stable plan
   identity and explicit unsupported capabilities.
3. Define `EXPLAIN` as plan-only and `ANALYZE` as measured, read-only execution.
4. Preserve P6 read leases, H2-D11 revisions, temporal visibility, ACLs,
   budgets, exactness and legacy compatibility.
5. Keep the first implementation small enough to prove semantics before
   indexed shortcuts, cost-based optimization, ANN or parallel execution.

### Non-goals

- Replacing public HQL1, `/v1`, or `query-ir.v1` in this slice. The bounded
  compatibility adapter is not a public cutover.
- Enabling a new REST, NAPI, FFI, SDK, MCP or mobile public surface.
- Adding a storage engine, changing WAL authority, or migrating user databases.
- Implementing cost-based optimization, adaptive plans, ANN shortcuts,
  speculative pushdown or parallel execution.
- Defining a write/mutation execution path. H2-D10 remains separately gated.
- Claiming full UEE-HQL2 Blueprint completion or P13/P16 qualification.

## Contract decisions

### G4-D01 — Explicit version dispatch

The new path is selected by an explicit envelope containing a supported v2
contract identifier, such as `genesis.api.v2` with a closed source selector
for `hql1.compat`, `hql.v2` or `query-ir.v2`. The exact envelope field names
are implementation details to be frozen in the typed API review, but dispatch
must be explicit and closed.

The engine must not infer a version from query text, operator spelling,
unknown fields or a failed v1 attempt. Unsupported or missing versions fail
before data operators or mutations run. A v1 request never silently falls
through to v2.

### G4-D02 — Two frontends, one typed lowering path

HQL1 compatibility parsing, HQL2 parsing and typed Query IR v2 decoding
produce source/typed representations with preserved identifier identity. The
HQL1 compatibility frontend is admitted only for forms with differential
evidence against the legacy behavior. All admitted frontends then enter the
same binder and produce the same logical contract:

```text
HQL1 compatibility text -------> legacy AST --+
HQL2 text ---------------------> source AST --+--> parity gate --+
query-ir.v2 JSON --------------> closed IR ---+                 |
                                                               v
                                                shared binder + auth scope
                                                               |
                                                               v
                                                       BoundLogicalPlan
                                                               |
                                                       deterministic planner
                                                               v
                                                        PhysicalQueryPlan
                                                               |
                                                  one authorized P6 ReadView
                                                               |
                                                    typed execution + result
```

The runtime accepts only closed typed operator configuration. A generic JSON
map, caller-supplied physical plan, or unknown operator field cannot bypass
binding or inject execution behavior.

### G4-D03 — Bound logical plan

The binder validates and resolves, before data execution:

- operator reachability, cycles, ordered input arity and scope;
- closed types, nullability, comparator and arithmetic rules;
- catalog/source identity, collection/vector dimension and registered
  tokenizer dependencies;
- valid-time and transaction-time selectors under one selected frontier;
- namespace, policy, recursive reference and capability authorization;
- query-local limits for nodes, depth, rows, graph work, distance work,
  memory and output;
- supported/unsupported status for every requested operator.

The result is a typed `BoundLogicalPlan` with stable logical node IDs,
resolved source identities, typed literals and explicit limit values. It does
not retain arbitrary transport JSON as an execution protocol.

For HQL1 compatibility, the binder receives a declared adapter capability and
the exact supported form. An unproven HQL1 form remains on its existing
legacy path or fails according to its existing contract; it is not counted as
G4 shared-pipeline coverage.

Authorization and the required query scope are established before parsing or
binding can reveal protected source details. `EXPLAIN` uses the same
authorization boundary and fails closed when the authorized catalog cannot be
resolved.

### G4-D04 — Deterministic first planner

The first planner is a deterministic rule planner. Given the same contract
version, authorized catalog, schema/revision metadata, logical plan and
limits, it must produce the same physical node sequence, stable node IDs and
plan identity independent of JSON object/map insertion order.

The initial rules may select only semantics-preserving operators whose exact
behavior is covered by the P7 oracle and the relevant source capability
contract. The planner must preserve declared bag multiplicity, ordering,
NULL behavior, top-k/filter order, temporal horizon, ACL visibility and
result-completeness metadata.

The first planner does not claim a cost model, index coverage, selectivity,
parallelism or approximation unless the value is measured or supplied by an
approved capability contract. ANN, adaptive execution, speculative rewrites,
parallel readers and cost-based reordering remain later phases.

### G4-D05 — One read authority for execution

Every supported G4 read executes under one authorized P6 `ReadView`/lease and
one selected transaction frontier. Source adapters for Node, Edge, Row,
Vector and Annotation must use the same temporal, retention, revision and ACL
authority. A source that cannot satisfy the bound contract returns an explicit
unsupported or unavailable result; it may not substitute current rows for a
historical request.

The first implementation may retain serialized readers. It must not advertise
concurrent immutable-generation execution until there is implementation and
qualification evidence for that claim.

### G4-D06 — `EXPLAIN` is plan-only

`EXPLAIN` parses, authorizes, binds and plans the request, then returns the
chosen plan and declared capability boundaries. It must not:

- open data scans, graph expansion, vector distance loops or mutation paths;
- build or refresh an index, compact storage, write a WAL event or advance a
  durable frontier;
- return actual execution counters, fabricated zero costs or unverified index
  coverage;
- disclose schema, source or policy details outside the caller's authorized
  catalog.

The output must distinguish at least:

| Field family | Meaning |
|---|---|
| Contract and plan identity | v2 contract, planner version and stable plan hash |
| Logical/physical nodes | typed node kind, stable ID, inputs and declared order |
| Source/capability status | authorized source, supported/unsupported/unavailable state |
| Estimates | estimate only; unknown when no approved estimate exists |
| Exactness metadata | exact, approximate, incomplete or unavailable with reason |
| Warnings/errors | explicit unsupported, budget, policy or binding outcome |

No field named `actual`, `observed`, `rows_read`, `elapsed`, or equivalent may
be populated by an estimate. Unknown values are represented as unknown with a
reason, not as a meaningful zero.

### G4-D07 — `ANALYZE` is measured, read-only execution

`ANALYZE` executes the same bound physical plan as a read query inside one P6
read boundary and returns the normal result metadata plus measured per-node
counters. It must not perform writes, auto-create indexes, compact storage or
return a successful partial aggregate after a resource failure.

Counters are emitted only when measured by the executing operator. The initial
closed counter vocabulary is:

| Counter | Required interpretation |
|---|---|
| `rows_in` / `rows_out` | rows entering and leaving the operator |
| `bytes_read` | bytes actually read, when the source measures them |
| `work_units` | bounded operator work units defined by that operator |
| `index_probes` | actual index probes, not a planned access path |
| `distance_evaluations` | actual vector distance calculations |
| `graph_expansions` | actual graph expansion steps |
| `memory_peak_bytes` | measured peak operator memory, if instrumented |
| `spill_bytes` | actual spill bytes; zero only when measured as zero |
| `elapsed_ns` | measured elapsed time with clock/source metadata |

Unavailable counters are `unknown` with an availability reason. Counter units,
clock source, sampling and overflow behavior must be included in the typed
diagnostic schema; the implementation cannot silently coerce unavailable data
to zero.

### G4-D08 — Fail-closed support and error behavior

Parsing success is not execution support. Unsupported grammar, operator,
selector, source capability, policy combination, mutation or diagnostic mode
must fail with a stable error class before any data effect. A v2 request cannot
fall back to v1 to make an unsupported request appear successful.

The initial error classes are:

| Class | Boundary |
|---|---|
| `invalid_request` | malformed envelope, AST, IR or limit |
| `unauthorized` | missing or insufficient query scope/policy |
| `unsupported` | valid syntax/type but no approved execution capability |
| `snapshot_unavailable` | requested temporal/revision frontier cannot be served |
| `budget_exceeded` | a preflight or measured query budget is exceeded |
| `resource_unavailable` | authorized source or required runtime resource is unavailable |
| `execution_failed` | a verified runtime failure with no safe result |

Exact codes and transport mappings are implementation-review items. They must
remain stable for the approved v2 contract and must not expose hidden source
existence through distinguishable unauthorized errors.

### G4-D09 — Compatibility and persistence boundary

Existing HQL compatibility, `/v1`, NAPI legacy methods and `query-ir.v1` keep
their current routing and semantics during G4. The shared-v2 implementation
may be introduced behind an explicit internal feature/capability gate; public
surface wiring is P13 work.

G4 makes no WAL, snapshot, schema, migration, backup/restore, GBF2/GBO2 or
user-database change. P6 and H2-D11 remain the authority for durable revisions,
leases, temporal visibility and ACL. A rollback removes the v2 registration or
feature gate and leaves the v1 path intact.

### G4-D10 — Bounded HQL1 compatibility adapter

HQL1 is a shared-pipeline frontend only where a legacy-to-v2 differential
proves equal result bags, declared order, errors, authorization behavior and
completeness metadata. The adapter is allowlisted by form and capability; it
does not expand HQL1 grammar or infer a v2 request from query text.

The current HQL1/v1 route remains the compatibility authority until a separate
surface decision authorizes cutover. A v2 envelope may explicitly select the
HQL1 compatibility frontend for an allowlisted form. A failed HQL2 or IR v2
bind never falls back to HQL1, and an unsupported HQL1 form never receives a
fabricated plan or counter result.

## Security and operational boundaries

### Authorization and information disclosure

The caller's authenticated scope and namespace policy are inputs to binding,
not strings supplied by the query. Actor strings from transport payloads do
not become authenticated principals. `EXPLAIN` must not reveal unauthorized
source names, schema fields, index existence, hidden row counts or policy
metadata. Ambiguous visibility fails closed.

### Resource safety

Preflight reserves bounded resources before recursive evaluation. Planner and
diagnostic output are bounded by the same query-local limits as execution.
There is no unbounded plan expansion, dynamic operator loading, arbitrary
expression evaluation or caller-selected physical access path.

### Observability

The implementation should record contract version, query mode, plan hash,
source capability result, exactness/completeness status and counter
availability in structured diagnostics. Query values and protected schema
details must not be logged by default. Observability output is evidence only;
it does not substitute for P7 differential, hosted or production acceptance.

## Alternatives considered

| Alternative | Decision | Reason |
|---|---|---|
| Keep separate HQL and IR planners | Reject | Duplicates binder/security semantics and permits frontend drift. |
| Replace v1 in one cutover | Reject | Breaks compatibility and removes a reversible rollback boundary. |
| Start with cost-based/ANN planning | Reject | Exactness, capability and counter truth are not yet qualified broadly. |
| Add `EXPLAIN` without a shared binder | Reject | A diagnostic view would report a different contract from execution. |
| Allow caller-supplied physical plans | Reject | Expands the trust boundary and bypasses policy/budget controls. |

## Implementation plan and ownership

The serialized plan remains:

| Stage | Scope | Verification gate |
|---|---|---|
| G4-DOC | Approve this ADR and reconcile parent/peer versions | Owner approval and doc validation; completed for this slice |
| G4-RED | Qualify dispatch, binding, deterministic plans, no-effect EXPLAIN, measured ANALYZE and errors | Scalar, graph/vector and source/annotation slices passed hosted #249/#250/#251; history/change passed hosted #252; lexical/context qualification is approved, broader operator/frontend qualification remains open |
| G4-AST/BIND | Implement v2 source AST/IR decoding and shared typed binder | Implemented in the P8 candidate; binder/type/ACL/limit gates and independent review remain |
| G4-PLAN | Implement deterministic physical plan and stable identity | `PhysicalPlanV2` and canonical identity are implemented; bounded golden/identity qualification passes local221/221 across18 targets; hosted passed in #254; broader P7 and independent review remain open |
| G4-EXPLAIN | Plan-only diagnostics and truthful unknown-estimate rules | Node/Edge/Row/Annotation frontier/filesystem checks passed in #251; history/change extension passed hosted #252; lexical/context extension and independent breadth remain open |
| G4-ANALYZE | Read-only measured counters | Scoped scalar, graph/vector and source/annotation matrices passed #249/#250/#251; history/change passed hosted #252; lexical/context passes local 170/170 across 14 targets, hosted passed in #253; lexical/context and broader provenance/budget matrix remain open |
| G4-PARITY | Differential HQL1 compatibility, HQL2 and Query IR v2 for supported operators | P8 exact-oracle tests cover implemented slices; full operator matrix remains open |
| G4-VERIFY | Local gates, evidence reconciliation, independent review and final scope audit | #249/#250/#251/#252/#253 exact-head PR workflows passed before merge; #251 local runtime remains historically NOT_RUN (4551), #252 local runtime passed. Lexical/context passed hosted #253; identity/golden, independent/device/release acceptance remain separate |

File ownership remains serialized: one owner for `src/lib.rs`, one owner for
`src/query/*`, new tests in new files, and no public transport edits until the
P13 boundary is separately opened. Any need to change P6, H2-D11, mutation,
schema or transport contracts returns to documentation review.

## Acceptance criteria

The following are the G4 implementation-completion criteria; this ADR approval
opens the bounded implementation lane but does not claim that every criterion
is already complete:

1. This ADR is owner-approved and the registry, C4 map and orchestration plan
   agree on its status and scope.
2. The parity-proven HQL1 subset, HQL2 and Query IR v2 have one documented
   closed binder and one logical-plan shape; no frontend-specific execution
   shortcut is accepted.
3. Deterministic planning has a stable identity and explicit capability state;
   unsupported rewrites and estimates remain unknown or fail closed.
4. `EXPLAIN` has no data-operator, mutation, index-build, WAL or frontier side
   effect in instrumented tests.
5. `ANALYZE` reports only measured actuals, keeps estimates separate, and
   remains read-only under one P6 boundary.
6. P7 exact-oracle differential tests cover each implemented operator and
   preserve bags, ordering, NULLs, temporal visibility, ACLs, budgets and
   completeness metadata.
7. Existing v1/legacy regression tests remain green and no public transport or
   user-database migration is introduced.
8. Independent review and hosted/device/release gates are recorded separately;
   missing or skipped evidence is `NOT_RUN`, not acceptance.

## Open implementation questions

These questions do not change the architectural boundary, but must be resolved
in the typed API review before broader implementation or surface expansion:

1. The exact closed Rust names for `BoundLogicalPlan`, `PhysicalQueryPlan`,
   diagnostic envelopes and error codes.
2. The later planner-version compatibility policy beyond the current
   `hql2-rule-v1` identity.
3. Which counter fields are instrumented in the first implementation and the
   precise clock/unit metadata for each field.
4. The later P13 mapping of the approved v2 contract to REST/NAPI/FFI/SDK/MCP
   surfaces; no mapping is selected by this ADR.

Any answer that changes version dispatch, the P6 read boundary, compatibility
behavior, persistence, mutation authority or diagnostic truth rules requires
an ADR amendment and renewed review.

## Approval and sign-off

| Role | Decision | Evidence |
|---|---|---|
| Product authority | Approved | User approval `approve`, 2026-10-04 |
| Architecture | Accepted; C-3/HIGH | This ADR plus synchronized parent/peer docs |
| Implementation | Partial / local verification | Plan identity plus recovered typed actual counters, closed JSON tests and fresh focused/root-HQL2 evidence |
| Independent review | NOT_RUN | Must occur after implementation/verification |
| Hosted/device/release | NOT_RUN | Outside this documentation slice |

## Version diff and changelog

Version diff 0.1.29b -> 0.1.30b: record the selected C1b nullable ordering matrix and bounded author evidence; preserve the existing G4/P8 decision and all broad G4/P8/P6/P9-P16 gates. No architecture decision, runtime or contract change.

Version diff `0.1.26b -> 0.1.27b`: record the exact-tree independent source review for the Phase B1 candidate while retaining hosted review, exact-head CI and downstream gates; no runtime, decision or contract change.

Version diff `0.1.25b -> 0.1.26b`: reconcile the Phase B1 scalar type-admissibility candidate on PR271 main, retain the selected scope and downstream gates; no runtime, decision or contract change.

Version diff `0.1.24b -> 0.1.25b`: reconcile the merged PR #269 SourceScan status on current main and point to the finite G4/P8 type/order/composition/provenance ledger; no runtime or contract change.

Version diff: `0.1.3b -> 0.1.4b` selectively recovers the approved typed
diagnostic vocabulary, checked counter arithmetic and truthful source timing
from the archived branch onto current main. The recovery validation above is
separate from historical evidence and broader G4 qualification.

Version diff: `0.1.2b -> 0.1.3b` reconciles the approved G4 plan-identity
slice with the P8 typed runtime and records the integrated 42-target HQL2/REST
and strict-Clippy passes. It preserves explicit-v2 and legacy-surface
boundaries, marks G4 partial, and keeps independent, refreshed hosted, device
and release qualification open.

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.30b | 2026-10-28 | active | Record selected C1b nullable sorting evidence; preserve the G4/P8 contract and open broad gates | working-tree | Codex |
| 0.1.28b | 2026-10-27 | active | Record selected C1 ordering candidate and local broad results; preserve full-key tie and wider gates | working-tree | Codex |
| 0.1.27b | 2026-10-27 | active | Record exact-tree Phase B1 independent source review; formal hosted review and exact-head CI remain pending | working-tree | Codex |
| 0.1.26b | 2026-10-26 | active | Record Phase B1 selected scalar type-admissibility candidate on PR271 main; preserve broad G4/P8/P6/P9-P16 gates | working-tree | Codex |
| 0.1.25b | 2026-10-26 | active | Reconcile merged PR #269 current-main host status and retain broad G4/P8/P6/P9-P16 gates | working-tree | Codex |
| 0.1.10b | 2026-10-20 | active | Approve tests-only G4 identity/golden matrix and reconcile hosted PR253 | working-tree | Codex |
| 0.1.9b | 2026-10-20 | active | Approve test-only lexical/context mode/counter/P7 qualification and reconcile hosted PR252; no runtime contract change | working-tree | Codex |
| 0.1.8b | 2026-10-20 | active | Approve test-only History/Change diagnostic matrix and reconcile PR251 hosted qualification; preserve runtime contracts and broader acceptance gates | working-tree | Codex |
| 0.1.7b | 2026-10-20 | active | Owner-approved source/annotation diagnostics, exact rows and truthful unknowns; reconcile scalar/graph hosted evidence while retaining broader acceptance gates | working-tree | Codex |
| 0.1.6b | 2026-10-19 | active | Approve graph/vector counter qualification after scalar integration, allowing local work during the Actions incident; preserve runtime contracts and broader qualification gates | working-tree | Codex |
| 0.1.5b | 2026-10-19 | active | Owner-approved scalar and multi-input diagnostic qualification; exact fixture counters, P7 result parity and budget-failure matrix; preserve partial G4 and shape-based identity | working-tree | Codex |
| 0.1.4b | 2026-10-19 | active | Recover typed measured/unknown counters, explicit units and provenance, overflow distinction and truthful SourceScan timing; retain broader G4 qualification gates | working-tree | Codex |
| 0.1.3b | 2026-10-06 | active | Reconcile G4 with P8/H2-D11 runtime; integrated HQL2/REST 42-target sweep and strict Clippy in both feature configurations pass; full oracle breadth, independent review and hosted/device/release gates remain open | working-tree | ATHER |
| 0.1.2b | 2026-10-04 | active | Owner-approved G4 contract; implement stable planner metadata and shared plan hash for `EXPLAIN`/`ANALYZE`, with focused local evidence and remaining G4 gates open | working-tree | ATHER |
| 0.1.1b | 2026-10-04 | draft | Add the parity-proven HQL1 compatibility adapter to the explicit-v2 shared pipeline while preserving legacy routing and all implementation gates | working-tree | ATHER |
| 0.1.0b | 2026-10-04 | draft | Propose explicit-v2 shared HQL2/Query IR lowering, deterministic planning, plan-only EXPLAIN, measured read-only ANALYZE, compatibility boundary and gated implementation sequence | working-tree | ATHER |
