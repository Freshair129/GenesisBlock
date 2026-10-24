---
title: "GenesisBlockDB Document Registry"
doc_id: "DOC-REGISTRY-GENESISBLOCKDB"
status: draft
version: "0.5.123+draft"
updated: "2026-10-24"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/README.md"
  - "docs/DOC-STATUS.md"
  - "docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md"
  - "docs/FLOW--GENESISRAG17-PIPELINE.md"
  - "docs/GENESISRAG17-EXTENSION-MAP.md"
  - "docs/SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md"
  - "docs/SPEC--GENESISDB-P6-PEER-LOCALITY-ADDENDUM.md"
  - "docs/adr/ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION.md"
---

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

Current selected evidence: [P8 Join result metadata](REPORT--HQL2-P8-CORE-2026-09-28.md#approved-g4p8-join-result-metadata-qualification---2026-10-24).



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

Current selected evidence: [P8 result metadata](REPORT--HQL2-P8-CORE-2026-09-28.md#approved-g4p8-independent-result-metadata-qualification---2026-10-24).



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

Current evidence: [P8 independent host reproduction](REPORT--HQL2-P8-CORE-2026-09-28.md#approved-g4p8-independent-host-reproduction---2026-10-24).

Version transition: `0.5.120+draft` -> `0.5.121+draft`; independently executed host evidence and PR264 reconciliation only.

## Approved G4/P8 broad host qualification - 2026-10-24

Owner approved a six-existing-document evidence consolidation (LOW, C-3),
including existing HQL2/P6/compatibility regressions, one fresh whole-scope
source/evidence/architecture review and normal PR delivery after complete CI.
No test/production/runtime/API/P6/WAL/hash/schema/ACL/stream or surface contract
changes are admitted. Host first; owner reports physical devices unavailable.

Current baseline is PR263 merged main `0a5db778e533152d20fdbd8be7dfcc0e03c4fc4b`
with reviewed source tree `ed2ba0fcf3d802f0c5e2f66a66c3a817c774ee56`.
The P8 report's **Current G4/P8 broad host qualification ledger** maps all eight
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

Canonical ledger: [current P8 acceptance partitions](REPORT--HQL2-P8-CORE-2026-09-28.md#current-g4p8-broad-host-qualification-ledger---2026-10-24).

Version transition: `0.5.119+draft` -> `0.5.120+draft`; broad qualification evidence/navigation only.

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

# GenesisBlockDB Document Registry

## Approved G4 plan identity/golden qualification - 2026-10-20

Owner approved tests/docs-only qualification on main `24d7eed`; LOW risk, C-3.
Independently specify canonical shape/hash goldens, JSON map-order stability,
parameter values versus shape, controlled node/operator/input/column changes,
and per-request normal/EXPLAIN/ANALYZE parity. Keep the existing shape hash,
planner/runtime/API/P6/WAL/schema contracts. Equal hashes do not imply equal
semantics or authorization. Local qualification passes 221/221 across 18 targets;
strict core/default Clippy, format/docs/version/diff gates pass; hosted pending;
broad G4/P8/P13 and independent/device/release acceptance remain open.

PR #253 exact head `768b56a28678904e505bf0a88e149480a0dd561c` completed
four successful PR workflows (15 SUCCESS checks plus one schedule-only skip),
including Rust Ubuntu/macOS/Windows, before normal merge as
`24d7eeddac7fad3503df5d53c282105ce93f0196`. Its local runtime passed 170 tests
across 14 targets; earlier fixture failures/RCA remain historical evidence.


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


## Approved G4 source and annotation diagnostics - 2026-10-20

The owner approved test-only source/annotation diagnostic qualification on
main `8b18a2c`: exact rows, empty input, mode parity, actor authority,
EXPLAIN no-effect and fail-closed budgets under the existing Storage/P6 contract.
See the G4 ADR and P8 checkpoint for the matrix and validation. PR #249/#250
hosted checks completed before merge. Broader G4/P8/P13 and independent,
device/release gates remain open. This slice is LOW risk, C-3.


## 1. Purpose

This registry identifies the active product, requirements, architecture, contract, evidence, and historical entrypoints for GenesisBlockDB.

It does not replace code and test evidence. Product documents define intent and requirements; implementation status must remain traceable to code, tests, benchmarks, audits, and release evidence.

## 2. Registry rules

- One canonical product definition per document role.
- Product requirements must remain independent of GoVibe, NotiKeeper, or another single client.
- Client-specific schemas live in client repositories or adapters.
- `source_of_truth: false` and superseded documents do not compete with active canonical documents.
- Performance claims must point to reproducible evidence.
- A path in this registry must exist at the registered revision.

The registry is an index of canonical entrypoints, not an exhaustive catalog
of every historical, exploratory, benchmark, interview, or ADR document under
`docs/`. Unregistered documents remain reviewable references, but they do not
claim product-definition or implementation-status authority unless a current
registry row explicitly names them.

## 3. Product and requirements

| Role | Doc ID | Version | Status | Owner | Path |
|---|---|---|---|---|---|
| BRD | `BRD-GENESISBLOCKDB` | `0.1.0+draft` | draft | Freshair129 / Product Authority | `docs/BRD--GENESISBLOCKDB.md` |
| PRD | `PRD-GENESISBLOCKDB-PLATFORM` | `0.1.1+draft` | draft | Freshair129 / Product Authority | `docs/PRD--GENESISBLOCKDB-PLATFORM.md` |
| SRS | `SRS-GENESISBLOCKDB` | `0.1.1+draft` | draft | GenesisBlockDB Engineering | `docs/SRS--GENESISBLOCKDB.md` |

## 4. Architecture and contracts

| Role | Doc ID | Version | Status | Owner | Path |
|---|---|---|---|---|---|
| Architecture composition | `MASTER-SPEC-GENESISBLOCKDB` | `2.3.65b` | current | GenesisBlockDB Architecture | `docs/MASTER-SPEC--GENESIS-DB.md` |
| Architecture index | `C4--GENESISDB-ARCHITECTURE` | `0.1.95b` | current | GenesisBlockDB Architecture | `docs/C4--GENESISDB-ARCHITECTURE.md` |
| Execution DAG guard | `SPEC--EXECUTION-DAG-GUARD-V1` | `0.1.0` | accepted | Sol Decision / Contract and AC | `docs/SPEC--EXECUTION-DAG-GUARD-V1.md` |
| HQL2 execution decision | `ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY` | `0.1.1b` | accepted | Boss (Founder / Product Authority) | `docs/adr/ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY.md` |
| HQL2 G4 shared-pipeline contract | `ADR--GENESISDB-HQL2-G4-SHARED-PIPELINE` | `0.1.23b` | active | Boss (Founder / Product Authority) | `docs/adr/ADR--GENESISDB-HQL2-G4-SHARED-PIPELINE.md` |
| HQL2 P8 completion addendum | `ADR--GENESISDB-HQL2-P8-COMPLETION-ADDENDUM` | `0.1.15b` | accepted | Boss (Founder / Product Authority) | `docs/adr/ADR--GENESISDB-HQL2-P8-COMPLETION-ADDENDUM.md` |
| HQL2 Sequence pattern constraints | `ADR--GENESISDB-HQL2-PATTERN-CONSTRAINTS` | `0.2.1b` | beta | Boss (Founder / Product Authority) | `docs/adr/ADR--GENESISDB-HQL2-PATTERN-CONSTRAINTS.md` |
| HQL2 durable revisions and annotations | `ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS` | `0.8.27b` | beta | Boss (Founder / Product Authority) | `docs/adr/ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS.md` |
| UEE-HQL2 orchestration plan | `IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22` | `0.8.98b` | beta | Boss (Founder / Product Authority) | `docs/IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22.md` |
| HQL2 typed P8 boundary | `SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY` | `0.2.67b` | beta | Boss (Founder / Product Authority) | `docs/SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY.md` |
| HQL2 P8 core checkpoint | `REPORT--HQL2-P8-CORE-2026-09-28` | `0.1.90b` | beta | GenesisBlockDB Engineering | `docs/REPORT--HQL2-P8-CORE-2026-09-28.md` |
| HQL2 P7 bounded oracle evidence | `REPORT--HQL2-P7-ORACLE-2026-09-28` | `0.1.12b` | beta | GenesisBlockDB Engineering | `docs/REPORT--HQL2-P7-ORACLE-2026-09-28.md` |
| Commit correctness | `SPEC--WAVE-A-COMMIT-CORRECTNESS` | `0.1.0b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-A-COMMIT-CORRECTNESS.md` |
| Durable collections and edge history | `SPEC--WAVE-B-DURABLE-COLLECTIONS-EDGE-HISTORY` | `0.1.2b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-B-DURABLE-COLLECTIONS-EDGE-HISTORY.md` |
| Generation, lease, temporal and ACL contract | `SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL` | `0.5.47b` | beta | GenesisBlockDB Engineering | `docs/SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md` |
| P6 peer-locality addendum | `SPEC--GENESISDB-P6-PEER-LOCALITY-ADDENDUM` | `0.1.16b` | active | GenesisBlockDB Engineering | `docs/SPEC--GENESISDB-P6-PEER-LOCALITY-ADDENDUM.md` |
| Peer membership and sync authorization | `ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION` | `1.0.0` | accepted | GenesisBlockDB Architecture | `docs/adr/ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION.md` |
| Backup and restore contract | `SPEC--GENESISDB-BACKUP-RESTORE-U9` | `0.1.8b` | beta | Boss (Founder / Product Authority) | `docs/SPEC--GENESISDB-BACKUP-RESTORE-U9.md` |
| Query correctness | `SPEC--WAVE-C-QUERY-CORRECTNESS` | `0.2.1b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-C-QUERY-CORRECTNESS.md` |
| Query budgets and quality gates | `SPEC--WAVE-D-BUDGETS-QUALITY-GATES` | `0.1.0b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-D-BUDGETS-QUALITY-GATES.md` |
| Query context and client capability | `SPEC--WAVE-E-QUERY-CONTEXT-CLIENT-CAPABILITY` | `0.1.2b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-E-QUERY-CONTEXT-CLIENT-CAPABILITY.md` |
| ADR | `ADR-GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE` | `0.1.0+draft` | proposed | GenesisBlockDB Architecture | `docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md` |
| ADR | `ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY` | `1.0.1` | accepted | Product Authority | `docs/adr/ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY.md` |
| Query contract | `SPEC-GENESISDB-TYPED-QUERY-IR-V1` | `1.0.3` | accepted | GenesisBlockDB Architecture | `docs/SPEC--GENESISDB-TYPED-QUERY-IR-V1.md` |
| Client contract | `CONTRACT-CLIENT-NAMESPACE-AND-SCHEMA` | `0.1.0+draft` | draft | GenesisBlockDB Engineering | `docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md` |
| API reference | `API_REFERENCE` | generated | current | GenesisBlockDB Engineering | `docs/API_REFERENCE.md` |
| GenesisRAG17 integration ADR | `ADR-GENESISRAG17-SEPARATE-WORKER-PUBLICATION` | `1.0.8b` | beta | GenesisBlockDB Architecture | `docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md` |
| GenesisRAG17 execution flow | `FLOW-GENESISRAG17-PIPELINE` | `1.0.3b` | beta | GenesisBlockDB Architecture | `docs/FLOW--GENESISRAG17-PIPELINE.md` |
| GenesisRAG17 extension map | `MAP-GENESISRAG17-EXTENSIONS` | `1.0.4b` | beta | GenesisBlockDB Architecture | `docs/GENESISRAG17-EXTENSION-MAP.md` |

## 5. Product narrative and evidence

| Role | Doc ID | Version | Status | Owner | Path |
|---|---|---|---|---|---|
| Positioning | `POSITIONING-GENESISBLOCKDB` | n/a | current | GenesisBlockDB Product | `docs/POSITIONING.md` |
| Whitepaper | `WHITEPAPER-GENESISBLOCKDB-SEMANTIC-SUBSTRATE` | `0.1.0+draft` | draft | GenesisBlockDB Architecture | `docs/WHITEPAPER--GENESISBLOCKDB-SEMANTIC-SUBSTRATE.md` |
| Database whitepaper | `WHITEPAPER--GENESIS-DB` | n/a | current | GenesisBlockDB Architecture | `docs/WHITEPAPER--GENESIS-DB.md` |
| Performance report | `REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE` | n/a | historical | GenesisBlockDB Engineering | `docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md` |
| Product version | `VERSION` | current | current | GenesisBlockDB Engineering | `docs/VERSION.md` |

## 6. Historical and superseded documents

| Role | Doc ID | Version | Status | Replaced by | Path |
|---|---|---|---|---|---|
| Historical terminology whitepaper | `WHITEPAPER-GENESIS-KNOWLEDGE-SYSTEM-HISTORICAL` | `2.1.0-superseded` | superseded | `WHITEPAPER-GENESISBLOCKDB-SEMANTIC-SUBSTRATE` | `docs/WHITEPAPER--GENESIS-KNOWLEDGE-SYSTEM.md` |
| Historical implementation status snapshot | `DOC-STATUS-GENESISBLOCKDB-HISTORICAL` | `2026.06.21+archived` | superseded | `DOC-REGISTRY-GENESISBLOCKDB` plus current code/test evidence | `docs/DOC-STATUS.md` |

## 7. Client boundary

```text
GoVibe domain       NotiKeeper domain       Future client domain
      |                     |                         |
      +-------- client adapters / SDK contracts -----+
                            |
                GenesisBlockDB generic core
```

The registry records GenesisBlockDB product documents only. GoVibe and NotiKeeper application schemas are external client contracts and must not be registered as native GenesisBlockDB ontology.

## 8. Known follow-up documents

The following documents should be created only when implementation work requires them:

- generic node/edge mutation contract;
- WAL durability acknowledgment contract;
- backup/restore and migration contract;
- SDK/interface conformance matrix;
- NotiKeeper adapter conformance report;
- third-client namespace conformance report.

## 9. Changelog

Version diff 0.5.101+draft -> 0.5.102+draft: register the Master Spec journal
authorization preflight budget, fail-closed recovery, and resource-bound contract
for issue #245.

Version diff 0.5.100+draft -> 0.5.101+draft: accept the closed-roster and
authenticated-gossip, role, fixed-quorum, and event-bound vote contracts for
issues #236 and #237; require explicit local authority in multi-node mode and
synchronize Master and C4 references.

Version diff 0.5.99+draft -> 0.5.100+draft: register the source-backed
peer-membership authorization ADR and synchronize Master, C4, and P6 document
versions before policy selection.

Version diff 0.5.98+draft -> 0.5.99+draft: register the approved execution DAG
guard contract and architecture tooling boundary. W1 remains non-dispatchable.

Version diff 0.5.97+draft -> 0.5.98+draft: record passing fresh and committed
Linux worker addon tests on PR head ab38d50 (26/0/6) and all hosted CI workflow
gates; model-dependent paths, migration and full integration qualification
remain pending.

Version diff 0.5.96+draft -> 0.5.97+draft: record the candidate GenesisRAG17
engine upgrade separately from its historical baseline, synchronize parent
architecture references and refresh the Linux addon provenance. The Bookworm
fresh build passed 26/0/6 (six model-dependent skips); the committed-artifact
rerun and full integration qualification remain pending at that revision.

Version diff 0.5.95+draft -> 0.5.96+draft: record the first Bookworm
container run failing before build because checkout's temporary HOME hid the
safe-directory setting from the job shell. Use a command-scoped trust entry for
the exact workspace path; native build and artifact verification remain pending.

Version diff 0.5.94+draft -> 0.5.95+draft: record the committed Linux addon
drift and prepare the workflow to build, test and upload a replacement before
the independent committed-artifact gate. The workflow now locks and fingerprints
its dependency inputs; the artifact refresh and its provenance reconciliation
remain pending until a hosted rebuild is available. Hosted verification is
pending on the updated PR head.

Version diff 0.5.92+draft -> 0.5.93+draft: record current mainline P6
closeout `9821508` in the isolated HQL2 candidate and synchronize the H2-D11,
P8, plan, report and P6 registry entries. Local Windows checks pass P6
49/49, focused integrated targets 44/44, and HQL2 407/0/1 across 40 targets.
Broad P8/P13, shared-runtime, hosted, independent-review and release/transport
qualification remain open.

Version diff 0.5.91+draft -> 0.5.92+draft: reconcile the HQL2 AnnotationScan
temporal/ACL evidence with the P6 mainline closeout. Register H2-D11, P8, P7,
plan, C4, Master, P6 and U9 at their current versions; preserve the annotation
future-valid/post-S controls and separate query-level FORBIDDEN evidence. P8
and broader release/transport qualification remain open.

Version diff 0.5.90+draft -> 0.5.91+draft: register P6 contract v0.5.46b after reintegration on
mainline `4b78596`. Record local Verify on the integrated tree: P6 acceptance 49/49, focused
snapshot/lease/history 35/35, check/format/strict core Clippy/docs (0/241)/diff PASS. Astra's Final remains limited
to the predecessor reviewed slice; it was not rerun on this rebased HEAD, and hosted checks remain
pending. Preserve all D7/P7 records; no P7 or release scope is added.

Version diff 0.5.89+draft -> 0.5.90+draft: reconcile the P6 closeout registry with the D7/P7
mainline history, preserve both version histories without duplicate version IDs, and register
the reconciled P6 contract at v0.5.45b with peer-locality addendum v0.1.15b. No runtime behavior,
release, deployment, or P7 scope is added by this documentation reconciliation.

Version diff 0.5.88+draft -> 0.5.89+draft: register P6 spec v0.5.45b and peer addendum
v0.1.15b. Record Astra 6 Final PASS_WITH_LIMITATIONS for the reviewed local P6 slice and its
accepted stale-Plan fallback fixture limitation. Hosted/cross-platform CI and release/deployment
qualification remain NOT_RUN. No P7, merge, release or deployment.

Version diff 0.5.87+draft -> 0.5.88+draft: register P6 spec v0.5.44b and peer addendum
v0.1.14b. Record current corrected-tree independent Verify PASS (P6 49/49, focused 34/34,
check/format/docs/diff PASS) and broad independent Review PASS, including its bounded stale-Plan
fallback fixture limitation. Astra 6 Final remains NOT_RUN. No P7, merge, release or deployment.

Version diff 0.5.85+draft -> 0.5.86+draft: register P6 spec v0.5.42b and peer addendum
v0.1.12b. Record the checked-increment source correction across public writer, signed-event
validation and snapshot/replay transition paths. The two ACL u64::MAX RED regressions are GREEN;
focused remediation is 34/34, exact P6 is 49/49, and the expanded matrix is 132/132. Fresh Verify
and Review are pending; Astra Final is NOT_RUN. No P7, merge, release or deployment.

Version diff 0.5.84+draft -> 0.5.85+draft: register P6 spec v0.5.41b and peer addendum
v0.1.11b. Record two RED regressions reproducing the ACL u64::MAX saturating-increment defect in
the public writer and signed event chain. Review is FAIL until the checked-increment correction
passes fresh Verify and independent Review; Astra Final remains NOT_RUN. No P7, merge, release or
deployment.

Version diff 0.5.83+draft -> 0.5.84+draft: register P6 spec v0.5.40b and peer addendum
v0.1.10b. Record independent Review P2: saturating ACL revision increment permits same-revision
changed policy at u64::MAX. Update contract and RCA before implementation; Review is FAIL, fresh
Verify/Review and Astra Final are pending. No P7, merge, release or deployment.

Version diff 0.5.82+draft -> 0.5.83+draft: register P6 spec v0.5.39b and peer addendum
v0.1.9b. Record current-tree independent Verify PASS after Plan-only temporal RED/GREEN correction:
P6 acceptance 49/49, focused remediation 32/32, `cargo check`, format, docs validation and diff
check pass. Independent Review and Astra 6 Final remain pending. No P7, merge, release or deployment.

Version diff 0.5.81+draft -> 0.5.82+draft: register P6 spec v0.5.38b and peer addendum
v0.1.8b. Record the Plan-only temporal selector RED/GREEN correction, current local P6 acceptance
49/49, focused remediation 32/32 and expanded 14-target matrix 130/130. Fresh independent Verify
and Review remain pending; Astra 6 Final is NOT_RUN. No P7, merge, release or deployment.

Version diff 0.5.80+draft -> 0.5.81+draft: register P6 spec v0.5.37b and peer addendum
v0.1.7b. Record independent Verify results (49/49 exact P6, 31/31 focused), reconcile the older
45/45 record as historical, and add the Plan-only temporal selector Review finding and RCA. Review
is FAIL pending the regression/source correction; Astra 6 Final is NOT_RUN. No P7, merge, release or
deployment.

Version diff 0.5.78+draft -> 0.5.79+draft: register the two bounded P6 Review
remediations, their RCA records and execution DAG, and the order separating parallel
test-only lanes from serialized `src/lib.rs` changes. Scope prior receipt-authority
PASS_WITH_LIMITATIONS evidence narrowly; broad P6 Review is FAIL and Final is
NOT_RUN pending implementation and gates. No P7, merge, release, or deployment.

Version diff 0.5.77+draft -> 0.5.78+draft: merge the owner-approved G4
shared-pipeline contract and stable plan identity onto the current mainline
while preserving the D7 and P8-R3 evidence. Update the registry rows to the
reconciled C4, P8 boundary, orchestration plan and report versions. The
environment-blocked full REST rerun remains a limitation; no public transport,
migration, release, deploy or independent-review gate is closed by this merge.

Version diff 0.5.77+draft -> 0.5.78+draft: merge the owner-approved G4
shared-pipeline contract and stable plan identity onto current mainline while
preserving the D7 and P8-R3 evidence. Reconcile C4, orchestration plan, P8
report and typed-boundary conflicts; the broader runtime and qualification
gates remain open.

Version diff 0.5.78+draft -> 0.5.79+draft: reconcile the owner-approved G4
contract and stable plan identity with D7's 39-target HQL2 evidence and H2-D11
restore verification. Synchronize C4, P8 boundary/report, orchestration plan,
P8 addendum, revision ADR, P6 and U9 registry versions. G4/P8 remain partial;
fresh hosted checks, independent review and broader qualification remain open.

Version diff 0.5.75+draft -> 0.5.76+draft: synchronize the conditional D7
extension to one exact same-alias string ID predicate on labeled zero-hop
scans after legacy/HQL2 differential evidence for match/miss, wrong-label
exclusion and backslash/Unicode handling. Record adapter 11/11, ordering 2/2,
labeled-filter 2/2, HQL2 394/0/1 across 35 targets and selected P6 peer tests
45/0/0 across seven targets. Update the accepted addendum, P6/P8 evidence,
plan, report, C4 and Master versions. PR #212 baseline has 10 pass, five fail
and one skipped. PR #213 code commit checks report 40 pass, four worker failures
with cause unconfirmed, five skipped and one Windows Cargo cancellation at the
configured 15-minute job limit. No P6 contract/schema/transport behavior changed; broad shared-runtime,
P8/P13 and qualification gates remain open.

Version diff 0.5.74+draft -> 0.5.75+draft: register the test-only HQL/typed-IR
ChangeScan P7 differential for five exclusive-after/inclusive-through windows,
including empty bounds and selected `tx_as_of` frontiers; History/Change 17/17
and HQL2 392/0/1 across 34 targets. Record merged PR #210 hosted status (11
checks pass, four worker markerless-identity failures, one skipped) and update
the report, P6/P8 evidence, plan, C4 and Master versions. No runtime/contract/
schema/P6/transport behavior change; broad P8/P13 and independent-review gates
remain open.

Version diff 0.5.73+draft -> 0.5.74+draft: register the test-only HQL/typed-IR
Node/Edge/Row source-scan differential against P7, the focused 10/10 result,
and the fresh 391/0/1 regression run across 34 root HQL2 targets; update the
P8 report, implementation plan, C4 and Master versions. The prior 393 total
was not reproduced. No runtime/contract/schema/P6/transport change; broad
P8/P13 and independent-review gates remain open.

Version diff 0.5.72+draft -> 0.5.73+draft: synchronize the implementation plan
and HQL2 P8 report with the merged PR #208 hosted result (40 success, 5 failed,
5 skipped); record four worker markerless-identity bootstrap failures and the
Windows annotation query-budget failure whose isolated rerun passed 1/1, with
cause unconfirmed. Keep shared-runtime, P8/P13 and review gates open; no P6
contract change.

Version diff 0.5.71+draft -> 0.5.72+draft: synchronize the approved D7
zero-hop exact string-ID equality extension across the P8 addendum/boundary,
plan, P8 report, C4 and Master; match/no-match legacy/HQL2 differential,
adapter 11/11, ordering 2/2 and HQL2 393/0/1 across 34 targets. Preserve
fail-closed labeled predicates, zero-hop ordering and unproven forms; no P6
grant/lease/schema/migration or transport change. Independent review and broad
P8/P13 gates remain open.

Version diff 0.5.70+draft -> 0.5.71+draft: synchronize the test-only
HQL/typed-IR nested-JSON Sequence property differential against P7 across the
P7 report, P8 checkpoint/addendum, orchestration plan, C4 and Master. Exact
nested node/edge objects and arrays match; near-match candidates are excluded.
Pattern target 11/11 and HQL2 392/0/1 across 34 targets. No
runtime/contract/schema/P6/transport change; broad exact-oracle and P8/P13 gates
remain open.

Version diff 0.5.69+draft -> 0.5.70+draft: synchronize the test-only
HQL/typed-IR optional Sequence edge-property-miss differential against P7
across the P7 report, P8 checkpoint/addendum, orchestration plan, C4 and Master;
preserve four input rows and NULL-extend endpoint/edge aliases. Pattern target
10/10 and HQL2 389/0/1 across 34 targets. No runtime/contract/schema/P6/
transport change; broad exact-oracle and P8/P13 gates remain open.

Version diff 0.5.68+draft -> 0.5.69+draft: synchronize the test-only
HQL/typed-IR Sequence node/edge exact-property differential against independent
P7 across the P7 report, P8 checkpoint/boundary and execution plan; focused
pattern target 8/8 and HQL2 387/0/1 across 34 targets. The fixture proves
primitive UTF-8/integer equality and wrong-value exclusion only. No
runtime/contract/schema/P6/transport change; broad exact-oracle and P8/P13 gates
remain open.

Version diff 0.5.67+draft -> 0.5.68+draft: synchronize the test-only
HQL/typed-IR AnnotationLookup differential against independent P7 across the
P7 report, P8 boundary/checkpoint and execution plan; focused target 6/6 and
HQL2 386/0/1 across 34 targets. Clarify supported Compact base patterns versus
Sequence-only constraints. No runtime/contract/schema/P6/transport change;
broad exact-oracle and P8/P13 gates remain open.

Version diff 0.5.66+draft -> 0.5.67+draft: synchronize the focused D7 ordering-
review coverage closure across the accepted P8 addendum and plan; add source-
alias (`a.id`) DESC differential, bringing positive cases to target
default/ASC/DESC and source DESC. Ordering 2/2, adapter 10/10, HQL2 385/0/1
across 34 targets. Focused review found no semantic/authorization defect; no
runtime/P6 contract/schema/transport change; broader D7/P8/P13 gates remain open.

Version diff 0.5.65+draft -> 0.5.66+draft: synchronize the accepted D7
one-hop projected-endpoint `ORDER BY` extension across P8/P6/plan; record
ordering 2/2, adapter 10/10, HQL2 385/0/1 across 34 targets and selected
P6/schema-v6/ACL tests 93/0/0 across 11. No P6 contract/ACL/lease/schema,
migration or transport change; broader P8/P13 and independent-review gates stay
open.

Version diff 0.5.64+draft -> 0.5.65+draft: synchronize P7/P8/plan/C4/master
records for the test-only HQL/typed-IR scalar-pipeline P7 differential: six
fixtures, 12 Storage executions, 2/2 focused tests and 383/0/1 across 33 HQL2
targets. No runtime/contract/schema/transport change; broad acceptance and
qualification gates remain open.

Version diff 0.5.63+draft -> 0.5.64+draft: synchronize the approved D7
differential extension for one plain-ASCII label on zero-hop HQL1 ID scans;
record adapter 10/10 while HQL2 remains 382/0/1 across 33 targets and P6/
schema-v6/compatibility remains 194/0/0. Update P8/P6/plan/C4/master versions;
no P6 contract/schema/transport change, and shared-runtime/review/broad P8/P13
gates remain open.

Version diff 0.5.62+draft -> 0.5.63+draft: synchronize HQL2/P7/P8/P6/plan,
C4 and master evidence for the test-only HQL/typed-IR `Values`/`UnionAll`
differential against P7: 169 nullable bag pairs and 338 Storage executions.
Record HQL2 382/0/1 across 33 targets and P6/schema-v6/compatibility 194/0/0.
No runtime, contract, schema or transport change. Keep broad exact-oracle,
ChangeScan, P8/P13, independent review and hosted qualification open.

Version diff 0.5.61+draft -> 0.5.62+draft: synchronize P6/P8/plan evidence to
PR #196 head `8ac07f6`, merged at `fb7085a`. The hosted snapshot has 10 passed,
five failed and one skipped check: worker tests fail on Linux/macOS/Windows,
the rebuilt Linux addon/worker fails at markerless database identity, and the
Windows Rust suite fails after 15m16 with its detailed log unavailable. Keep
worker startup and Windows job-time corrections gated, and P8/P13 acceptance
open; no P6 contract or schema change is recorded.

Version diff 0.5.60+draft -> 0.5.61+draft: extend the storage-backed P7
ChangeScan differential to Edge/Row/Vector/Annotation alongside Node; seven
events match exact sequence, operation and subject identity for HQL and typed
IR. Record History/Change 16/16 and HQL2 380/0/1 across 32 targets. Broader
ChangeScan semantics, independent review and full P8/P13 qualification remain
open.

Version diff 0.5.59+draft -> 0.5.60+draft: register the Node-revision
ChangeScan differential against P7, synchronize P7/P8/plan versions, and
record PR #194 merged at `4f02d6b` with 48 checks passing, five failing and
five skipped. History/Change 15/15 and the 32-target HQL2 sweep 379/0/1;
other ChangeScan source kinds, worker correction and broad acceptance remain
open.

Version diff 0.5.58+draft -> 0.5.59+draft: register the WAL-derived P7
HistoryScan differentials for all five supported kinds and synchronize the
P7/P8/P6/plan/report/C4/master version chain. Record History/Change 15/15,
Annotation source 7/7, P7 130/130, HQL2 379/0/1 and P6/schema-v6/compatibility
194/0/0. Artifact HistoryScan, broad ChangeScan/P8/P13, hosted checks and review
remain open.

Version diff 0.5.55+draft -> 0.5.56+draft: synchronize H2-D11, P6, P8,
plan, C4 and master references to the verified annotation ACL conformance
change. The regression passes; HQL2 is 375/0/1 across 32 targets and the
selected P6/schema-v6 suite is 43/0/0. At the prior PR head, worker checks fail
and the Windows Rust job was cancelled; this source patch awaits hosted CI and
review. No contract/schema/migration change; broad P8/P13 gates remain open.

Version diff 0.5.54+draft -> 0.5.55+draft: synchronize P6, C4, master,
H2-D11, orchestration plan and P8 report versions with PR #194 run
37083654705 at docs-only head 43cc6e8. Record four worker bootstrap failures
and a Windows Rust Join budget failure; local Join target passes 5/5, exact
budget dimension remains unconfirmed, and PR remains OPEN/UNSTABLE. Keep
worker correction approval-gated and broad acceptance open.

Version diff 0.5.53+draft -> 0.5.54+draft: synchronize P6, C4, master,
H2-D11, orchestration plan and P8 report versions with the latest HQL2
regression checkpoint (374/0/1 across 32 targets); record PR #194 head
31168524 as OPEN/UNSTABLE with four worker checks failing at fresh schema-v6
bootstrap. Keep worker correction approval-gated and broad acceptance open.

Version diff 0.5.52+draft -> 0.5.53+draft: register HQL/typed-IR Row
HistoryScan/ChangeScan parity and exact row-property hydration (1/1), update
the synchronized H2-D11/P8/plan/report versions, and record the 374/0/1
32-target HQL2 regression sweep. Broad P8/P13, hosted CI and independent
review gates remain open.

Version diff 0.5.51+draft -> 0.5.52+draft: register the implemented HQL2/IR
`tx_as_of` path with one no-fallback frontier across sources, graph/vector/
annotation operators, hydration and `Snapshot.tx`; record focused 56/56,
History/Change 14/14, HQL2 373/0/1 and P6/compatibility 194/0/0. Local
regression is not broad P8/P13 acceptance; hosted CI and independent review
remain open.

Version diff 0.5.50+draft -> 0.5.51+draft: synchronize the accepted P8
transaction-time snapshot contract across P6, H2-D11, P8, report and plan;
record that runtime currently rejects explicit `tx_as_of` and uses the current
frontier, with implementation/verification and hosted CI/review/broad P8/P13
gates still open.

Version diff 0.5.49+draft -> 0.5.50+draft: register the verified H2-D11 Vector
HistoryScan implementation and synchronize H2-D11, P6, P8, checkpoint and plan
versions; record the still-open P8 `tx_as_of`, hosted CI, broader acceptance and
independent-review gates.

Version diff 0.5.47+draft -> 0.5.48+draft: register HQL `JOIN TABLE` lowering
through the approved RowScan/Join contract for Inner/Left/Semi/Anti and bare
JOIN default; HQL/typed IR match independent P7, including duplicate,
missing-property NULL and JSON-null result bags. Record 365/0/1 across 31
HQL2 targets; synchronize P8, checkpoint, plan, Master and C4. Hosted CI for
this local revision is pending; broader P8/P13/review gates remain open.

Version diff 0.5.46+draft -> 0.5.47+draft: register the storage-backed typed-IR
Join P7 differential for Inner/Left/Semi/Anti and 364/0/1 across 30 HQL2
targets; synchronize P8, checkpoint, plan and C4 versions. At parent PR head
484916b core checks pass but worker checks fail on all OSes with
`RECOVERY_REQUIRED: markerless database identity is missing`; no reviews exist,
and the local test is not in CI yet, so PR #194 stays open and unmerged.

Version diff 0.5.45+draft -> 0.5.46+draft: register the storage-backed
HQL/typed-IR aggregate differential against independent P7 over 121 nullable
bags; synchronize P8, checkpoint, plan and C4 versions; record 363/0/1 across
29 HQL2 targets. Hosted Rust/core checks pass on all three OSes, but worker
checks fail across OSes and review remains pending, so merge and broader P8/
P13/review gates remain open.

Version diff 0.5.44+draft -> 0.5.45+draft: register the storage-backed
HQL/typed-IR scalar differential against independent P7 over 81 nullable bags;
synchronize P8, checkpoint, plan and C4 versions; record 362/0/1 across 28
HQL2 targets and 190/0/0 across 11 compatibility targets. Hosted worker checks
still fail; retain merge and broader P8/P13/review gates.

Version diff 0.5.43+draft -> 0.5.44+draft: synchronize final full Rust and
strict Clippy verification across H2-D11, P6, P8, plan and parent documents;
retain `probe_vs_recall` as NOT_RUN and all broader P8/P13/review/release gates.

Version diff 0.5.42+draft -> 0.5.43+draft: record verified H2-D11 R6b
schema-v6 recovery, sync Query IR V1 1.0.3/match_path from upstream, and
update the parent/P6/HQL2 verification records; full P8/P13 gates remain open.

Version diff 0.5.41+draft -> 0.5.42+draft: register owner-approved H2-D11
R6b/P6 schema-v6 activation authority for WAL-only recovery and synchronize
ADR, P6, Master, C4 and HQL2 plan versions. Runtime recovery remains pending.

Version diff 0.5.40+draft -> 0.5.41+draft: register H2-D11 ADR 0.8.6b's
approved AnnotationPut evidence-field clarification; evidence remains distinct
from targets and follows the existing reference ACL path. No schema or
migration change is authorized by this documentation update.

Version diff 0.5.39+draft -> 0.5.40+draft: register D7's differential-proven
one-hop endpoint-ID exact string filter, 9/9 focused adapter tests, 361/0/1
across 27 root HQL2 targets and 190/0/0 across 11 separate compatibility
targets; synchronize canonical HQL2 documents and retain other HQL1,
independent-review, shared-runtime/P8/P13 gates.

Version diff 0.5.35+draft -> 0.5.36+draft: synchronize canonical versions and
register approved P8 D1-D5 implementation, the 9/9 focused target, three D5
literal-size unit tests and 528/0/1 across 37 explicit regression targets.
Retain independent review and broad P8/P13 acceptance gates.

Version diff 0.5.34+draft -> 0.5.35+draft: register owner-approved P8
completion addendum D1-D6 and synchronize the P8, P6, pattern ADR, plan,
master, architecture index and core checkpoint; implementation evidence was
pending at that checkpoint.

Version diff 0.5.33+draft -> 0.5.34+draft: register the implemented
lease-bound HQL/typed-IR Sequence node-ID/label slice, 7 focused passes and
338/0/1 across 25 HQL2 targets; retain ACL-hidden fixture and P8 gates.

Version diff 0.5.32+draft -> 0.5.33+draft: register the delegated HQL2
Sequence node-ID/conjunctive-label contract across P6/P8 and the implementation
plan; runtime implementation and verification remain pending.

Version diff 0.5.31+draft -> 0.5.32+draft: register a passing HQL root-Match
differential against the independent P7 graph bag and 331/0/1 across 24 root
targets; broad exact-oracle and P8 acceptance remain open.

Version diff 0.5.30+draft -> 0.5.31+draft: register HQL/IR exact-KNN and
Original Rerank differentials against the independent P7 oracle and 330/0/1
across 23 root targets; broad exact-oracle and P8 acceptance remain open.

Version diff 0.5.28+draft -> 0.5.29+draft: register the candidate HQL2
Sequence node ID/label contract and P6 dataflow diagram; no source changes are
authorized before owner approval.

Version diff `0.5.27+draft -> 0.5.28+draft`: synchronize checked HQL
remainder/typed-IR `rem` across the P8 contract, plan, checkpoint, architecture
index and master spec; record 79 focused passes and 328/0/1 across 22 HQL2
target tests. Constrained patterns, remaining operators, transport, independent
review and full P8 gates remain open.

| Version | Date | Owner | Summary |
|---|---|---|---|
| 0.5.111+draft | 2026-10-20 | GenesisBlockDB Architecture | Reconcile hosted PR254 and bounded G4 acceptance matrix; keep broad gates partial |
| 0.5.110+draft | 2026-10-20 | GenesisBlockDB Architecture | Approve bounded G4 identity/golden qualification and reconcile hosted PR253; preserve production contracts and broad gates |
| 0.5.109+draft | 2026-10-20 | GenesisBlockDB Architecture | Approve bounded G4 lexical/context diagnostics and reconcile hosted PR252; preserve production contracts and broader gates |
| 0.5.106+draft | 2026-10-19 | GenesisBlockDB Architecture | Synchronize the approved graph/vector counter verification scope across G4, P8, plan, C4 and Master Spec |
| 0.5.105+draft | 2026-10-19 | GenesisBlockDB Architecture | Synchronize approved G4 scalar diagnostic qualification across the ADR, P8 report, plan, C4 and Master Spec; retain partial acceptance |
| 0.5.102+draft | 2026-10-19 | GenesisBlockDB Architecture | Register the `2.3.46b` Master Spec journal authorization preflight budget and fail-closed recovery contract for #245. |
| 0.5.101+draft | 2026-10-16 | GenesisBlockDB Architecture | Accept the closed peer roster, authenticated gossip, role checks, fixed quorum, and event-bound vote contract; require an explicit local role in multi-node mode; synchronize Master and C4 references for #236 and #237 |
| 0.5.100+draft | 2026-10-16 | GenesisBlockDB Architecture | Register candidate peer-membership authorization contract and synchronize Master, C4, and P6 versions; no trust policy approved |
| 0.5.99+draft | 2026-10-10 | GenesisBlockDB Architecture | Register bounded execution DAG guard, independent candidate gates and synthetic pilot contract. |
| 0.5.98+draft | 2026-10-06 | GenesisBlockDB Architecture | Record passing fresh and committed Linux worker addon tests (26/0/6) on PR head ab38d50 and successful tests/security/performance/consumer workflows; six model-dependent tests and migration/full integration qualification remain pending |
| 0.5.97+draft | 2026-10-06 | GenesisBlockDB Architecture | Record the GenesisRAG17 candidate engine source and historical baseline; synchronize ADR/C4/Master Spec/extension map provenance; refresh Linux addon metadata; fresh Bookworm worker suite 26 passed, 0 failed, 6 model-dependent skipped; full committed-artifact and integration qualification pending |
| 0.5.96+draft | 2026-10-06 | GenesisBlockDB Architecture | Record Bookworm job's Git dubious-ownership failure before build; scope safe.directory to the exact checkout path for provenance; artifact rebuild and hosted verification pending |
| 0.5.95+draft | 2026-10-06 | GenesisBlockDB Architecture | Prepare locked Linux addon rebuild/test/upload before the independent committed-artifact gate; record stale-binary finding in the RCA; artifact provenance refresh and hosted verification pending |
| 0.5.94+draft | 2026-10-06 | GenesisBlockDB Architecture | Record the G3 recovery-proof test gap and P7.1 test-only acceptance; synchronize plan 0.8.78b while keeping model/code authorization and broad P7 gates open |
| 0.5.93+draft | 2026-10-06 | GenesisBlockDB Architecture | Record current P6 mainline closeout integration and local results: P6 49/49, focused integrated targets 44/44, HQL2 407/0/1 across 40 targets; synchronize H2-D11/P8/plan/report entries; broader P8/P13, hosted and release gates remain open |
| 0.5.92+draft | 2026-10-06 | GenesisBlockDB Architecture | Reconcile HQL2 AnnotationScan P7 temporal/ACL evidence with the P6 mainline closeout; preserve future-valid/post-S controls and separate query-level FORBIDDEN evidence; P8 and broader release/transport qualification remain open |
| 0.5.83+draft | 2026-10-06 | GenesisBlockDB Architecture | Synchronize controlled AnnotationScan P7 temporal differential and separate HQL/IR Annotation(Read) denial evidence across P7/P8/H2-D11/plan/report; four focused targets pass 42/42; no runtime/P6/schema/transport change |
| 0.5.82+draft | 2026-10-06 | GenesisBlockDB Architecture | Synchronize test-only HQL/typed-IR AnnotationScan P7 differential and current H2-D11/P8/plan/report versions; four focused targets passed 42/42, then source target passed 7/7 after final S,V alignment; no runtime/P6/schema/transport change |
| 0.5.81+draft | 2026-10-06 | GenesisBlockDB Architecture | Reconcile stale H2-D11 History/Change and ChangeScan quota-status text; Artifact History remains unsupported and timing noninterference is unclaimed; no source/schema/migration change |
| 0.5.80+draft | 2026-10-06 | GenesisBlockDB Architecture | Clarify U9 compatibility gates: format, engine name and readable schema are enforced; engine_version remains audit provenance; no source behavior change |
| 0.5.79+draft | 2026-10-06 | GenesisBlockDB Architecture | Reconcile approved G4 stable plan identity with D7/H2-D11 evidence; synchronize C4, P8 boundary/report/addendum, orchestration plan, revision ADR, P6 and U9; broad G4/P8 qualification remains open |
| 0.5.78+draft | 2026-10-06 | GenesisBlockDB Architecture | Integrate approved G4 shared-pipeline contract and stable plan identity with mainline; resolve parent/peer documentation conflicts while preserving open runtime/qualification gates |
| 0.5.77+draft | 2026-10-05 | GenesisBlockDB Architecture | Synchronize approved D7 single-property and exact-seed depth-one TRAVERSE plus mainline R5 restore evidence across P8/P6/U9/plan/C4/Master; adapter 13/13, property/order/filter 2/2 each, HQL2 404/0/1 across 39 targets and U9 restore 7/7; preserve open shared-runtime/P8/P13/review/release gates |
| 0.5.76+draft | 2026-10-04 | GenesisBlockDB Architecture | Synchronize conditional D7 labeled zero-hop exact string-ID filter after legacy/HQL2 match/miss/wrong-label/backslash/Unicode differential; adapter 11/11, ordering 2/2, labeled-filter 2/2, HQL2 394/0/1 across 35 targets, selected P6 peers 45/0/0 across 7; update accepted addendum and P6/P8/plan/report/C4/Master; PR #213 code commit checks 40 pass, 4 worker failures with unconfirmed cause, 5 skipped, 1 Windows Cargo cancellation at 15-minute job limit; no P6 contract/schema/transport change; broad shared-runtime/P8/P13 remain open |
| 0.5.75+draft | 2026-10-04 | GenesisBlockDB Architecture | Register test-only HQL/typed-IR ChangeScan P7 cursor-window differential (five `(after, through]` windows with tx_as_of frontiers), History/Change 17/17 and HQL2 392/0/1 across 34 targets; record PR #210 checks: 11 pass, four worker markerless-identity failures, one skipped; update P6/P8 evidence, report/plan/C4/Master versions; no runtime/contract/schema/P6/transport change; broad P8/P13/review gates remain open |
| 0.5.74+draft | 2026-10-04 | GenesisBlockDB Architecture | Register test-only HQL/typed-IR Node/Edge/Row source-scan P7 differential, focused source target 10/10 and HQL2 391/0/1 across 34 root targets; update P8 report/plan/C4/Master; no runtime/contract/schema/P6/transport change; broad P8/P13/review remains open |
| 0.5.73+draft | 2026-10-04 | GenesisBlockDB Architecture | Synchronize plan/report with merged PR #208 hosted checks (40 success, 5 failure, 5 skipped); four worker markerless-identity failures and one Windows annotation query-budget failure with isolated rerun 1/1, cause unconfirmed; broad shared-runtime/P8/P13/review gates remain open |
| 0.5.72+draft | 2026-10-04 | GenesisBlockDB Architecture | Synchronize D7's differential-proven same-alias exact string ID equality on unlabeled zero-hop scans across P8 addendum/boundary, plan, P8 report, C4 and Master; adapter 11/11, ordering 2/2, HQL2 393/0/1 across 34 targets; preserve fail-closed boundaries and no P6/schema/transport changes; independent review and broad P8/P13 gates remain open |
| 0.5.71+draft | 2026-10-04 | GenesisBlockDB Architecture | Synchronize test-only nested-JSON Sequence property P7 differential across P7/P8/addendum/plan/C4/Master docs; exact HQL/typed-IR nested object/array equality, near-match exclusion; pattern 11/11, HQL2 392/0/1 across 34 targets; no runtime/contract/schema/P6/transport change; broad exact-oracle/P8/P13 gates remain open |
| 0.5.70+draft | 2026-10-04 | GenesisBlockDB Architecture | Synchronize test-only optional Sequence edge-property-miss P7 differential across P7/P8/addendum/plan/C4/Master docs; preserve four input rows and NULL-extend endpoint/edge aliases; pattern target 10/10, HQL2 389/0/1 across 34 targets; no runtime/contract/schema/P6/transport change; broad exact-oracle/P8/P13 gates remain open |
| 0.5.69+draft | 2026-10-04 | GenesisBlockDB Architecture | Synchronize test-only HQL/typed-IR Sequence node/edge exact-property P7 differential across P7/P8/boundary/plan docs; pattern target 8/8, HQL2 387/0/1 across 34 targets; primitive UTF-8/integer only, no runtime/contract/schema/P6/transport change; broad exact-oracle/P8/P13 gates remain open |
| 0.5.68+draft | 2026-10-04 | GenesisBlockDB Architecture | Synchronize test-only AnnotationLookup P7 differential across P7/P8/plan docs; focused target 6/6, HQL2 386/0/1 across 34 targets; clarify Compact base-pattern support vs Sequence-only constraints; no runtime/contract/schema/P6/transport change; broad exact-oracle/P8/P13 gates remain open |
| 0.5.67+draft | 2026-10-04 | GenesisBlockDB Architecture | Synchronize focused D7 ordering-review coverage closure across P8/plan; add source-alias DESC differential, four positive ordering cases, ordering 2/2, adapter 10/10, HQL2 385/0/1 across 34 targets; no runtime/P6 contract/schema/transport change; broader D7/P8/P13 gates remain open |
| 0.5.66+draft | 2026-10-04 | GenesisBlockDB Architecture | Synchronize accepted D7 one-hop projected-endpoint ORDER BY across P8/P6/plan; ordering 2/2, adapter 10/10, HQL2 385/0/1 across 34 targets and selected P6/schema-v6/ACL tests 93/0/0 across 11; no P6 grant/ACL/lease/schema/migration or transport change; broad P8/P13/review remain open |
| 0.5.65+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize P7/P8/plan/C4/master records for test-only HQL/typed-IR scalar-pipeline P7 differential (six fixtures, 12 Storage executions, 2/2 focused, HQL2 383/0/1 across 33 targets); no runtime/contract/schema/transport change; broad gates remain open |
| 0.5.64+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize D7's differential-proven single-label zero-hop HQL1 extension across P8/P6/plan/C4/master; adapter 10/10, HQL2 382/0/1 across 33 targets and P6/compatibility 194/0/0; no P6 contract/schema/transport change; broad gates remain open |
| 0.5.63+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize P7/P8/P6/plan/C4/master evidence for the test-only Values/UnionAll P7 differential (169 nullable bag pairs, 338 Storage executions); HQL2 382/0/1 across 33 targets, P6/compatibility 194/0/0; no runtime/schema/transport change; broad gates remain open |
| 0.5.62+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize P6/P8/plan evidence to merged PR #196; hosted checks 10 pass, five fail, one skipped, including worker bootstrap failures and unverified Windows Rust failure at 15m16; retain approval gates and broad P8/P13 |
| 0.5.61+draft | 2026-10-03 | GenesisBlockDB Architecture | Extend test-only P7 ChangeScan differential to Edge/Row/Vector/Annotation; seven events match sequence, operation and identity for HQL/typed IR; History/Change 16/16 and HQL2 380/0/1; retain broader semantic and full P8/P13 gates |
| 0.5.60+draft | 2026-10-03 | GenesisBlockDB Architecture | Register the Node-revision ChangeScan P7 differential and updated P7/P8/plan versions; record PR #194 merged with 48 pass, five fail and five skipped; retain other source-kind and broad HQL2 acceptance gates |
| 0.5.59+draft | 2026-10-03 | GenesisBlockDB Architecture | Register WAL-derived independent P7 HistoryScan differentials for Node/Edge/Row/Vector/Annotation and synchronize HQL2/P7/P8/P6/plan/C4/master versions; History/Change 15/15, Annotation source 7/7, P7 130/130, HQL2 379/0/1, P6/schema-v6/compatibility 194/0/0; Artifact and broad acceptance/hosted review remain open |
| 0.5.58+draft | 2026-10-03 | GenesisBlockDB Architecture | Register the WAL-derived independent P7 Vector HistoryScan differential and synchronize P7/P8/P6/plan/report/C4/master versions; P7 130/130, History/Change 14/14, HQL2 377/0/1, P6/schema-v6/compatibility 194/0/0; prior PR head has five failed checks and no reviews, new checks pending; broad P8/P13 open |
| 0.5.56+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize HQL2/P6/plan/report/C4/master references for explicit Annotation(Read) on annotation source and ChangeScan subjects; HQL2 375/0/1 and selected P6/schema-v6 43/0/0 pass; possible ChangeScan budget side channel, prior PR worker failures and cancelled Windows Rust remain open |
| 0.5.55+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize HQL2/P6/plan/report references and PR #194 run 37083654705: four worker failures plus one Windows Rust Join budget failure; PR OPEN/UNSTABLE, no implementation change, broad P8/P13/review gates remain open |
| 0.5.54+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize HQL2/P6/plan/report references to current versions and 374/0/1 across 32 targets; record PR #194 as OPEN/UNSTABLE with four worker-CI failures; worker correction remains approval-gated, broad P8/P13/review gates open |
| 0.5.53+draft | 2026-10-03 | GenesisBlockDB Architecture | Register HQL/typed-IR Row HistoryScan/ChangeScan parity, exact row-property hydration (1/1), and HQL2 regression 374/0/1 across 32 targets; hosted CI, independent review and broad P8/P13 gates remain open |
| 0.5.52+draft | 2026-10-03 | GenesisBlockDB Architecture | Register implemented HQL2/IR `tx_as_of` with one no-fallback frontier; focused 56/56, History/Change 14/14, HQL2 373/0/1 and P6/compatibility 194/0/0; hosted CI, review and broad P8/P13 gates remain open |
| 0.5.51+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize accepted HQL2 `tx_as_of` contract and record runtime rejection/current-frontier gap; runtime acceptance, hosted CI/review and broader P8/P13 gates remain open |
| 0.5.50+draft | 2026-10-03 | GenesisBlockDB Architecture | Register and synchronize verified Vector HistoryScan HQL/IR parity, P6 vector floor and owner ACL; HQL2 367/0/1 and P6/compatibility 194/0/0; keep tx_as_of, hosted CI, review and broader P8/P13 gates open |
| 0.5.49+draft | 2026-10-03 | GenesisBlockDB Architecture | Register approved H2-D11 Vector HistoryScan identity, floor and owner ACL; synchronize P6/P8/plan versions, with runtime implementation and parity verification pending |
| 0.5.48+draft | 2026-10-03 | GenesisBlockDB Architecture | Implement HQL JOIN TABLE lowering through the approved RowScan/Join contract for four kinds and bare default; HQL/typed IR match independent P7 for duplicate, missing-property NULL and JSON-null result bags; record 365/0/1 across 31 targets; hosted CI pending, P8/P13/review gates remain open |
| 0.5.47+draft | 2026-10-03 | GenesisBlockDB Architecture | Register storage-backed typed-IR Join P7 differential for all four join kinds and 364/0/1 across 30 HQL2 targets without expanding HQL JOIN; parent head 484916b core checks pass but worker checks fail and no PR review exists; PR remains unmerged |
| 0.5.46+draft | 2026-10-03 | GenesisBlockDB Architecture | Register storage-backed HQL/typed-IR P7 aggregate differential over 121 nullable bags; synchronize canonical HQL2/plan/C4 versions and 363/0/1 across 29 targets; hosted Rust/core checks pass on Linux/macOS/Windows, worker checks fail across OSes and review remains pending. |
| 0.5.45+draft | 2026-10-03 | GenesisBlockDB Architecture | Register storage-backed HQL/typed-IR P7 scalar differential over 81 nullable bags; synchronize canonical HQL2/plan/C4 versions and 362/0/1 plus 190/0/0 regression results; latest worker CI and Windows Rust checks fail, broader merge/P8/P13/review gates remain open. |
| 0.5.44+draft | 2026-10-02 | GenesisBlockDB Architecture | Synchronize full locked/offline Rust suite and both strict Clippy results across current HQL2/H2-D11/P6 documents; `probe_vs_recall` NOT_RUN and broader qualification gates remain open. |
| 0.5.43+draft | 2026-10-02 | GenesisBlockDB Architecture | Record verified R6b fixture recovery, Query IR V1 1.0.3/match_path integration and 40 passing HQL2/durability/authority targets; full P8/P13 qualification remains open. |
| 0.5.42+draft | 2026-10-02 | GenesisBlockDB Architecture | Register owner-approved H2-D11 R6b schema-v6 WAL-only recovery contract and synchronized parent/P6/plan docs; implementation pending. |
| 0.5.41+draft | 2026-10-02 | GenesisBlockDB Architecture | Register owner-approved H2-D11 AnnotationPut evidence-field clarification and ADR 0.8.6b; separate evidence references remain under existing ACL behavior. |
| 0.5.40+draft | 2026-10-02 | GenesisBlockDB Architecture | Register D7's differential-proven one-hop endpoint-ID exact string filter, 9/9 focused tests, 361/0/1 HQL2 and 190/0/0 compatibility regression results; retain shared-runtime, independent review and P8/P13 gates. |
| 0.5.39+draft | 2026-10-02 | GenesisBlockDB Architecture | Register D7's differential-proven one-hop HQL1 extension and parser preflight, 8/8 focused tests, 361/0/1 HQL2 and 190/0/0 compatibility regression results; retain shared-runtime, independent review and P8/P13 gates. |
| 0.5.38+draft | 2026-10-02 | GenesisBlockDB Architecture | Register D7's actor-scoped HQL1 projection, 5/5 adapter tests and 354/0/1 across 27 HQL2 targets; synchronize canonical documents and retain shared-runtime/P8/P13 gates. |
| 0.5.37+draft | 2026-10-02 | GenesisBlockDB Architecture | Record the D4 edge-property-before-SHORTEST regression, 10/10 P8 completion tests and 349/0/1 across 26 HQL2 targets plus separate P6/schema-v6/compatibility evidence; retain broad P8/P13 gates. |
| 0.5.36+draft | 2026-10-02 | GenesisBlockDB Architecture | Synchronize canonical HQL2 doc versions; register D1-D5 implementation, focused/budget/literal-size tests, and 528/0/1 across 37 selected targets; retain independent review and broad P8/P13 gates. |
| 0.5.35+draft | 2026-10-02 | GenesisBlockDB Architecture | Register accepted P8 D1-D6 decisions and synchronized contracts; no implementation or P8-completion evidence inferred. |
| 0.5.34+draft | 2026-09-30 | GenesisBlockDB Architecture | Register implemented lease-bound HQL/typed-IR Sequence node-ID/label slice, 7 focused passes and 338/0/1 across 25 HQL2 targets; retain ACL-hidden fixture and broader P8 gates. |
| 0.5.33+draft | 2026-09-30 | GenesisBlockDB Architecture | Register delegated HQL/typed-IR Sequence node-ID/label contract under P6; runtime implementation pending, with properties/Compact constraints fail-closed. |
| 0.5.32+draft | 2026-09-30 | GenesisBlockDB Architecture | Register passing HQL root-Match differential against independent P7 graph bag and 331/0/1 across 24 root HQL2 targets; retain broad oracle and P8 gates. |
| 0.5.31+draft | 2026-09-30 | GenesisBlockDB Architecture | Register passing HQL/IR exact-KNN and Original Rerank differentials against independent P7 ranking and 330/0/1 across 23 root HQL2 targets; retain broad oracle and P8 gates. |
| 0.5.30+draft | 2026-09-30 | GenesisBlockDB Architecture | Register passing HQL/IR exact-KNN differential against the independent P7 oracle and 329/0/1 across 23 root HQL2 targets; retain broad oracle and P8 gates. |
| 0.5.29+draft | 2026-09-30 | GenesisBlockDB Architecture | Register candidate lease-bound HQL/IR Sequence node ID/label semantics and the approval gate; implementation remains unauthorized pending owner approval. |
| 0.5.28+draft | 2026-09-30 | GenesisBlockDB Architecture | Register checked HQL `%`/typed-IR `rem`, 79 focused passes and 328/0/1 across 22 HQL2 targets; retain remaining HQL2/P8 gates. |
| 0.5.27+draft | 2026-09-30 | GenesisBlockDB Architecture | Register HQL2 NULLS LAST default and exact DecimalU64 structural parameters; record 58 focused passes and 325/0/1 across 22 targets; retain remaining HQL2/P8 gates. |
| 0.5.26+draft | 2026-09-30 | GenesisBlockDB Architecture | Register exact typed DecimalU64/no-cast structural HQL parameters and 323/0/1 across 22 HQL2 targets; retain remaining HQL2/P8 gates. |
| 0.5.25+draft | 2026-09-30 | GenesisBlockDB Architecture | Register exact typed-IR root Match anchors and 319/0/1 across 22 HQL2 targets; retain remaining HQL2 and P8 gates. |
| 0.5.24+draft | 2026-09-30 | GenesisBlockDB Architecture | Synchronize P6 spec and P8 report to HistoryScan/ChangeScan runtime and 9/9 focused tests; retain remaining operator and P8 gates. |
| 0.5.23+draft | 2026-09-30 | GenesisBlockDB Architecture | Register lease-bound HistoryScan/ChangeScan, recursive reference ACL, 9/9 focused tests and 318/0/1 across 22 HQL2 targets; retain remaining operator and P8 gates. |
| 0.5.22+draft | 2026-09-29 | GenesisBlockDB Architecture | Register exact HQL/IR KNN and Original Rerank on lease-bound original vectors; synchronize H2-D11/P6/P8 and parent documents; record 307/0/1 across 21 root HQL2 targets and retain remaining operator/P8 gates. |
| 0.5.21+draft | 2026-09-29 | GenesisBlockDB Architecture | Record the RowScan after_image correction and then-current 296/3/1 HQL2 sweep; synchronize plan/spec/report/C4 and retain remaining operator/P8 gates. |
| 0.5.20+draft | 2026-09-29 | GenesisBlockDB Architecture | Register focused RowScan property-hydration RED and RCA, synchronize HQL2 status docs, and retain the specific runtime approval gate. |
| 0.5.19+draft | 2026-09-29 | GenesisBlockDB Architecture | Register the explicit 21-target HQL2 sweep; retain three KNN/Rerank failures, distinguish oracle evidence from P8 acceptance and synchronize report/plan versions. |
| 0.5.18+draft | 2026-09-29 | GenesisBlockDB Architecture | Register 26/26 Blueprint semantic-oracle passes for separate annotation evidence; synchronize HQL2 plan/report versions and retain operator/P8 gates. |
| 0.5.17+draft | 2026-09-29 | GenesisBlockDB Architecture | Record 62/62 separate H2-D11 fixture tests and synchronize orchestration plan/report versions; retain HQL2 operator and P8 gates. |
| 0.5.16+draft | 2026-09-29 | GenesisBlockDB Architecture | Register structural HQL/IR root Match with deterministic SHORTEST; retain anchors, constrained patterns, six unavailable operators, vector/P8 and release gates. |
| 0.5.15+draft | 2026-09-29 | GenesisBlockDB Architecture | Register typed IR Vector parameter decoding with space/dimension checks and two focused passing tests; retain HQL inference, original-vector reads, Knn/Rerank, P8 and release gates. |
| 0.5.14+draft | 2026-09-29 | GenesisBlockDB Architecture | Register structural HQL/IR Sequence Expand; update HQL2 checkpoint to 206 passing tests across 21 targets, with pattern-constraint/P8/release gates open. |
| 0.5.13+draft | 2026-09-29 | GenesisBlockDB Architecture | Register constrained HQL/IR Expand and separate AnnotationPut evidence fixture/oracle; update HQL2 checkpoint to 199 passing tests across 20 targets, with P8/release gates open. |
| 0.5.12+draft | 2026-09-29 | GenesisBlockDB Architecture | Register HQL/IR AnnotationLookup semantics and 194 passing P8/P6/H2-D11 tests across 19 targets; keep P8 and release gates open. |
| 0.5.11+draft | 2026-09-29 | GenesisBlockDB Architecture | Register typed selective HQL2 hydration and 188 passing P8/P6 tests across 18 targets; keep remaining operators, transport and P8 partial. |
| 0.5.10+draft | 2026-09-29 | GenesisBlockDB Architecture | Register four revision-bound source scans, annotation reference ACL, payload-budget precharge and 133/133 selected P8/P6 tests; keep P8 partial. |
| 0.5.9+draft | 2026-09-29 | GenesisBlockDB Architecture | Register annotation CAS-write, normalized target/evidence projection, P6 policy-event v2 and 8/8 tests; keep annotation read ACL and P8 source adapters open. |
| 0.5.8+draft | 2026-09-29 | GenesisBlockDB Architecture | Register durable vector revision write/projection/reopen and fail-closed ingress evidence; annotation/ACL, source adapters, user DB migration and release gates remain open. |
| 0.5.7+draft | 2026-09-29 | GenesisBlockDB Architecture | Register 19/19 fixture migration/recovery evidence and H2-D11/P6/P8/plan/C4 checkpoint versions; user DB migration and release qualification remain open. |
| 0.5.6+draft | 2026-09-29 | GenesisBlockDB Architecture | Record owner approval of amended H2-D11 ADR 0.4.0b and fixture-only migration implementation authority; synchronize ADR, P6, plan and C4 statuses. |
| 0.5.5+draft | 2026-09-28 | GenesisBlockDB Architecture | Register amended H2-D11 recovery, fold/cold-reopen, reader-compatibility, peer-isolation and receipt-proof gates; ADR 0.4.0b awaits owner approval. |
| 0.5.4+draft | 2026-09-28 | GenesisBlockDB Architecture | Register owner-approved migration envelope/source-coordinate candidate and pending implementation gate across ADR, P6, plan and C4. |
| 0.5.3+draft | 2026-09-28 | GenesisBlockDB Architecture | Register fixture-backed relational row revision and registry persistence evidence; retain vector, annotation, adapter, migration and folded-peer gaps. |
| 0.5.2+draft | 2026-09-28 | GenesisBlockDB Architecture | Register current H2-D11 graph revision implementation evidence and updated plan/ADR versions while keeping remaining scope explicit. |
| 0.5.1+draft | 2026-09-28 | GenesisBlockDB Architecture | Correct H2-D11 implementation status and register updated ADR/plan and focused KeyCodec evidence without implying migration completion. |
| 0.5.0+draft | 2026-09-28 | GenesisBlockDB Architecture | Register owner-approved H2-D11 and synchronize its approved P8/P6 contracts and staged implementation plan. |
| 0.4.8+draft | 2026-09-29 | GenesisBlockDB Architecture | Registered Typed Query IR V1 1.0.3 and C4 0.1.14b after implementing the typed linear match_path slice. |
| 0.4.7+draft | 2026-09-23 | GenesisBlockDB Architecture | Sync P6 spec version after snapshot-authority and HQL ACL review corrections. |
| 0.4.6+draft | 2026-09-23 | GenesisBlockDB Architecture | Classify Enforced-ACL legacy accessors and refresh the P6 specification version. |
| 0.4.5+draft | 2026-09-22 | GenesisBlockDB Architecture | Registered the owner-approved P6 contract and synchronized Master/C4 document versions. |
| 0.4.4+draft | 2026-09-22 | GenesisBlockDB Architecture | Registered the approved Wave E query context and client capability implementation record. |
| 0.4.3+draft | 2026-09-10 | GenesisBlockDB Architecture | Truth-synced the Wave D beta status and Typed Query IR 1.0.2 registry entries after release verification. |
| 0.4.2+draft | 2026-09-08 | RWANG | Reconcile GenesisRAG17 document versions 1.0.3b after zuri's pre-merge ADR-071 to ADR-073 collision repair; preserve historical rows. |
| 0.4.1+draft | 2026-09-08 | GenesisBlockDB Architecture | Reconciled the GenesisRAG17 registry entries with the live zuri ADR-071 reference and retained historical report links. |
| 0.4.0+draft | 2026-09-08 | GenesisBlockDB Architecture | Registered the GenesisRAG17 TEST worker/publication ADR, execution flow and extension map. |
| 0.3.9+draft | 2026-09-08 | GenesisBlockDB Engineering | Registered the Wave D candidate for query budgets, REST execution control and per-index quality evidence. |
| 0.3.8+draft | 2026-09-08 | GenesisBlockDB Engineering | Recorded large-collection filtered-ANN fix `d8ef9af`, 28-cell oracle evidence and rebuilt N-API/MCP runtime pass. |
| 0.3.7+draft | 2026-09-08 | GenesisBlockDB Engineering | Recorded approved Wave C implementation checkpoint `cbe5a04` and beta exit evidence. |
| 0.3.6+draft | 2026-09-08 | GenesisBlockDB Engineering | Registered the Wave C query-correctness candidate packet. |
| 0.3.5+draft | 2026-09-08 | GenesisBlockDB Engineering | Registered the B4 identity-preserving replay benchmark correction. |
| 0.3.4+draft | 2026-09-08 | GenesisBlockDB Engineering | Recorded Wave B approval, implementation and synchronized parent versions. |
| 0.3.3+draft | 2026-09-08 | GenesisBlockDB Engineering | Registered Wave B candidate design; implementation remains unapproved. |
| 0.3.2+draft | 2026-09-08 | GenesisBlockDB Architecture | Registered approved Wave A commit contract and synchronized parent architecture versions. |
| 0.3.0+draft | 2026-08-14 | GenesisBlockDB Architecture | Registered the accepted Typed Query IR ADR/spec and removed it from the follow-up list. |
| 0.3.1+draft | 2026-08-14 | GenesisBlockDB Architecture | Truth-synced Query IR spec, ADR and C4 versions after the partial search/traverse implementation. |
| 0.2.0+draft | 2026-08-13 | GenesisBlockDB Architecture | Reconciled registered frontmatter and defined canonical-entrypoint scope for automated validation. |
| 0.1.0+draft | 2026-08-03 | GenesisBlockDB Architecture | Established the active standalone-product registry and separated historical status snapshots. |
