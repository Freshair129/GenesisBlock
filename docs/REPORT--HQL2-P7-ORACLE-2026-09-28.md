---
doc_id: REPORT--HQL2-P7-ORACLE-2026-09-28
version: "0.1.9b"
created_at: "2026-09-28T01:38:00+07:00,ATHER,fc851e9"
last_update: "2026-10-04T03:40:00+07:00,ATHER"
status: beta
owner: "GenesisBlockDB Engineering"
attributes:
  domain: verification-evidence
  scope: P7 independent fixture interpreter
---

# HQL2 continuation — P7 evidence and remaining boundary

## Revision and ownership

- Base `fc851e9139041624f3459a7f8cae7e062220c1f8`.
- Isolated branch `codex/hql2-execution`, checkout
  `C:/Users/freshair/.codex/worktrees/hql2-execution/GenesisBlock`.
- Owner approved [execution ADR](adr/ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY.md)
  with “ลุย”. No production source, disk schema, migration or transport changes
  have been made by this P7 implementation.
- The protected primary-checkout probe was not read, changed, staged or run.
  Primary user WIP was not copied into this worktree. The original P7
  test-support checkpoint did not push, merge or deploy; this continuation's
  branch/PR state is recorded in the current P8 checkpoint.
- Late read-only status check found the locally stored `origin/main` ref had
  advanced to `8091a56f2592b8cb28bbae12421cacc9352c2cab`; primary main remains
  `fc851e9`, now ahead 1 / behind 79, with common ancestor `4010702`.
  No fetch/pull/rebase/merge was performed by this task. The incoming history
  includes release/mobile/distribution work; no incoming commits touched
  `src/query`, `src/uee_v2.rs` or this oracle test target. The isolated baseline
  and its local P6 fix were preserved. Integration against the advanced ref is
  a separate unresolved gate, not covered by tests on this pinned base.

## Implemented test support

| Test target | Responsibility |
|---|---|
| `hql2_oracle_tests` | Independent scalar/relational interpreter: NULL, bag multiplicity, joins, aggregate, explicit sort/null placement, exact vector distances, typed equality, bounded literals |
| `hql2_graph_oracle_tests` | Namespace/kind/revision identities, current policy, retained temporal versions, H2-D11 Vector HistoryScan identity/owner ACL, annotation targets, path modes, optional/shortest patterns, selector/source hash validation |
| `hql2_rank_oracle_tests` | Original-vector fidelity, explicit fingerprints, BM25 fixture corpus, candidate-only lineage, source ranks/RRF, registered fixture tokenizer and evidence packing |
| `hql2_pipeline_oracle_tests` | Composes all 23 logical families through the independent domain kernels, retaining bag rows, typed references, scope and score provenance |

Each module can compile with `rustc --test` without linking the production
crate. Expected rows/errors/numeric scores are direct test assertions, not
results captured from the engine. The tiny-graph suite additionally compares
576 bounded cases with a separately expressed Cartesian-product baseline;
relational tests enumerate 100 small join multiplicity cases.

The pipeline uses iterative postorder dispatch and validates logical depth
before executing. Graph work accounting is shared across source, transform,
authorization and corpus calls, including empty-input validation. Errors abort
the whole fixture query; graph quotas never reset for each carried input row.

## Fixed review counterexamples

Measured RED cases and corrective evidence are retained under `.brain/rca/`:

- Negative-dot cancellation could select the wrong row; validating a large
  finite query via its self-distance could reject a valid empty query.
- Nonfinite nested literals and synthetic KNN implementation stages bypassed
  value checks or consumed logical depth incorrectly.
- Annotation/change subjects needed recursive current authorization; future
  or denied annotations must not create observable history dependencies.
- Imported path bindings needed endpoint connectivity checks.
- BM25 corpus statistics needed graph/document policy intersection, not only
  candidate filtering.
- PROJECT needed typed entity-literal propagation and score-origin/current-
  binding separation; SEMI/ANTI could not expose right-side score bindings.
- Missing KNN vectors remove rows, while Rerank still requires originals.
- PACK produces one typed package binding usable by RETURN/PROJECT, not the
  original input rows. Source Match rejects optional mode absent from its wire
  config; optional expansion remains a distinct transform contract.
- A legal-depth chain exposed a test-thread stack overflow; explicit postorder
  evaluation replaces recursive multi-domain dispatch.

## Frozen fixture profile and limits

- Floating expected-score checks use absolute tolerance `1e-12` on the tiny
  declared fixtures. Ranking comparisons/ties do not use this tolerance:
  order is finite distance/score then stable encoded row key.
- Distances use original f64 values; L2 is squared, DOT is negated and uses
  compensated accumulation, cosine scales before norms. Nonfinite products or
  results reject. Compensation is not a claim of arbitrary-precision arithmetic.
