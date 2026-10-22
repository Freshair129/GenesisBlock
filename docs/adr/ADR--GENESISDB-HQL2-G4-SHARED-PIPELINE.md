---
doc_id: ADR--GENESISDB-HQL2-G4-SHARED-PIPELINE
version: "0.1.15b"
created_at: "2026-10-04T13:08:29+07:00,Codex,2be63a7"
last_update: "2026-10-23T00:00:00+07:00,Codex"
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
