---
doc_id: ADR--GENESISDB-HQL2-G4-SHARED-PIPELINE
version: "0.1.2b"
created_at: "2026-10-04T13:08:29+07:00,Codex,2be63a7"
last_update: "2026-10-04T13:55:28+07:00,Codex"
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

# G4 shared query pipeline, deterministic planner and truthful diagnostics

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

The current worktree implements only the first bounded plan-identity slice:
stable planner metadata and a shared plan hash are exposed on `EXPLAIN` and
`ANALYZE`. Full G4 remains `PARTIAL`; the remaining stages and independent,
hosted, device and release gates are not promoted by this slice.

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
| G4-RED | Add tests for dispatch, closed binding, deterministic plans, no-effect `EXPLAIN`, measured `ANALYZE` and fail-closed errors | Focused plan-identity contract test passes; broader G4 coverage remains open |
| G4-AST/BIND | Implement v2 source AST/IR decoding and shared typed binder | Binder/type/ACL/limit tests plus review |
| G4-PLAN | Implement deterministic physical plan and stable identity | Stable identity slice passes locally; golden/P7 breadth remains open |
| G4-EXPLAIN | Implement plan-only diagnostics and unknown-estimate rules | No-read/no-write instrumentation tests |
| G4-ANALYZE | Implement read-only measured counters | Counter provenance and no-partial-result tests |
| G4-PARITY | Differential HQL1 compatibility, HQL2 and Query IR v2 for supported operators | Equal bags, order, errors and completeness |
| G4-VERIFY | Run local gates, reconcile evidence, independent review and final scope audit | PASS/FAIL/NOT_RUN ledger with no promotion of missing gates |

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
| Implementation | Partial / local verification | `ExplainResultV2` plan identity fields, canonical hash and focused contract test |
| Independent review | NOT_RUN | Must occur after implementation/verification |
| Hosted/device/release | NOT_RUN | Outside this documentation slice |

## Version diff and changelog

Version diff: `0.1.1b -> 0.1.2b` records owner approval of the exact G4 contract
and the bounded plan-identity implementation slice. It preserves the
explicit-v2 and legacy-surface boundaries and does not authorize public
transport, migration, merge, release or deployment.

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.2b | 2026-10-04 | active | Owner-approved G4 contract; implement stable planner metadata and shared plan hash for `EXPLAIN`/`ANALYZE`, with focused local evidence and remaining G4 gates open | working-tree | ATHER |
| 0.1.1b | 2026-10-04 | draft | Add the parity-proven HQL1 compatibility adapter to the explicit-v2 shared pipeline while preserving legacy routing and all implementation gates | working-tree | ATHER |
| 0.1.0b | 2026-10-04 | draft | Propose explicit-v2 shared HQL2/Query IR lowering, deterministic planning, plan-only EXPLAIN, measured read-only ANALYZE, compatibility boundary and gated implementation sequence | working-tree | ATHER |