- Entity equality includes namespace/kind/id/revision. Undeclared domain
  ordering rejects; path tie ordering is the explicit graph fixture contract.
- Ranking corpus is a single namespace with Node owners; the adapter rejects
  another kind/namespace. BM25 uses pretokenized, fingerprinted fixture tokens,
  not a production Thai tokenizer. Canonical text is not normalized in place.
- Missing text documents are nonmatching for LexicalMatch and absent from its
  corpus; ContextPack requires a document for selected evidence. UNION rejects
  incompatible scalar types or column-wide score provenance instead of labeling
  ordinary numbers as ranked scores.
- Context tokenizer is registered test-only Unicode-scalar counting, including
  citation syntax and separators. It is not an LLM model tokenizer.
- Annotation selectors are Whole/TextPosition; history/change/corrections use
  explicit immutable fixture revisions/events. These do not create durable
  annotation or transactional storage capability.
- Vector HistoryScan uses the H2-D11 compact JSON `(owner_id, collection_id)`
  subject identity and authorizes through its owner Node. P7 still rejects live
  Vector Scan; the storage-backed test profile compares its retained histories.
- The algebra is a typed test representation, not the public JSON wire decoder.
  Scalar support is the declared Bool/I64/F64/UTF-8/vector/list/domain fixture
  profile, not every Blueprint decimal/date/UUID/map/function combination.
- Fixture graph quotas and row limits are not production memory/spill/deadline
  accounting or P12 qualification. No ANN recall or performance claim is made.

Operator-family dispatch and green tests do not by themselves prove complete
Blueprint conformance. Unsupported profile cases must be extended explicitly
before using them as production differential acceptance cases.

## Verification

Prerequisite G0/P6, executed in this worktree:

```powershell
cargo test --locked --offline --no-default-features --test uee_g0_contract_tests --test p6_generation_tests --test p6_lease_tests --test p6_visibility_tests --test p6_peer_authority_tests --target-dir G:\GenesisBlock_Dev\GenesisBlock\target\hql2-execution
```

Result: **36 passed, 0 failed**; build including Windows link took 13m39s.

Combined P7 command:

```powershell
cargo test --locked --offline --no-default-features --test hql2_oracle_tests --test hql2_graph_oracle_tests --test hql2_rank_oracle_tests --test hql2_pipeline_oracle_tests --target-dir G:\GenesisBlock_Dev\GenesisBlock\target\hql2-execution
```

Fresh main-agent standalone rebuild of all four targets passed: 29 relational,
38 graph, 37 rank/context and 25 pipeline tests = **129 passed, 0 failed**.
This command used Rust 2021 std-only test binaries, with no engine linkage.
The original Python reference helpers were also run separately via unittest:
25 passed; they are package evidence, not engine conformance.

Independent review accepted the bounded profile after correction of the
reported defects; the last Count/Collect empty/nonempty regression was rebuilt
and independently rechecked. Combined Cargo completed successfully (exit 0):
**38 graph + 29 relational + 25 pipeline + 37 ranking = 129 passed, 0 failed,
0 ignored**. Build including Windows library linking took 23m26s. This run
included the final Count/Collect regression, not an earlier test snapshot.

Bounded P7 Verify/Review/Final gates pass for the explicitly declared fixture
profile on this base. This does not promote unsupported profile cases or any
production HQL2/Blueprint obligation. Full Rust/Node/mobile/performance/hosted
qualification was not run for this test-support-only slice.

Storage-backed HQL and typed-IR HistoryScan bags for Node, Edge, Row, Vector
and Annotation now compare against P7 catalogs assembled from signed-WAL
revision facts and captured transaction/valid-time windows. The fixtures verify
exact revision identity and temporal intervals, with edge-endpoint, vector-owner
and annotation target/evidence ACL dependencies represented in P7. Artifact
HistoryScan remains unsupported. The P7 oracle implementation itself is
unchanged: its graph target passes 39 tests and combined graph/relational/rank/
pipeline targets pass 130/130. The HistoryScan/ChangeScan target passes 16/16,
the Annotation source target passes 7/7, the prescribed 34-target HQL2
regression passes 386/0/1, and the separate 11-target P6/schema-v6/
compatibility group passes 194/0/0. These remain local test evidence; hosted
checks, broad semantic acceptance and review are outstanding.

Storage-backed HQL and typed-IR ChangeScan bags now compare all five supported
revision subjects (Node, Edge, Row, Vector and Annotation) against explicit P7
events built from signed-WAL mutations and projection transaction stamps. The
new fixture checks seven Edge/Row/Vector/Annotation events, exact sequence,
operation and full subject identity, including edge endpoints, vector owner and
annotation target/evidence dependencies. HQL and typed IR agree; the
History/Change target passes 16/16 and the 33-target HQL2 sweep passes 383/0/1.
This closes the supported subject-kind differential gap only; broader
ChangeScan error/semantic cases and full P8/P13 acceptance remain open.

Storage-backed HQL/typed-IR `Values` plus `UnionAll` match independent P7 for
all 169 pairs from 13 nullable bags (empty and length-one/two bags over NULL,
-1 and 2). Explicit `ORDER BY x ASC NULLS LAST` compares deterministic result
bags while retaining duplicates and NULLs. The focused test passes 1/1 across
338 Storage query executions. This is test-only evidence; broad exact-oracle
coverage and full P8/P13 acceptance remain open.

Storage-backed HQL and typed-IR scalar pipelines also match independent P7 for
six deterministic fixtures across projection/arithmetic/order/offset/take and
filter/project/distinct/order/take. Both frontends are compared independently
to P7 and to each other over 12 Storage executions; the differential target
passes 2/2 including the prior 81-bag scalar test. The HQL2 sweep passes
383/0/1 across 33 targets. This remains bounded test evidence; broad oracle and
full P8/P13 acceptance remain open.

Storage-backed HQL and typed-IR AnnotationLookup bags match independent P7 for
the frozen-target fixture, including an evidence-only nonmatch, optional NULL
extension and duplicate-input multiplicity. The focused AnnotationLookup
target passes 6/6; the explicit HQL2 sweep passes 386/0/1 across 34 targets.
This is test-only differential evidence; the P7 oracle and runtime/contract/
schema/P6/transport behavior are unchanged, and broad P8/P13 acceptance remains
open.

Storage-backed HQL and typed-IR Sequence property constraints now also match
independent P7 for positive exact values: the start/end nodes use UTF-8 `name`
fields and the edge uses an integer `weight`. Wrong-valued parallel-edge and
endpoint candidates are excluded; the focused pattern target passes 8/8 and
the explicit 34-target HQL2 sweep passes 387/0/1. This bounded primitive-scalar
fixture does not claim nested-JSON oracle coverage or expand runtime behavior.

At the original P7 checkpoint, documentation validation reported `0
violations in 233 files`; scoped rustfmt checks of the eight then-new Rust
files and `git diff --check` passed. Current continuation checks are recorded
in the updated P8 checkpoint.

## Remaining work and authority

[P8 concrete boundary](SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY.md) and its
completion addendum have since been owner-approved and partially implemented.
This report records storage-backed test differentials for the already-approved
HistoryScan kinds; it does not expand production capability or imply complete
P8/R1 acceptance.

Full HQL2 completion remains open: the full accepted operator/source matrix,
broad exact-oracle and legacy differential coverage, P13/public-surface parity,
runtime budgets, index lifecycle and P16 qualification. The existing
190-obligation ledger is not promoted by test-only fixtures. Hosted CI,
publication, device validation, soak/crash qualification and deployments have
not been run or claimed.

## Version diff

Version diff `0.1.8b -> 0.1.9b`: add a storage-backed HQL/typed-IR Sequence
node/edge exact-property differential against independent P7 for positive
UTF-8 and integer values, including wrong-value candidate exclusion; focused
pattern target 8/8 and HQL2 387/0/1 across 34 targets. No runtime, contract,
schema, P6 or transport behavior changed; broad oracle and P8/P13 acceptance
remain open.

Version diff `0.1.7b -> 0.1.8b`: add a storage-backed HQL/typed-IR
AnnotationLookup differential against independent P7 for frozen targets,
evidence-only nonmatch, optional NULL extension and duplicate-input
multiplicity; the focused target passes 6/6 and HQL2 passes 386/0/1 across 34
targets. No runtime, contract, schema, P6 or transport behavior changed; broad
oracle and P8/P13 acceptance remain open.

Version diff `0.1.6b -> 0.1.7b`: add six storage-backed scalar-pipeline
fixtures for HQL and typed IR against independent P7, covering projection,
arithmetic, filter, distinct, sort, offset and take in 12 Storage executions.
The focused target passes 2/2 and the HQL2 sweep passes 383/0/1 across 33
targets. No runtime or contract behavior changed; broad oracle and P8/P13
acceptance remain open.

Version diff `0.1.5b -> 0.1.6b`: add the storage-backed HQL/typed-IR
`Values`/`UnionAll` differential against P7 for 169 nullable bag pairs and 338
Storage executions; explicit null-last ordering preserves duplicate/NULL bag
semantics. The HQL2 sweep passes 382/0/1 across 33 targets; the separate P6/
schema-v6/compatibility sweep remains 194/0/0. No runtime, contract, schema or
transport behavior changed; broad oracle and P8/P13 gates remain open.

Version diff `0.1.4b -> 0.1.5b`: extend the storage-backed P7 ChangeScan
differential from Node to Edge, Row, Vector and Annotation revisions. Compare
seven events for exact sequence, operation and subject identity while retaining
recursive endpoint/owner/target/evidence dependencies; HQL and typed IR agree.
Record History/Change 16/16 and HQL2 380/0/1 across 32 targets. Broader
ChangeScan semantics and full P8/P13 remain open.

Version diff `0.1.2b -> 0.1.3b`: record storage-backed P7 HistoryScan
differentials for all five supported kinds from WAL-derived revisions and
captured frontiers; record History/Change 15/15, Annotation source 7/7,
HQL2 379/0/1 and P6/schema-v6/compatibility 194/0/0. P7's own 130/130 result
is unchanged. Artifact HistoryScan and broad P8/P13/hosted/review gates remain
open.

| Document | Before | After |
|---|---|---|
| HQL2 execution ADR | candidate 0.1.0b | owner-accepted 0.1.0b |
| P8 typed boundary | absent | candidate 0.1.1b |
| P7 evidence report | absent | verified beta 0.1.1b |
| Master specification | 2.3.1b | 2.3.2b |
| C4 map | 0.1.13b | 0.1.14b |
| Document registry | 0.4.7+draft | 0.4.8+draft |
| Staged adoption ADR | 0.1.0b | 0.1.1b, historical candidate status retained |
| Orchestration plan | 0.2.1b | 0.2.2b |
| Engine/package | 0.2.6 | unchanged |

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.9b | 2026-10-04 | beta | Add storage-backed HQL/typed-IR Sequence node/edge exact-property P7 differential for positive UTF-8 and integer values with mismatching candidates excluded; focused pattern target 8/8, HQL2 387/0/1 across 34 targets; no runtime/contract/schema/P6/transport change; broad oracle and P8/P13 remain open | working-tree | ATHER |
| 0.1.8b | 2026-10-04 | beta | Add test-only HQL/typed-IR AnnotationLookup differential against independent P7 for frozen targets, evidence-only nonmatch, optional NULL extension and duplicate-input multiplicity; focused target 6/6 and HQL2 386/0/1 across 34 targets; no runtime/contract/schema/P6/transport change; broad oracle and P8/P13 remain open | working-tree | ATHER |
| 0.1.7b | 2026-10-03 | beta | Add test-only HQL/typed-IR scalar pipeline P7 differential for six fixtures and 12 Storage executions; focused target 2/2, HQL2 383/0/1 across 33 targets; no runtime/contract/schema/transport change; broad oracle and P8/P13 remain open | working-tree | ATHER |
| 0.1.6b | 2026-10-03 | beta | Add storage-backed HQL/typed-IR Values/UnionAll P7 differential for 169 nullable bag pairs and 338 Storage executions; HQL2 382/0/1 across 33 targets, P6/compatibility 194/0/0; no runtime/schema/transport change; broad oracle and P8/P13 remain open | working-tree | ATHER |
| 0.1.5b | 2026-10-03 | beta | Extend the HQL/typed-IR P7 ChangeScan differential to Edge/Row/Vector/Annotation; seven events match on sequence, operation and subject identity; History/Change 16/16 and HQL2 380/0/1; retain broad semantic and P8/P13 gates | working-tree | ATHER |
| 0.1.4b | 2026-10-03 | beta | Add the Node-revision ChangeScan HQL/typed-IR differential against P7 for exact sequence, operation and subject revision; History/Change 15/15, HQL2 379/0/1; retain other source-kind and broad P8/P13 gates | working-tree | ATHER |
| 0.1.3b | 2026-10-03 | beta | Record WAL-derived storage-backed P7 HistoryScan differentials for Node/Edge/Row/Vector/Annotation; History/Change 15/15, Annotation source 7/7, HQL2 379/0/1 and P6/compatibility 194/0/0; retain Artifact, broad oracle, hosted and review gates | working-tree | ATHER |
| 0.1.2b | 2026-10-03 | beta | Extend test-only P7 graph reference with approved H2-D11 Vector HistoryScan compact identity and owner-Node ACL; record WAL-derived HQL/typed-IR differential evidence and local sweeps | working-tree | ATHER |
| 0.1.0b | 2026-09-28 | draft | Record implemented P7 profile, measured corrections and remaining P8/R1 gates | working-tree | ATHER |
| 0.1.1b | 2026-09-28 | beta | Record final combined Cargo 129/129 and independent bounded-profile acceptance; P8 approval and upstream integration remain open | working-tree | ATHER |
