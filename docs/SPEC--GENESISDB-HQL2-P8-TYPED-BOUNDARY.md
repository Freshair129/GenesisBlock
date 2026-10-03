---
doc_id: SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY
version: "0.2.55b"
created_at: "2026-09-28T01:25:00+07:00,ATHER,fc851e9"
last_update: "2026-10-03T20:45:00+07:00,ATHER"
status: beta
superseded_by: null
owner: "Boss (Founder / Product Authority)"
attributes:
  doc_type: specification
  domain: query-execution
  scope: P8 typed frontend and core-only query boundary
  risk: HIGH
  complexity: C-3
---

# P8 typed boundary — approved concrete contract

## Status

The owner approved the architecture in
[HQL2 execution ADR](adr/ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY.md).
The owner approved version 0.1.1b with "approve" on 2026-09-28, including
integration of upstream into the isolated HQL2 worktree while preserving P6/P7.
The owner approved the concrete H2-D11 durable-revision and annotation contract
on 2026-09-28. This 0.2.0b amendment requires database-bound record references,
UUIDv4 durable revisions, and the approved source-history/annotation gates.
This approved contract freezes the exact interface/config/result decisions that the
ADR requires before P8 source changes. The HQL2 parser, typed scalar binder/runtime,
catalog ReadView and H2-D11 revision writers have local implementation evidence.
Node, edge, registered-row and annotation scans now return database-bound revision
references from one P6 lease, with bounded keyset pages, source filters and
catalog-bound row tables. Annotation scans check normalized target/evidence
references; `prop`/`has_prop` use binder-issued `FieldIdV2` and aligned
`ExecBatchV2` batches under the same lease. Runtime property names remain
supported; hydration is performed per operator, and query rows retain only
revision-bound entities plus selected typed values, not whole JSON payloads.
Annotation target/evidence access is rechecked before body hydration, and
`LEFT JOIN` identity/property values preserve nullable behavior. AnnotationLookup
now works from both HQL and typed IR: frozen targets match the exact revision,
live targets resolve at the pinned S,V, evidence refs do not produce lookup
matches, and optional/required forms preserve their contracted row behavior.
The earlier 21-target offline sweep passed 307 tests with zero failures and
one ignored parser child entrypoint exercised by its parent. The later
HistoryScan/ChangeScan implementation and final 22-target sweep are recorded
below. Exact KNN and Original Rerank now lower from HQL and bind from typed IR
to the same kernel under one P6 lease. The ReadView checks namespace and node
access, validates the exact owner revision at pinned S,V, and reads only its
schema-v6 original vector revision; it does not read HNSW, quantized data or a
sidecar. `space_id` resolves to the collection's H2-D11 fingerprint: lowercase
SHA-256 over the domain `genesis.hql2.vector-space.v1:` and the compact ordered
tuple (collection name, model label, dimension, metric, quantization). HQL
contextual vector parameters infer finite f64 values and are checked against
that space and dimension; typed IR declarations must agree. Exact L2 uses
squared Euclidean distance and Cosine uses the collection metric, with lower
distance ranked first and deterministic owner-identity/input-order ties.
KNN skips input owners without an original vector; Original Rerank fails closed
if any candidate lacks one. Candidate and distance budgets fail without
partial rows. Projected score is a typed Score (`.value` canonical, `.distance`
an HQL alias). Approximate KNN remains fail-closed. `Expand`
supports compact and ordered multi-segment Sequence patterns through HQL and
typed IR. Each segment supports direction/relation filters and bounded 0..=32
hops; Walk/Trail/Simple uniqueness applies across the complete sequence.
Optional null extension, per-step node/edge aliases, one typed path value,
deterministic ordering and fail-closed graph-budget exhaustion use the same P6
lease and measured expansion counters. Root `Match` now executes structural
compact and ordered Sequence patterns through HQL and typed IR; `SHORTEST`
returns one deterministic minimum-hop path per endpoint pair, with edge then
vertex identity ordering for ties. Typed-IR root Match anchors now accept only
validated, non-null `RecordRefV2` literals or declared parameters for node
aliases; full identity is compared within the same graph snapshot, without a
direct lookup, before SHORTEST deduplication. Focused Match and value targets
pass 12/12 and 6/6; the post-anchor 22-target HQL2 sweep passes 319/0/1.
The owner delegated the bounded Sequence-pattern decision on 2026-09-30.
Sequence ID, conjunctive labels and D4 node/edge property constraints now run
for HQL and typed IR against one authorized P6 graph snapshot; candidate IDs
are never looked up directly. Exact JSON values are selectively hydrated under
budget and filtered before SHORTEST; Compact constraints remain unavailable
and fail closed. The 10/10 P8 completion target and latest 383/0/1 root-HQL2
sweep across 33 targets cover this slice, including edge-property filtering
before SHORTEST and the 10/10 D7 actor-scoped HQL1 adapter, now including a
differential-proven single-label zero-hop form. P6
D1's exact-record-only fixture denies before parsing; it does not assert
hidden-vs-absent query results. A read-only review found an undercount in
JSON-literal size measurement; the lowerer now performs an iterative checked
serialized-size preflight with three passing unit tests, and a focused
confirmation review marked that finding fixed. A separate independent
read-only implementation review found no concrete static defect. Transport
parity and broader P8 acceptance remain open; this checkpoint does not close
HQL2/P8.
HistoryScan and ChangeScan execute from exact retained schema-v6 revisions
under the P6 ReadView. Storage-backed HQL and typed-IR HistoryScan bags for
Node, Edge, Row, Vector and Annotation now match independently assembled P7
catalogs from WAL revision facts and captured frontiers/valid-time windows.
The fixtures cover Node's close/replacement frames, Edge retraction and endpoint
ACL, exact Row revision IDs, H2-D11 Vector tuple identity and owner-Node ACL,
and Annotation target/evidence references with explicit body-read permission.
The focused History/Change target passes 16/16 and the Annotation source target
passes 7/7. Artifact HistoryScan remains capability-unsupported under the
approved contract. Storage-backed P7 ChangeScan differentials now cover all five
supported revision kinds. The current explicit 33-target HQL2 sweep passes
383/0/1, with the ignored parser child entrypoint exercised by its parent. A
storage-backed HQL/typed-IR `Values`/`UnionAll` differential matches independent
P7 for 169 nullable bag pairs (338 Storage executions), retaining duplicate
and NULL multiplicity through explicit null-last ordering. The scalar pipeline
differential adds six fixture cases (12 Storage executions) across HQL and
typed IR for projection/arithmetic/order/offset/take and
filter/project/distinct/order/take, each matching independent P7 and each
other. These remain focused test evidence, not expanded contract scope.
The accepted transaction-time contract selects one historical frame S for the
entire query and reports `Snapshot.tx = S`. HQL2 and typed-IR `query_v2` now
execute this contract end to end: all revision-backed scans, graph/vector/
annotation operators, property hydration and result metadata use the same S,
while the validated P6 generation, catalog and current policy stay pinned.
Per-source history floors fail closed before reads, and no path falls back to
current state. Five focused targets pass 56/56; the HistoryScan/ChangeScan
  target passes 16/16; the explicit 33-target HQL2 regression sweep passes
  383/0/1 and the separate P6/schema-v6/compatibility sweep passes 194/0/0.
These are local regression results, not broad P8/P13 or transport acceptance.
PR #196 at head `8ac07f6` was merged at `fb7085a`. Its 16 displayed checks
include 10 passes, five failures and one skip. Worker tests fail on
Linux/macOS/Windows, the rebuilt Linux addon/worker fails at markerless
database identity, and Windows Rust fails at 15m16 with detailed logs
unavailable. These hosted failures do not alter the typed contract or local
test counts; worker startup and Windows job-time corrections remain open.
LexicalMatch uses only registered `unicode-whitespace-bm25-v1` exact scan;
ContextPack uses registered `unicode-scalar-v1` with same-revision evidence,
source hash and Unicode-scalar offsets. Contextual NULL/list/JSON literal
typing is implemented with ambiguous or heterogeneous cases failing closed.
Their 10/10 focused target and budget-exhaustion cases pass. D7 admits the
differential-tested zero-hop HQL1 ID projection with either no label or one
plain-ASCII label, plus bounded unlabeled one-hop projections through
actor-bearing `Storage::query_v2`, including one exact endpoint-ID string
equality filter on one-hop patterns; the existing v1 transports and all other
HQL1 forms remain on their prior path. Legacy parser resources are preflighted
and reserved before AST construction. Full shared HQL1/v2/IR binding, broad
exact-oracle coverage and resource/cancellation gates remain open. Earlier
D1-D6 static review found no concrete implementation finding; independent D7
review remains pending. This source checkpoint does not qualify P8 or full
HQL2. No engine version is changed.

On 2026-09-29 the owner delegated the remaining HistoryScan/ChangeScan semantic
choice. The decisions below freeze their query contract; implementation and
focused local verification are recorded in the P8 core checkpoint, without
qualifying P8.

Parent: [Master specification](MASTER-SPEC--GENESIS-DB.md).
Peers: [P6 read contract](SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md),
[G0 structural contracts](SPEC--GENESISDB-UEE-HQL2-G0-CONTRACTS.md),
[legacy typed IR](SPEC--GENESISDB-TYPED-QUERY-IR-V1.md), and
[compatibility HQL](SPEC--HQL-V2.md). The approved storage contract is
[H2-D11](adr/ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS.md).

The owner approved [P8 completion addendum 0.1.1b](adr/ADR--GENESISDB-HQL2-P8-COMPLETION-ADDENDUM.md)
on 2026-10-02. It authorizes the bounded `unicode-whitespace-bm25-v1`
exact-scan and `unicode-scalar-v1` ContextPack profiles, Sequence node/edge
property constraints, and contextual NULL/list/JSON literal binding. It also
replaces the infeasible ACL-hidden-vs-absent `NO_MATCH` fixture with the P6
pre-parse `FORBIDDEN/authorize` assertion for an exact-record-only actor.
These D1-D5 runtime slices now have focused tests and explicit regression-sweep
evidence in the isolated worktree; the reviewer-identified D5 size-counter gap
was corrected and confirmation review marked it fixed. A separate independent
read-only implementation review found no concrete static defect. No P8
completion is claimed; the full D6 shared-pipeline, oracle, resource and P13
gates remain.

## Accepted decisions

1. Add a **nonpublishing catalog-only authorization boundary** for EXPLAIN.
   It holds the commit lock, validates current policy, reads schema/capability
   metadata, and cannot enumerate user rows. It never calls `pin_generation`.
   That existing function may flush indexes, checkpoint and append a signed
   GenerationPublished receipt; therefore using it for EXPLAIN violates D08.
2. Keep plan-only and executed results separate at the Rust boundary. A plan
   reports an ephemeral catalog stamp, not an invented durable snapshot.
   `QueryOutcomeV2::Plan` below is not yet a REST response or an alteration of
   the vendored Blueprint result schema. P13 must freeze its transport mapping.
3. Add typed, authorized source enumeration under a P6 ReadView. At this
   checkpoint Node/Edge/Row/Annotation scans, exact KNN/Original Rerank
   original-vector lookups, and HistoryScan/ChangeScan execute under the
   crate-private P6 ReadView. Unsupported source kinds fail with capability
   errors; this does not expose Storage.
4. Preserve current default-namespace ownership and WAL authority. Do not
   invent model fingerprints or historical annotation data from current rows.
   The approved H2-D11 addendum authorizes a versioned revision event and an
   additive schema-v6 implementation; it does not authorize running migration
   against a user database, switching to GBF2/GBO2, or bypassing per-source
   capability/history gates.
5. `HistoryScan(kind,id)` binds a constant UTF-8 logical ID and enumerates every
   retained matching revision committed by the pinned frontier `S` whose
   half-open valid interval contains the request's selected valid time `V`.
   It intentionally ignores `tx_to` as a visibility filter, includes durable
   retract tombstones, and orders by numeric `tx_from` then `revision_id`.
   Each row is a typed `HistoryRevision` carrying the exact subject reference,
   stored operation, transaction interval and valid interval. Exact property
   reads use that revision under the same lease; they never fall back to the
   current revision. Retract rows expose metadata but no user properties.
   Node/Edge use the P6 graph floor; Row, Vector and Annotation use their
   H2-D11 source floors. A Vector logical ID is the compact JSON encoding of
   the ordered `(owner_id, collection_id)` pair (for example,
   `["node-a","default"]`); its current P6 ACL is evaluated through the
   owner Node at the same lease. Artifact history remains
   capability-unsupported because no durable artifact source/floor is approved
   or implemented.
6. `ChangeScan(after_seq)` emits typed `ChangeEvent` values for durable
   revision rows in the exclusive/inclusive transaction window
   `(after_seq,S]`, independent of valid time. `S` is the pinned P6 frontier;
   `after_seq > S` is an invalid-bounds bind error and equality yields an empty
   feed. A stored `retract` maps to `retract`; an `upsert` with a predecessor
   maps to `correct`; an `upsert` without one maps to `upsert`. The event's
   sequence is canonical decimal-u64, and its subject is the exact
   database-bound revision reference. No persistent `Event` record kind is
   invented. Rows sort by `(tx_from, kind, record_id, revision_id)` so
   multi-record transactions are deterministic. The feed covers currently
   revisioned Node, Edge, Row, Vector and Annotation records; an unknown or
   unsupported revision kind fails closed rather than producing a partial
   feed. Before reading, the cursor must be at or above every participating
   source floor (graph, row, vector and annotation), otherwise return
   `HISTORY_UNAVAILABLE`. A floor is the minimum accepted exclusive cursor:
   equality is valid and scans only revisions strictly after the floor. Thus
   a migration baseline at frontier `F` remains available to HistoryScan at
   `F` but is not emitted as a ChangeScan event after cursor `F`. Current P6
   authorization applies at the pinned lease; inaccessible subjects are
   omitted, and Edge endpoints, Vector owners, and Annotation targets/evidence
   are checked recursively. An Annotation subject without explicit
   Annotation(Read) is excluded before ChangeScan candidate-budget accounting
   and materialization; this grant is not required for other readable
   ChangeScan subjects. Budget exhaustion returns an error, never a partial
   feed.

## Module ownership and data flow

```text
src/query/hql.pest + ast.rs       existing compatibility path, unchanged
src/query/hql2/{syntax,ast}.rs    explicit hql.v2 source + spans
src/query/hql2/{wire,bind}.rs     closed config + shared semantic binding
src/query/hql2/{plan,exec}.rs     deterministic plan + typed batches
src/query/hql2/{value,error}.rs   native values + stable errors
src/lib.rs                      one owner: authorization/catalog/read adapter

HQL2 -> source AST -> closed logical DAG <- query-ir.v2
                    -> shared binder <- authorized catalog
                    -> bound DAG -> deterministic physical plan
                                      |-> plan-only catalog result
                                      `-> one P6 lease
                                          |-> current typed scans
                                          |-> HistoryScan -> exact retained revisions
                                          `-> ChangeScan -> bounded revision events
                                              -> floor + current-ACL checks -> typed result
```

Only the production implementation uses these modules. Test support under
`tests/support/hql2_*reference.rs` must never be imported by engine source.
Differential fixtures are adapted independently into each representation.

## Rust signatures

Names here are proposed contracts, not existing symbols. Internal bound types
have private fields/constructors; no serde deserialization of trusted plans.

```rust
pub fn parse_hql2(source: &str) -> Result<Hql2Statement, QueryErrorV2>;

pub(crate) fn lower_hql2(
    statement: Hql2Statement,
) -> Result<LogicalRequestV2, QueryErrorV2>;

pub(crate) fn decode_ir_v2(
    ir: QueryIrV2,
) -> Result<LogicalRequestV2, QueryErrorV2>;

pub(crate) fn bind_v2(
    request: LogicalRequestV2,
    params: &BoundParametersV2,
    catalog: &AuthorizedCatalogV2,
) -> Result<BoundQueryV2, QueryErrorV2>;

pub(crate) fn plan_v2(
    query: BoundQueryV2,
) -> Result<PhysicalPlanV2, QueryErrorV2>;

impl Storage {
    pub fn query_v2(
        &self,
        access: AccessContext,
        request: QueryRequestV2,
    ) -> Result<QueryOutcomeV2, QueryErrorV2>;
}

pub enum QueryOutcomeV2 {
    Rows(QueryResultV2),
    Plan(ExplainResultV2),
}
```

`AccessContext` has the existing P6 in-process trust assumption. Request
namespace must agree with it. No request actor string becomes authenticated
identity; no REST/NAPI/FFI route is added by this contract.

`Hql2Statement` separates Read, Explain, AnalyzeRead, Mutation and Ddl.
Every AST node has `Span { start_byte: usize, end_byte: usize }`, half-open
within the original UTF-8 source. Parse the complete pinned grammar without
claiming all statements execute. Mutation/DDL and write ANALYZE reject with
CAPABILITY_UNSUPPORTED before pin/publication/effects until D10 is implemented.

## Closed logical configuration

The vendored [query IR schema](../tests/fixtures/hql2/blueprint/contracts/query-ir.schema.json)
is authoritative for field spelling, enum discriminators, nested shapes,
required fields and scalar bounds. All structs use `deny_unknown_fields`;
unknown fields may not survive in a generic config map after decoding.

| Wire operator | Ordered inputs | Required typed fields; optional fields in parentheses |
|---|---:|---|
| NodeScan | 0 | `as: Symbol`, `(label: Utf8)` |
| EdgeScan | 0 | `as: Symbol`, `(relation: Utf8)` |
| RowScan | 0 | `table: Symbol, as: Symbol` |
| AnnotationScan | 0 | `as: Symbol` |
| Values | 0 | `param: Symbol, as: Symbol` |
| HistoryScan | 0 | `kind: EntityKind (node, edge, row, vector, annotation; artifact unsupported), id: Expr, as: Symbol` |
| ChangeScan | 0 | `after_seq: DecimalU64, as: Symbol` |
| Match | 0 | `pattern: Pattern, anchors: Map<Symbol,Expr>, shortest: bool` |
| Filter | 1 | `predicate: Expr<Nullable<Bool>>` |
| Project | 1 | `fields: Vec<NamedExpr>` (nonempty) |
| Distinct | 1 | empty closed struct |
| Knn | 1 | `entity, collection: Symbol, query: Expr<Vector>, k: u32, mode: ExactOrApprox, as: Symbol` |
| Rerank | 1 | `entity, collection: Symbol, query: Expr<Vector>, k: u32, fidelity: Original, as: Symbol` |
| Expand | 1 | `pattern: Pattern, optional: bool` |
| AnnotationLookup | 1 | `target, as: Symbol, optional: bool` |
| LexicalMatch | 1 | `entity, field, index: Symbol, query: Expr<Utf8>, k: u32, as: Symbol` |
| Join | 2 | `kind: InnerOrLeftOrSemiOrAnti, condition: Expr<Nullable<Bool>>` |
| Aggregate | 1 | `group_by: Vec<NamedExpr>, aggregates: Vec<NamedAggregate>` |
| Sort | 1 | `keys: Vec<(Expr, AscOrDesc, NullsFirstOrLast)>` (nonempty) |
| Take | 1 | `count: u64` |
| Offset | 1 | `count: u64` |
| UnionAll | 2 | empty closed struct |
| ContextPack | 1 | `text, evidence: Expr, tokens: u32, tokenizer: Utf8, as: Symbol` |

For HQL2 only, `ORDER BY` may omit `NULLS FIRST/LAST`; lowering defaults that
key to `NULLS LAST` for both ascending and descending order. An explicit null
placement is preserved. Typed IR continues to require its closed `NullOrder`
field, so omission is never inferred at the IR boundary. HQL lowering and
storage-backed execution implement this default; the focused lower/execution/
scalar suites pass 58/58.

`Pattern` has closed Compact and Sequence variants exactly matching the schema.
Compact: start/end/optional edge aliases, relations, direction, min/max hops,
mode and optional path alias. Sequence: start node, ordered edge/node steps,
mode and optional path alias. Inline conditions belong inside optional patterns.
Bounds are checked before execution, even on empty input. Shortest is bounded,
unweighted, one deterministic path per bound endpoint pair; no weighted mode.

Root `Match.anchors` maps pattern node aliases to non-null `Expr<Entity>`
values. Because Match has no input scope, values must be literals or declared
parameters. Binding rejects non-node aliases (including edge/path aliases),
nullable values and other types. Execution compares complete `RecordRefV2`
identity (`database_id`, `namespace`, `kind`, `id`, `revision`) against nodes
already visible in the same P6 ReadView; it performs no direct anchor lookup.
Missing, stale, foreign-database or unauthorized references therefore yield no
matching row and cannot probe record existence. Anchor predicates apply before
deterministic SHORTEST deduplication.

`NamedExpr = { expression: Expr, as: Symbol }`. Wire aggregate call expressions
are decoded to CountAll/Count/Sum/Avg/Min/Max/Collect during binding, never
dispatched by unchecked function strings inside an execution loop.

`Expr` is a closed recursive enum from `$defs/expr`; registered functions are
resolved by name, arity, argument/result types, purity and error behavior.
Unregistered functions reject. Decimal frontier text is range-checked to u64;
usize conversion occurs only after configured limits, never unchecked casting.

## Types, scope and validation

The type registry includes the Blueprint scalar/domain types. Execution
capability is explicit per type/function. A recognized but unimplemented type
returns CAPABILITY_UNSUPPORTED, not JSON-string coercion or inferred support.
The first native kernels use Bool/I64/finite F64/UTF-8/nullable/list/typed
entity-revision/path/vector/score/context values. JSON is confined to declared
properties and boundary encoding, never used as the operator row protocol.

Validate all DAG nodes and reject cycles, orphan/unreachable nodes, wrong
ordered arity, duplicate IDs, alias ambiguity, incompatible UNION schemas,
undeclared fields/parameters, invalid aggregate scope, and collection/analyzer
fingerprints. Both frontend forms use this binder. Maximum depth is 128,
maximum nodes 10,000; check iteratively before recursive AST lowering/execution.

PROJECT replaces scope. SEMI/ANTI expose left fields only. Domain equality
uses namespace/kind/id/revision; a sortable domain comparator must be explicitly
registered. Missing optional JSON properties are NULL, unknown declared fields
are binding errors even for empty input. R1 preserves left-to-right potentially
throwing expression evaluation; no filter/top-k or outer-join rewrite is enabled.

## Results and errors

Executed QueryResultV2 follows the vendored
[result schema](../tests/fixtures/hql2/blueprint/contracts/query-result.schema.json):
request_id, real snapshot, typed columns/rows, semantics, completeness,
index_frontiers, required nullable cursor, optional explain/error. No timings
field is added to the closed Blueprint result. Sequence values serialize as
decimal strings. Unknown counters/frontiers are absent, never invented zeros.

Separate candidate search, distance fidelity, index coverage, execution
completeness and candidate-only rerank scope. An explicit TAKE is completion;
resource exhaustion is error, never a successful partial aggregate. ContextPack
emits a typed package binding consumable by RETURN/PROJECT, with exact registered
token counting of final text including separators/citations and omitted evidence.

`ExplainResultV2 = { request_id: String, catalog: CatalogStampV2,
plan: Vec<ExplainNodeV2>, root: String }`.
`CatalogStampV2 = { observed_frontier: u64, policy_revision: u64,
schema_fingerprint: String }` is catalog-only, not a durable snapshot lease.
`ExplainNodeV2 = { id: String, logical_op: QueryOpV2, physical_op: String,
inputs: Vec<String>, columns: Vec<ColumnV2>, estimates: Option<EstimateV2>,
actual: Option<ActualCountersV2> }`. `actual` is omitted for plan-only EXPLAIN.
Unknown estimates are absent. ANALYZE uses measured execution counters, not
post-hoc row-count guesses. Plans redact query literals and inaccessible metadata.

QueryErrorV2 follows the vendored
[error schema](../tests/fixtures/hql2/blueprint/contracts/error.schema.json):
code, stage, safe message, retryable; optional source span/detail/outcome only
when applicable. The Rust representation boxes optional detail to keep the
structured error small; its serialized JSON shape is unchanged. Stages:
contract, parse, bind, authorize, plan, execute, encode.
Stable codes: VERSION_UNSUPPORTED, HQL_PARSE_ERROR, BIND_ERROR,
CAPABILITY_UNSUPPORTED, AUTH_REQUIRED, FORBIDDEN, COLLECTION_SPACE_MISMATCH,
EXACT_ORIGINAL_UNAVAILABLE, BEYOND_HORIZON, SNAPSHOT_EXPIRED,
QUERY_BUDGET_EXCEEDED, CANCELLED. Preserve P6 error causes without exposing IDs.

## Reviewed and approved contract details

### Authorization, lifetime and bind/execute consistency

Initial v2 composed/catalog reads require `Read Namespace(access.namespace)`
in Enforced mode. Exact Node/Table/Collection grants alone are insufficient
for this new composed boundary, matching P6's broad-query rule. Disabled mode
retains its trusted embedded-caller behavior. Graph/vector ownership remains
`default`; another namespace is CAPABILITY_UNSUPPORTED only after namespace
authorization succeeds. Missing and inaccessible names have the same safe
FORBIDDEN response when revealing their existence would cross authorization.

Query catalog access is **not** public operational telemetry. Authorized callers
may inspect only their namespace's table/field types, permitted collection
definitions and implemented capability flags. No row counts, samples, histograms,
hidden object names or hidden-schema fingerprints are returned. Public versions
and health/capability booleans remain outside this schema-inspection API.

Proposed private acquisition signature:

```rust
fn with_hql2_catalog<T>(
    &self,
    access: &AccessContext,
    callback: impl FnOnce(&AuthorizedCatalogV2<'_>) -> Result<T, QueryErrorV2>,
) -> Result<T, QueryErrorV2>;
```

The callback is crate-private. The catalog borrows the guarded storage metadata;
it cannot escape, be deserialized, expose Storage, or open data operators. The
commit lock remains held across authorization, request normalization, binding,
planning and result construction. EXPLAIN completes only after final policy/
readability validation while still holding that lock, then releases it.

For execution, **the same outer commit guard** is retained while pinning and
executing under `with_read_lease`; P6's mutex is reentrant. Reacquire the catalog
from the lease and compare schema/policy dependency stamps before opening any
operator. A mismatch is BIND_ERROR (`catalog_changed` detail), not reuse or live
fallback. Final P6 validation precedes encoding/result publication. No cached
bound plan survives this call in P8. A concurrent writer/policy change must wait
or invalidate the operation; tests must demonstrate no bind/execute gap.

### Exact proposed ReadView extension

These methods are `pub(crate)`, not new transport capabilities:

```rust
impl ReadView<'_> {
    pub(crate) fn hql2_catalog(&self) -> Result<AuthorizedCatalogV2<'_>, QueryErrorV2>;
    pub(crate) fn hql2_scan(
        &self, source: &BoundSourceV2, after: Option<&SourceKeyV2>,
        limit: std::num::NonZeroU32,
    ) -> Result<SourceBatchV2, QueryErrorV2>;
    pub(crate) fn hql2_hydrate(
        &self, records: &[RecordRefV2], fields: &[FieldIdV2],
    ) -> Result<ExecBatchV2, QueryErrorV2>;
    pub(crate) fn hql2_vectors(
        &self, records: &[RecordRefV2], collection: CollectionIdV2,
    ) -> Result<VectorBatchV2, QueryErrorV2>;
}
```

`BoundSourceV2` is a private enum with Node(label), Edge(relation),
Row(table_id), Annotation, History(kind,id), Change(after_seq). It captures no
independent clock/policy. All calls inherit exactly the lease's S,V and current
authorized scope, validate before access, and retain the outer query guard.
Every batch is owned and contains no borrowed raw storage pointer.

`RecordRefV2 = { database_id: String, namespace: String, kind: EntityKind,
id: String, revision: String }`. `database_id` is lowercase SHA-256 of the
persisted public verifying-key bytes, domain-separated by UTF-8
`genesis.api.v2.database:`. Durable-source `revision` is a lowercase UUIDv4
issued by the engine and is never inferred from a wall clock, local frame,
SQLite rowid or current-payload hash. Reject missing/foreign database IDs;
never bind them to the current database implicitly. `SourceKeyV2` is an
opaque engine-created ordered tuple
`(database_id,namespace,kind,id,revision)` bound to the lease and source;
callers cannot supply a trusted cursor from JSON.
`SourceBatchV2 = { records: Vec<RecordRefV2>, next: Option<SourceKeyV2> }`;
nonempty next means another page may exist, never a new snapshot. Limit is
1..=1024 with query memory reservation before allocation.

`ExecBatchV2 = { columns: Vec<ColumnV2>, rows: Vec<Vec<QueryValueV2>>,
row_keys: Vec<StableRowKeyV2> }`; lengths agree and values conform to columns.
`FieldIdV2`/`CollectionIdV2` are binder-issued u32 indices, private constructors.
`VectorBatchV2 = { entries: Vec<Option<OriginalVectorV2>> }` is input-aligned:
missing vector is None, missing original fidelity is an explicit error for
Rerank. `OriginalVectorV2 = { space_id: String, scalar: F32OrF64,
values: Vec<f64> }`; f32 originals promote losslessly for computation. No
quantized bytes may be labeled original.

Current source matrix: Values has no storage dependency. Node/Edge/Row/Annotation
scans and Node/Edge/Row/Vector/Annotation HistoryScan use their H2-D11 identity,
revision, retention and ACL adapters under P6. ChangeScan covers graph, row,
vector and annotation revision events; lexical/context scans use only their
registered bounded profiles. Artifact HistoryScan remains
CAPABILITY_UNSUPPORTED because no durable artifact source/floor is approved.
This is an implementation dependency matrix, **not a reduction of final HQL2
acceptance**.
Unsupported sources fail during binding, before pin. Adding these methods
amends the P6 ReadView/resource whitelist only as explicitly described here.
They never expose unscoped Storage or widen grant semantics.

### Request normalization and initial policy matrix

| Request field | Initial v2 behavior |
|---|---|
| language | explicit hql.v2 or query-ir.v2; hql.v1 is accepted only by actor-bearing `Storage::query_v2` after `Namespace(Read)` and namespace-equality checks, and only for D7's differential-tested zero-hop node-ID projection (unlabeled or one plain-ASCII label) or one-hop unlabeled/unconstrained node-ID projection allowlist (one-hop may include one endpoint `.id` string equality filter) lowered into the shared HQL2 pipeline; parser resource/work-limit failures return QUERY_BUDGET_EXCEEDED before legacy AST construction; other valid legacy forms/options return CAPABILITY_UNSUPPORTED; existing v1 endpoints remain unchanged |
| explain | omitted means None; textual EXPLAIN/ANALYZE supplies the mode if the envelope is omitted; if both are supplied they must agree, otherwise BIND_ERROR; no mode silently upgrades plan-only to execution |
| namespace/temporal | textual/envelope values must agree; omitted valid_at is one UTC instant captured under the query guard; omitted transaction selector is the pinned data frontier L; explicit `tx_as_of` selects S with `history_horizon <= S <= L`; every HQL/IR source, hydration path and `Snapshot.tx` uses S without current-state fallback |
| index_policy | omitted means MergeDelta; initial exact scan does not consume an index and reports not_applicable coverage; explicit Wait/Eventual and Approx requests return CAPABILITY_UNSUPPORTED until implemented |
| required_frontier | range-checked local frame sequence; if greater than current stable frame frontier, return INDEX_COVERAGE_TIMEOUT, retryable=true, without publication or waiting in P8 |
| transaction_id | absent for reads; a supplied value on a read is BIND_ERROR; writes remain unsupported before any effects |
| allow_partial | omitted/false supported; true returns CAPABILITY_UNSUPPORTED until a separate partial-result contract passes qualification |
| format | omitted/json supported; stream returns CAPABILITY_UNSUPPORTED until P13 stream terminal-frame tests pass |
| budgets | defaults: memory 64 MiB, elapsed 5000 ms, nodes 100000, edges 200000, candidates 100000, distances 1000000, result rows 10000, result bytes 32 MiB; each supplied positive limit may only reduce the default; zero spill supported, nonzero spill unsupported in P8 |

These fail-closed initial states are surfaced as capabilities, not advertised as
completed features. Exact execution must error on exceeded limits, never fall
back to ANN or omit rows. Mutation/DDL statements are parsed but not executed.

### Type and value payload freeze

Initial exact type spellings are `Bool`, `I64`, `DecimalU64`, `F64Finite`,
`Utf8`, `Nullable<T>`, `List<T>`, and `Vector<space_id,dim,f32|f64>` (no
whitespace). DecimalU64 accepts and emits only canonical unsigned decimal
strings and compares numerically; it preserves the complete local sequence range.
In HQL structural unsigned positions (`TAKE`, `SKIP`, `CHANGE SCAN AFTER`,
`KNN k`, and `RERANK k`), a parameter resolves only from an exact, non-null
typed `DecimalU64` value. No implicit cast or nullable widening is allowed.
`TAKE`, `SKIP`, and `CHANGE SCAN AFTER` retain the full u64 range; `KNN k` and
`RERANK k` must additionally fit u32. Missing and wrong-type parameters fail
binding. Runtime lowering now obtains these values through
`BoundParametersV2`; focused lowering/execution tests pass 50/50 across four
targets. The post-parameter explicit HQL2 sweep passed 323/0/1 across 22 targets
at that checkpoint; later results are recorded in the current P8 checkpoint.
These are local regression results, not full P8 acceptance.
Vector wire parameters retain the pinned `{type:"vector",space_id,values}`
shape. Scalar params use `{type:"Bool|I64|F64Finite|Utf8",value:...}`; I64
value is a signed canonical decimal string, Bool a JSON bool, F64Finite a
finite JSON number, Utf8 a JSON string. Nullable null uses JSON null with its
declared Nullable type. List value is an array recursively matching T. Unknown
mandatory fields reject; type/parameter declarations must agree with the
resolved HQL argument context. Ambiguous unconstrained HQL parameters bind-error.

IR literal `type` uses the same exact type spelling and `literal` carries its
value (I64 decimal string). HQL integer literals parse into checked I64 and
decimal/exponent literals into finite F64. No implicit I64/F64/string casts.
Recognized Blueprint types outside this initial kernel set are explicitly
unsupported, not interpreted as arbitrary JSON. Entity parameters are accepted
only as validated `RecordRefV2` values used directly by typed-IR root Match
anchors; Entity literals/parameters in other expressions and nested Entity
parameter types remain unsupported. Path/Score/Context/HistoryRevision/
ChangeEvent parameter construction remains unsupported; those types are
engine-issued.

`QueryValueV2` variants: Null, Bool(bool), I64(i64), DecimalU64(u64),
F64(f64 finite), Utf8(String), List(Vec<QueryValueV2>), Entity(RecordRefV2),
Path{vertices:Vec<RecordRefV2>,edges:Vec<RecordRefV2>,mode:PathMode},
Vector(OriginalVectorV2), Score{value:f64,owner:RecordRefV2,source:String,
scope:WholeInputOrCandidates,metric:String}, Context(ContextPackageV2),
HistoryRevision(HistoryRevisionV2), ChangeEvent(ChangeEventV2),
Json(serde_json::Value) only for explicit declared JSON-property payloads.
The boundary encodes these as closed tagged objects `{type,value}`; user JSON
properties stay explicitly tagged JSON and cannot impersonate domain values.

Initial scalar functions: `has_prop(Entity,Utf8)->Bool`,
`prop(Entity,Utf8)->Nullable<Json>`, `lower(Utf8)->Utf8`,
`length(Utf8)->I64` (Unicode scalar count). JSON numeric extraction requires an
explicit registered checked cast before numeric operations; no cast registry
entry means CAPABILITY_UNSUPPORTED. Add/Sub/Mul/Div and `rem` use same-type
checked I64 or finite F64; the Query IR v2 discriminator is `rem`, and HQL `%`
lowers to it. Null operands propagate Null. I64 remainder truncates the
quotient toward zero; zero divisors fail with `division_by_zero`, and
`i64::MIN % -1` fails with `integer_overflow`. F64 remainder follows the
truncating-quotient rule (result sign follows the left operand); positive or
negative zero divisors fail with `division_by_zero`, and a non-finite result
fails closed. DecimalU64 is not an arithmetic type. Boolean operators accept
Bool/Nullable<Bool>; comparisons require
same scalar/domain equality type. Domain ordering is unsupported unless named.
An unknown function is BIND_ERROR; a recognized unavailable function is
CAPABILITY_UNSUPPORTED. The full grammar function vocabulary is not implied
implemented by these initial entries.

Aggregate returns: count/count_all I64; sum same numeric input type (checked);
avg F64Finite; min/max same ordered scalar type; collect List<T> bounded by
query memory/row budgets. Empty global count is 0, empty collect is [], other
empty global aggregates are NULL; empty grouped input has no groups. Count
ignores NULL, collect retains NULL, other aggregates ignore NULL.

### Snapshot, plan fields and context payload

Executed snapshot mappings: `tx` is the selected **local frame sequence**, not
the transaction-only frontier; current selection uses `lease.generation.
wal_frontier`, historical selection uses validated tx_as_of. `valid_at` is the
captured/selected UTC instant, `catalog_generation` the actual leased generation
ID, and `policy_version` its ACL revision. Proposed database_id is lowercase
SHA-256 of the persisted **public** verifying-key bytes, domain-separated by
UTF-8 `genesis.api.v2.database:`. This identifies the existing identity lineage;
cloned/restored identity has the same ID. No private key or filesystem path is
returned and no new identity file is created. This mapping is an explicit
owner-approved decision; it does not alter legacy wire identity guarantees.

ColumnV2 fields exactly `{name:String,type:String,nullable:bool}`. Plan estimates
are `EstimateV2 {rows_min:u64,rows_max:Option<u64>,confidence:LowOrMediumOrHigh}`;
omit them entirely until authorized stats exist. No cost/timing estimate exists
in the initial plan. `ActualCountersV2` has optional u64 fields `input_rows`,
`output_rows`, `distance_evaluations`, `expanded_nodes`, `expanded_edges`,
`source_records_examined`, `peak_accounted_bytes`, `elapsed_micros`; absent means
not measured, numeric zero means measured zero. No raw ID/query labels.

Initial physical identifiers are `Values`, `AuthorizedNodeScan`,
`AuthorizedEdgeScan`, `AuthorizedRowScan`, `Filter`, `Project`, `NestedLoopJoin`,
`BoundedExpand`, `ExactVectorScan`, `Aggregate`, `Distinct`, `StableSort`,
`Take`, `Offset`, `UnionAll`, `ContextPack`. An unavailable logical source does
not get a fabricated physical implementation. B-tree/HNSW paths are not reported
by this initial planner. Catalog schema_fingerprint is SHA-256 over a canonical
sorted encoding of only authorized schema/type/collection definitions and their
implemented analyzer/model identities, excluding user rows/literals/secrets.
Missing required fingerprint metadata makes that capability unsupported.

ContextPackageV2 fields: `rendered_context:String`, `token_count:u64`,
`token_budget:u64`, `tokenizer_fingerprint:String`, `fragments:Vec<FragmentV2>`,
`omitted_refs:Vec<EvidenceV2>`, `truncated:bool`, `truncation_reason:Option<String>`.
FragmentV2 is `{text:String,citation:String,evidence:EvidenceV2}`; EvidenceV2 is
`{source:RecordRefV2,source_hash:String,start_scalar:u64,end_scalar:u64}`.
PACK returns exactly one row `{as: ContextPackageV2}`, replacing input scope;
empty input returns an empty package row. Token counts cover rendered text,
citations and separators, **not diagnostic fields/omitted_refs**. Unknown
tokenizer rejects even empty input. No model tokenizer is inferred from an LLM
name. Omitting a package row through later operators omits its entire payload.

`HistoryRevisionV2 = { subject:RecordRefV2, operation:Utf8, tx_from:DecimalU64,
tx_to:Nullable<DecimalU64>, valid_from:Utf8, valid_to:Nullable<Utf8> }` and
`ChangeEventV2 = { sequence:DecimalU64, operation:Utf8, subject:RecordRefV2 }`.
Their tagged result values are engine-issued, closed domain values, never
untyped JSON. History members `id`, `kind` and `revision_id` are typed aliases
of the exact subject identity; `.subject` allows explicit `prop(subject,name)`.
ChangeEvent has no fabricated persistent event ID. `HistoryRevision.operation`
preserves the stored `upsert|retract` value. `ChangeEvent.operation` maps an
upsert with predecessor to `correct`, a first upsert to `upsert`, and a retract
to `retract`. DecimalU64 ordering is numeric and serialization is canonical.
Historical property hydration resolves the exact subject revision despite a
closed `tx_to`; retracts expose metadata and return NULL/false for property
reads. HQL and typed Query IR share these member and output semantics.

### Error mapping and source positions

| Condition | Code / stage / retryable |
|---|---|
| unknown version | VERSION_UNSUPPORTED / contract / false |
| malformed syntax or literal | HQL_PARSE_ERROR / parse / false |
| unknown field, wrong type, config or conflicting modes | BIND_ERROR / bind / false |
| unavailable enumerator/type/function/tokenizer | CAPABILITY_UNSUPPORTED / bind / false |
| missing access context | AUTH_REQUIRED / authorize / false |
| denied or inaccessible object/policy scope | FORBIDDEN / authorize / false |
| vector space/dimension/metric mismatch | COLLECTION_SPACE_MISMATCH / bind / false |
| missing requested originals | EXACT_ORIGINAL_UNAVAILABLE / execute / false |
| temporal beyond retention | BEYOND_HORIZON / bind / false |
| expired/stale/revoked/foreign lease | SNAPSHOT_EXPIRED / execute / true |
| integer overflow, division by zero, nonfinite result | BIND_ERROR / execute / false, safe reason `integer_overflow`, `division_by_zero`, or `nonfinite_result` |
| quota/deadline/cancellation | QUERY_BUDGET_EXCEEDED or CANCELLED / execute / false |
| required frontier unavailable | INDEX_COVERAGE_TIMEOUT / bind / true |
| recovery-required/corrupt handle | DATA_CORRUPTION / execute / false |

AST span start_byte maps to 1-based line and Unicode-scalar column in the
original source. CRLF is one newline; a tab is one scalar column. UTF-8 byte
offsets never become UTF-16 columns. End-byte remains internal because the
vendored error schema carries only a starting line/column. Errors omit spans
for IR without source text. No row/embedding/hidden-name content appears in
safe messages; stable reason codes appear in the detail object.

## Implementation checkpoint (not P8 closure)

On integrated base `22bc11e` (upstream engine 0.2.9 plus preserved local P6/P7),
the Rust-only implementation begins with the grammar frontend, all 23 closed
IR config shapes, one scalar binder/planner and the authenticated query_v2
boundary. See [P8 evidence](REPORT--HQL2-P8-CORE-2026-09-28.md) for exact tests
and remaining gates. The approved target above is unchanged.

The current exact kernel covers Values, Filter, Project, Distinct, Sort, Take,
Offset, UnionAll, Aggregate, Join, NodeScan, EdgeScan, RowScan, AnnotationScan,
AnnotationLookup, HistoryScan, ChangeScan, exact KNN and Original Rerank. Compact and ordered
multi-segment Sequence Expand kernels
are available through HQL and typed IR; they traverse only exact node/edge
revisions visible under one P6 lease, enforce per-step bounded hops and global
path uniqueness, bind per-step aliases, support optional null extension and
typed paths, order results deterministically, and fail rather than returning
partial rows when the graph budget is exhausted. Sequence execution and root
Match support structural patterns plus Sequence ID, conjunctive-label and
node/edge property constraints under the same P6 snapshot; Compact remains the
frozen unconstrained form. HQL constraints lower to the typed Sequence kernel.
Typed-IR root Match anchors use the frozen exact-identity
contract above; focused Match execution passes 12/12 and Entity-value decoding
passes 6/6. KNN and Original Rerank use the same bound
vector-rank kernel: exact original vectors are read under the P6 lease at S,V,
and the collection space is bound by the H2-D11 fingerprint. Approximate KNN
remains unsupported. HistoryScan/ChangeScan execute from exact retained
schema-v6 revisions under the P6 lease, including Vector HistoryScan by the
H2-D11 tuple ID, current reference ACL, exclusive source floors, typed HQL/IR
parity and fail-closed budgets. Storage-backed P7 HistoryScan and ChangeScan
differentials cover Node, Edge, Row, Vector and Annotation; the focused
History/Change target passes 16/16 and Annotation source/ACL passes 7/7. The
current 33-target HQL2 sweep passes 383/0/1, with a separate 11-target
P6/schema-v6/compatibility sweep at 194/0/0. HQL2/IR `tx_as_of` now selects
one no-fallback frontier across scans, operators, hydration and `Snapshot.tx`;
five focused targets pass 56/56 and HistoryScan/ChangeScan passes 16/16.
Broader exact-oracle, independent-review, transport and P8/P13 gates remain
open.
LexicalMatch uses only the registered `unicode-whitespace-bm25-v1` exact-scan
profile; ContextPack uses `unicode-scalar-v1` with same-revision evidence,
SHA-256 and Unicode-scalar spans. Their catalog fingerprints are code-registered
and included in the authorized catalog stamp; no durable index is added.
HQL contextual NULL/list literals and explicit JSON object literals are
implemented with ambiguous/heterogeneous/domain-value cases failing closed.
Quoted non-ASCII symbols and other unsupported non-scalar stages still reject
explicitly rather than inventing semantics. HQL remainder is implemented and locally verified:
`%` lowers to closed Query IR `rem`, with checked same-type I64/F64 execution
and NULL propagation. Four focused lowering/wire/scalar/storage targets pass
79/79; the explicit 22-target HQL2 sweep passes 328/0/1. These are local
regression results, not full P8 acceptance. HQL implicit null ordering is
implemented as NULLS LAST for both directions. Structural unsigned
parameters use the exact DecimalU64 contract above and are locally verified.
HQL
supports compact and Sequence Expand forms, including optional forms, and
structural root Match with deterministic SHORTEST selection; Sequence
id/label/node-property/edge-property constraints filter before SHORTEST. The HQL
stage `ANNOTATIONS OF target AS alias` lowers to required lookup and
`OPTIONAL ANNOTATIONS OF target AS alias` lowers to left-preserving lookup.
Typed IR nullable/list payloads and explicit scalar ordering execute. These are
open implementation tasks, not amendments reducing final acceptance.

The ReadView now scans schema-v6 record projections at the lease frontier and
valid-time bound. Source cursors bind source, database, revision, policy
revision, lease owner/fence, generation, principal, namespace and valid time;
pagination counts candidates hidden by annotation-reference ACL. Node/Edge
source filters reserve payload memory before loading JSON, while RowScan binds
to the authorized schema catalog and returns its durable revision identity.
Annotation hydration rechecks target/evidence access before loading its body.
`FieldIdV2`/`ExecBatchV2` enforce typed, row-key-aligned hydration for covered
sources, including row-dependent names; whole payloads are not retained in
operator rows. The Node/Edge/Row HQL/IR differential now passes: RowScan reads
properties from the H2-D11 `after_image` payload while `r.id` remains the
durable UUIDv4 revision, distinct from the relational primary key. A separate
storage-backed HQL/typed-IR scalar differential passes 1/1 against the
independent P7 interpreter over all 81 four-value bags formed from NULL, -1
and 2; both frontends match P7 with NULLS LAST across 162 query executions. The
storage-backed aggregate differential also passes 1/1 over 121 nullable bags
of lengths 0-4, comparing HQL and typed IR to independent P7 for count-all,
count, sum, average, min, max and collect (242 Storage executions).
The storage-backed HQL/typed-IR Join differential passes for Inner/Left/Semi/Anti,
comparing exact result bags with independent P7 over duplicate, SQL-NULL and
JSON-null keys (5/7/3/2 output rows). HQL `JOIN TABLE` lowers to the existing
closed RowScan and two-input Join operators; an omitted kind defaults to
Inner. Semi/Anti retain only the left scope, while Left exposes nullable right
values. The test covers missing-property NULL separately from JSON null and
maps P7's otherwise-unavailable JSON values through an explicit test sentinel.
This bounded fixture establishes frontend parity for these join forms; it does
not close broad exact-oracle, independent-review or P8 acceptance. The RowScan property-hydration
regression and correction are recorded in local RCA
`.brain/rca/RCA--HQL2-ROW-SOURCE-PROPERTY-HYDRATION.md`.
The ReadView also performs exact vector lookup for KNN/Original Rerank. It
checks owner identity/revision, namespace/node authorization, collection
fingerprint and dimension at the leased frontier/valid-time, and loads only
original schema-v6 vector payloads. KNN omits missing originals while
Original Rerank refuses an incomplete candidate set; HNSW/quantized/sidecar
paths are not used. Test-only exact-KNN and Original Rerank differentials
compare HQL and typed IR with the independent P7 rank oracle; both pass and
cover equal-distance ordering, missing-original exclusion for KNN, and top-k
reranking. This is not broad exact-oracle coverage. The latest explicit
earlier root-HQL2 sweep passed 331/0/1 across 24 targets including these
differentials and a test-only root `MATCH` comparison against the independent
P7 graph bag. That comparison preserves parallel `LINK` edges and excludes a
non-matching relation; it does not constitute broad exact-oracle or P8
acceptance. HistoryScan/ChangeScan now read exact retained revisions;
their focused and all-HQL2 regression evidence is recorded in the current P8
report. The approved addendum completion target passes 10/10, including lexical
profile/P7-rank parity, ContextPack source/hash/scalar-offset/P7 parity,
Sequence node/edge exact JSON properties, pre-SHORTEST filtering, contextual
literal typing and no-partial budget exhaustion. The D7 HQL1 adapter target
passes 10/10 for unlabeled or single-label zero-hop and unlabeled one-hop
actor-bound differential execution,
parallel-edge multiplicity, all directions and wildcard relations, endpoint-ID
string equality on either endpoint, denial/mismatch before parse, malformed
syntax, fail-closed unlisted syntax and pre-parse resource rejection. The
latest 33-target root HQL2 sweep passes
383/0/1; a separate 11-target P6/schema-v6/compatibility group passes 194/0/0.
The prior combined 37-target run passed 528/0/1 before
the added edge-property regression; all remain regression evidence, not full
P8/P13 acceptance.
Vector writes now persist owner-bound schema-v6 revisions and reject
unversioned schema-v6 ingress; two focused tests cover projection,
collection-space binding, compaction/reopen and local/peer rejection. The
H2-D11 schema-v6 migration/recovery path has since been implemented and passes
19 dedicated tests on temporary fixtures only. This does not qualify migration
on a user database.
No REST, NAPI, FFI, SDK or MCP adapter is added. New query execution may invoke
existing P6 generation publication; plan-only EXPLAIN cannot. SQLite's volatile
shared-memory read marks are not durable query effects; tests separately
compare signed WAL and snapshot bytes.

Parser admission is budgeted before the PEG allocates: source length at most
262144 Unicode scalars, at most 2048 lexical units, and a conservative pinned-
grammar heap plus optional worker-stack reservation within the default 64 MiB.
The request budget can only reduce this allowance. Grammar-valid wide queries
may return QUERY_BUDGET_EXCEEDED; post-parse depth128/node10000 checks remain
separate shape limits. No process-global Pest setting is changed.

## Acceptance checklist

- 35 positive and 6 negative pinned syntax fixtures; trailing statements,
  Unicode/spans, numeric overflow and parameter-value separation tests.
- Closed-config validation and distinguishing cases for every one of 23 ops;
  empty-input binding, orphan DAG, depth/node limits and schema propagation.
- HQL2 and JSON IR differential results against independent P7 expected bags,
  explicit order/errors/semantic metadata for each supported source capability.
- EXPLAIN on fresh and changed stores leaves WAL bytes/frontiers, generation
  receipts, snapshots and data/index operator-open counters unchanged.
- One authenticated scope/S,V/read lease; current ACL, historical horizon,
  revoked/stale/expired lease and final validation reject without live fallback.
- All temporary resources/leases released on error/cancel; quotas reserved
  before allocations; tests show a budget error cannot masquerade as completion.
- Existing HQL/v1 tests pass unchanged. The D7 actor-scoped bridge supports
  only its exact differential-tested zero-hop allowlist (unlabeled or one
  plain-ASCII label) and unlabeled one-hop allowlist; its parser is
  preflighted and budgeted before AST construction. Unproven legacy lowering
  stays on the compatibility path and the shared-runtime completion obligation
  remains open until every declared legacy form has equivalent evidence.
- Independent implementation review and verification must pass before P8 closure.
  Parser/binder coverage alone does not close the storage-backed runtime gate.

## Version diff and changelog

Version diff `0.2.54b -> 0.2.55b`: record test-only storage-backed HQL/typed-IR
scalar pipeline differentials against independent P7 for six deterministic
fixtures and 12 Storage executions. The focused scalar target passes 2/2 and
the 33-target HQL2 regression sweep passes 383/0/1. No runtime, schema,
transport or contract semantics changed; broad exact-oracle, shared-runtime,
review, resource/cancellation and P8/P13 gates remain open.

Version diff `0.2.53b -> 0.2.54b`: extend the conditional D7 HQL1 adapter
allowlist with one plain-ASCII label on zero-hop node-ID projections after
legacy/HQL2 differential; record 10/10 focused tests. The HQL2 regression
sweep remains 382/0/1 across 33 targets and the separate P6/schema-v6/
compatibility sweep remains 194/0/0. One-hop labels and all other unproven
forms remain unsupported; shared-runtime, review and broad P8/P13 gates remain
open. No P6 grant, schema, transport or migration behavior changed.

Version diff `0.2.52b -> 0.2.53b`: add test-only HQL/typed-IR
`Values`/`UnionAll` differential evidence against P7 for 169 nullable bag pairs
and 338 Storage executions, preserving NULL/duplicate multiplicity through
explicit null-last ordering. The HQL2 regression sweep is 382/0/1 across 33
targets; the separate P6/schema-v6/compatibility sweep remains 194/0/0. No
contract, runtime, schema or transport behavior changed; broad exact-oracle,
ChangeScan, review, P8/P13 and qualification gates remain open.

Version diff `0.2.51b -> 0.2.52b`: synchronize hosted evidence for merged
PR #196 at head `8ac07f6` / merge `fb7085a`. Local History/Change 16/16,
HQL2 380/0/1 and P6/schema-v6/compatibility 194/0/0 remain unchanged; hosted
checks show five failures and one skipped, including worker bootstrap and an
unverified Windows Rust failure at 15m16. The contract is unchanged; full
P8/P13 and worker approval gates remain open.

Version diff `0.2.50b -> 0.2.51b`: extend the storage-backed P7 ChangeScan
differential from Node to Edge, Row, Vector and Annotation revisions. Compare
seven events for exact sequence, operation and subject identity, with recursive
edge-endpoint, vector-owner and annotation target/evidence dependencies; HQL and
typed IR agree. Record History/Change 16/16 and the 32-target HQL2 sweep
380/0/1. Broader ChangeScan semantics, full P8/P13, hosted checks and review
remain open.

Version diff `0.2.48b -> 0.2.49b`: add storage-backed P7 HistoryScan
differentials for Node, Edge, Row and Annotation, completing the five supported
HistoryScan kinds with the existing Vector profile. Expected catalogs use
WAL-derived revision facts and explicit frontiers; checks cover exact revision
identities, valid/transaction intervals and applicable endpoint/owner/reference
ACLs. Record History/Change 15/15, Annotation source 7/7, HQL2 379/0/1,
P7 130/130 and P6/schema-v6/compatibility 194/0/0. Artifact HistoryScan,
broad ChangeScan/oracle coverage, hosted checks/review and broad P8/P13 remain
open.

Version diff `0.2.46b -> 0.2.47b`: freeze and implement the narrow ChangeScan
ACL/budget rule: keep namespace-authorized non-annotation subjects readable,
and exclude Annotation subjects without Annotation(Read) before caller-budget
accounting. The adversarial threshold regression passes with ACL 11/11,
History/Change 14/14, HQL2 376/0/1 and selected P6/schema-v6 tests 43/0/0.
This verifies quota-result behavior, not timing noninterference. Broad P8/P13,
transport, hosted CI and independent-review gates remain open.

Version diff `0.2.45b -> 0.2.46b`: add storage-backed HQL/typed-IR Row
HistoryScan/ChangeScan parity for retained insert/update revisions, exact
after-image property hydration and Row subject identity (1/1); refresh the
explicit HQL2 regression sweep to 374/0/1 across 32 targets and correct the
stale plan status. Broad exact-oracle, resource/cancellation, independent
review, shared-runtime and P8/P13 gates remain open.

Version diff `0.2.44b -> 0.2.45b`: implement and locally verify P8 HQL2/IR
`tx_as_of` selection through one no-fallback frontier S across source scans,
graph/vector/annotation operators, source-floor checks, hydration and
`Snapshot.tx`; preserve the pinned P6 generation/current policy. Record 56/56
across five focused targets, 14/14 HistoryScan/ChangeScan, 373/0/1 across 31
HQL2 targets and 194/0/0 across 11 P6/schema-v6/compatibility targets. Broad
P8/P13, transport, hosted CI and independent-review gates remain open.

Version diff `0.2.43b -> 0.2.44b`: clarify that explicit P8 `tx_as_of` selects
one historical frontier S for every HQL/IR source, hydration path and
`Snapshot.tx`, with bounds `history_horizon <= S <= L` and no current-state
fallback. Runtime implementation was pending at that checkpoint.

Version diff `0.2.42b -> 0.2.43b`: implement HQL/typed-IR Vector HistoryScan
using the approved H2-D11 compact tuple identity, P6 vector source floor and
same-lease owner-node ACL; record 12/12 focused history/change tests, 367/0/1
across 31 HQL2 targets and 194/0/0 across 11 P6/compatibility targets. Broad
P8/P13, tx_as_of, transport and independent-review gates remain open.

Version diff `0.2.10b -> 0.2.11b`: record the measured RowScan property-hydration RED and RCA without changing the approved contract; the runtime correction remains approval-gated.
Version diff `0.2.11b -> 0.2.12b`: implement the bounded `after_image` hydration correction; focused source tests pass and the then-current 21-target sweep retains three vector operator failures.
Version diff `0.2.12b -> 0.2.13b`: implement exact KNN and Original Rerank with lease-bound original-vector reads, H2-D11 space fingerprints, HQL/IR parity and 307 passing tests across 21 root HQL2 targets; retain unsupported operators and P8 gates.
Version diff `0.2.16b -> 0.2.17b`: freeze root Match anchor semantics as exact, non-null `RecordRefV2` identity predicates over the pinned P6 graph snapshot; typed anchor execution remains pending, and other P8 gates stay open.
Version diff `0.2.17b -> 0.2.18b`: implement validated Entity literal/parameter anchors for typed-IR root Match, reject Entity construction outside anchors, apply predicates before SHORTEST, and record 12/12 Match, 6/6 value tests and 319/0/1 across 22 HQL2 targets; remaining constraints and P8 gates stay open.
Version diff `0.2.18b -> 0.2.19b`: implement exact DecimalU64/no-cast semantics for HQL structural unsigned parameters across TAKE, SKIP, CHANGE SCAN, KNN and RERANK; record 50 focused passes and 323/0/1 across 22 HQL2 targets while retaining remaining P8 gates.
Version diff `0.2.19b -> 0.2.20b`: implement HQL2-only implicit ORDER BY null placement as NULLS LAST for either direction while keeping the typed-IR NullOrder field required; record 58 focused passes and 325/0/1 across 22 HQL2 targets.
Version diff `0.2.20b -> 0.2.21b`: freeze checked same-type I64/F64 remainder semantics and add the `rem` Query IR v2 discriminator for HQL `%`.
Version diff `0.2.21b -> 0.2.22b`: implement HQL `%` through closed Query IR `rem` and checked shared scalar execution; record 79/79 focused passes and 328/0/1 across 22 HQL2 targets while retaining remaining P8 gates.
Version diff `0.2.22b -> 0.2.23b`: add a passing test-only exact-KNN differential for HQL and typed IR against the independent P7 oracle, including equal-distance ordering and missing originals; record 329/0/1 across 23 root targets; broad exact-oracle coverage and P8 closure remain open.
Version diff `0.2.23b -> 0.2.24b`: add and pass Original Rerank differential coverage for HQL and typed IR against the independent P7 oracle; record 2/2 ranking tests and 330/0/1 across 23 root targets; broad exact-oracle coverage and P8 closure remain open.
Version diff `0.2.24b -> 0.2.25b`: add and pass a test-only HQL root `MATCH` differential against the independent P7 graph bag; record 1/1 focused and 331/0/1 across 24 root HQL2 targets; broad exact-oracle coverage and P8 closure remain open.
Version diff `0.2.25b -> 0.2.26b`: record the delegated decision to implement only Sequence node ID and conjunctive label constraints through HQL and typed IR under one P6 snapshot; properties and Compact constraints remain fail-closed.
Version diff `0.2.26b -> 0.2.27b`: record the implemented P6-snapshot Sequence ID/label slice, 7 focused passes, and 338/0/1 across 25 HQL2 targets; keep ACL-hidden fixture, independent review, transport and full P8 acceptance open.
Version diff `0.2.27b -> 0.2.28b`: synchronize owner-approved P8 addendum D1-D6, authorizing bounded text profiles, provenance-preserving ContextPack, Sequence properties and contextual literals.
Version diff `0.2.29b -> 0.2.30b`: add and pass edge-property filtering-before-SHORTEST regression; record 10/10 completion tests, 349/0/1 across 26 root HQL2 targets and 190/0/0 across 11 separate P6/schema-v6/compatibility targets; independent static review found no concrete defect; retain broad P8/P13 gates.
Version diff `0.2.30b -> 0.2.31b`: approve an actor-scoped HQL1 adapter through `Storage::query_v2`; initially allow only a zero-hop node-ID projection lowered through the shared HQL2 pipeline after P6 authorization.
Version diff `0.2.31b -> 0.2.32b`: implement the approved D7 allowlist; record 5/5 focused actor-bound adapter tests and 354/0/1 across 27 root HQL2 targets; retain all other HQL1 forms, shared-runtime, P8/P13 and independent-review gates.
Version diff `0.2.34b -> 0.2.35b`: implement D7's one-hop endpoint-ID string equality predicate after legacy/HQL2 bag differential; record 9/9 focused adapter tests and the final 361/0/1 root-HQL2 plus 190/0/0 compatibility sweeps; retain the full shared-runtime/P8/P13/review gates.
Version diff `0.2.28b -> 0.2.29b`: implement approved D1-D5 with 9 focused passes, budget-failure/no-partial coverage, and 528/0/1 across 37 explicit HQL2/P6/schema-v6/compatibility targets; retain independent review and broad P8/P13 gates.

Version diff `0.2.35b -> 0.2.36b`: record implementation and fixture
verification of approved H2-D11 R6b markerless schema-v6 WAL recovery; crash
17/17, migration 19/19 and selected HQL2/durability/authority 40-target
aggregate pass; retain full P8/P13/review gates and the no-user-migration rule.

Version diff `0.2.40b -> 0.2.41b`: implement HQL `JOIN TABLE` lowering through
the approved typed-IR Join operator for Inner/Left/Semi/Anti, default bare JOIN
to Inner, and add a storage-backed HQL/IR differential against independent P7;
record 365/0/1 across 31 HQL2 targets. Broad exact-oracle, independent-review,
shared-runtime, resource/cancellation, transport and P8/P13 gates remain open.

Version diff `0.2.38b -> 0.2.39b`: add a storage-backed HQL/typed-IR aggregate
differential against independent P7 for all 121 nullable bags of lengths 0-4
(242 executions); record 1/1 focused and 363/0/1 across 29 HQL2 targets.
Broad exact-oracle, independent-review, shared-runtime, resource/cancellation,
transport and P8/P13 gates remain open.

Version diff `0.2.37b -> 0.2.38b`: add a storage-backed HQL/typed-IR scalar
differential against the independent P7 interpreter for 81 nullable bags;
record 1/1 focused, 362/0/1 across 28 HQL2 targets, and 190/0/0 across 11
P6/schema-v6/compatibility targets. Broad exact-oracle, independent-review,
shared-runtime, resource/cancellation, transport and P8/P13 gates remain open.

Version diff `0.2.36b -> 0.2.37b`: record final local HQL2/H2-D11 integration
verification, including full locked/offline Rust tests (`probe_vs_recall` NOT_RUN),
both default and no-default strict Clippy passes, and boxed optional error
detail with unchanged JSON shape; broader P8/P13/review gates remain open.

| From | To | Effect |
|---|---|---|
| 0.2.54b | 0.2.55b | Record six test-only storage-backed HQL/typed-IR scalar-pipeline fixtures against P7 with 12 Storage executions and 2/2 focused tests; update root-HQL2 regression to 383/0/1; preserve runtime, contract, schema, transport and broad-acceptance boundaries |
| 0.2.53b | 0.2.54b | Extend the conditional D7 HQL1 zero-hop node-ID allowlist to include a single plain-ASCII label after legacy/HQL2 differential; adapter 10/10; leave labeled one-hop and unproven forms fail-closed, with broad shared-runtime/P8/P13/review gates open |
| 0.2.43b | 0.2.44b | Freeze end-to-end P8 transaction-time selection: explicit `tx_as_of` selects S across HQL/IR sources, hydration and `Snapshot.tx`; enforce `history_horizon <= S <= L`, retain the current P6 lease/policy generation and prohibit fallback; runtime pending |
| 0.2.42b | 0.2.43b | Implement HQL/typed-IR Vector HistoryScan using H2-D11 compact JSON tuple identity, P6 vector floor and same-lease owner ACL; record focused 12/12, HQL2 367/0/1 and P6/compatibility 194/0/0; retain tx_as_of and broad P8/P13/review gates |
| 0.2.41b | 0.2.42b | Specify approved Vector HistoryScan using the H2-D11 compact JSON `(owner_id, collection_id)` logical ID, vector history floor and P6 owner-node ACL; implementation and parity evidence pending |
| 0.2.40b | 0.2.41b | Implement HQL JOIN lowering through existing RowScan/Join contract for four kinds, bare JOIN defaults to Inner; HQL/typed IR match P7 for duplicate, SQL-NULL and JSON-null keys; record 365/0/1 across 31 targets; broad oracle/review/P8/P13 gates remain open |
| 0.2.39b | 0.2.40b | Add and pass storage-backed typed-IR Join P7 differential for Inner/Left/Semi/Anti with duplicate/NULL/unmatched keys; record 364/0/1 across 30 HQL2 targets; do not expand HQL JOIN support or close broad oracle/review/P8/P13 gates |
| 0.2.38b | 0.2.39b | Add and pass storage-backed HQL/typed-IR P7 aggregate differential over 121 nullable bags; record 363/0/1 across 29 HQL2 targets; retain broad oracle, review, shared-runtime, resource/cancellation and P8/P13 gates |
| 0.2.37b | 0.2.38b | Add and pass storage-backed HQL/typed-IR P7 scalar differential over 81 nullable bags; record 362/0/1 across 28 HQL2 targets and 190/0/0 across 11 compatibility targets; retain broad oracle, review, shared-runtime, resource/cancellation and P8/P13 gates |
| 0.2.36b | 0.2.37b | Verify full locked/offline Rust suite and both strict Clippy modes; explicitly retain `probe_vs_recall` as NOT_RUN; box optional error detail internally without changing serialized JSON; keep full P8/P13/review gates open |
| 0.2.35b | 0.2.36b | Implement and verify H2-D11 R6b markerless schema-v6 recovery in temporary fixtures; 17/17 crash and 19/19 migration tests pass; selected aggregate passes 40 targets; full P8/P13/review gates remain open |
| 0.2.34b | 0.2.35b | Implement the conditionally approved one-hop endpoint-ID string equality filter; record 9/9 adapter tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 compatibility targets; retain shared-runtime/P8/P13 gates |
| 0.2.32b | 0.2.33b | Extend D7 with differential-proven one-hop HQL1 forms and pre-parse resource reservation; record 8/8 adapter tests, 361/0/1 HQL2 regression and 190/0/0 separate compatibility sweep; retain full shared-runtime/P8/P13 gates |
| 0.2.31b | 0.2.32b | Implement D7's exact actor-scoped HQL1 allowlist; record 5/5 focused tests and 354/0/1 across 27 root HQL2 targets; retain remaining shared-runtime/P8/P13 gates |
| 0.2.30b | 0.2.31b | Record approved D7 HQL1 adapter boundary and exact initial allowlist; implementation and differential evidence pending; retain all other shared-runtime/P8/P13 gates |
| 0.2.29b | 0.2.30b | Record edge-property-before-SHORTEST regression, 10/10 completion tests, 349/0/1 across 26 HQL2 targets and separate 190/0/0 across 11 P6/schema-v6/compatibility targets; retain broad P8/P13 gates |
| 0.2.28b | 0.2.29b | Implement approved P8 D1-D5 and record focused, budget, catalog-profile and 37-target regression evidence; retain broad P8/P13 gates |
| 0.2.27b | 0.2.28b | Synchronize the owner-approved P8 completion addendum D1-D6; implementation and verification remain pending |
| none | 0.1.1b candidate | Proposed typed P8 API/config/result boundary, concrete policy/type mappings and P6 catalog/read extensions; no implementation approval inferred |
| 0.1.1b candidate | 0.1.1b beta | Owner approved the unchanged contract and isolated upstream integration; no push, PR merge, deployment or user database migration authorized |
| 0.1.1b beta | 0.1.2b beta | Record the partial implementation matrix and evidence link without narrowing the approved target |
| 0.1.2b beta | 0.2.0b beta | Require database_id and UUIDv4 revision identity; bind source adapters to approved H2-D11 storage, retention and annotation contracts |
| 0.2.0b beta | 0.2.1b beta | Record fixture-only schema-v6 migration evidence while keeping source adapters, transport, and P8 acceptance open |
| 0.2.2b beta | 0.2.3b beta | Record annotation CAS-write, target/evidence normalization and P6 policy-v2 evidence; keep annotation reads, storage adapters, transport and P8 acceptance open |
| 0.2.3b beta | 0.2.4b beta | Record lease-bound Node/Edge/Row/Annotation scans, ACL checks, keyset pages, budgeted property access and nullable identity behavior; keep typed field-batch hydration, remaining operators, transport, and P8 acceptance open |
| 0.2.4b beta | 0.2.5b beta | Implement binder-issued field IDs and aligned selective typed hydration; preserve dynamic property names and keep remaining P8 gates open |
| 0.2.5b beta | 0.2.6b beta | Implement HQL/IR AnnotationLookup with frozen/live target binding, evidence exclusion, optional/required row behavior and 194 passing tests across 19 targets; retain eight operators and P8 gates |
| 0.2.6b beta | 0.2.7b beta | Implement constrained HQL/IR Expand and optional expansion, synchronize the separate AnnotationPut evidence example/oracle, and record 199 passing tests across 20 targets; keep full Match, extended patterns and P8 gates open |
| 0.2.8b beta | 0.2.9b beta | Record IR Vector parameter decoding and space/dimension mismatch checks; HQL inference, source reads, Knn/Rerank and P8 closure remain open |
| 0.2.9b beta | 0.2.10b beta | Implement structural HQL/IR root Match and deterministic SHORTEST; keep anchors, constrained patterns, six unavailable operators and P8 gates open |
| 0.2.12b beta | 0.2.13b beta | Implement exact KNN and Original Rerank on lease-bound original vectors; record the collection fingerprint contract and 307/0/1 tests across 21 root targets; keep explicit history/change, unsupported operators and P8 gates open |
| 0.2.15b beta | 0.2.16b beta | Implement HQL/typed-IR HistoryScan and ChangeScan over exact retained revisions with source floors, recursive ACL, typed parity and bounded fail-closed behavior; record 9 focused passes and 318/0/1 across 22 HQL2 targets; retain other P8 gates |
| 0.2.17b beta | 0.2.18b beta | Implement strict typed-IR root Match anchors under the P6 snapshot; reject Entity values outside anchor scope; record focused 12/12 + 6/6 and post-anchor 319/0/1 across 22 HQL2 targets; retain remaining HQL2/P8 gates |
| 0.2.21b beta | 0.2.22b beta | Implement checked same-type I64/F64 remainder for HQL `%` and typed-IR `rem`; record 79/79 focused passes and 328/0/1 across 22 HQL2 targets; retain remaining P8 gates |
| 0.2.22b beta | 0.2.23b beta | Add a test-only HQL/typed-IR exact-KNN differential against the independent P7 oracle; verify tie order and missing-original exclusion; retain broad oracle and P8 gates |
| 0.2.23b beta | 0.2.24b beta | Add test-only Original Rerank differential against independent P7 ranking for HQL and typed IR; verify 2/2 differential tests and 330/0/1 across 23 targets; retain broad oracle and P8 gates |
| 0.2.24b beta | 0.2.25b beta | Add test-only HQL root Match differential against the independent P7 graph bag; verify parallel-edge multiplicity, relation filtering, 1/1 focused and 331/0/1 across 24 targets; retain broad oracle and P8 gates |
| 0.2.25b beta | 0.2.26b beta | Freeze delegated Sequence node-ID/conjunctive-label scope, P6 no-probe semantics and fail-closed boundaries; runtime verification pending |
| 0.2.26b beta | 0.2.27b beta | Implement Sequence ID/labels under one P6 snapshot; record 7 focused passes and 338/0/1 across 25 HQL2 targets; retain remaining ACL-hidden fixture, review and P8 gates |

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.2.55b | 2026-10-03 | beta | Record test-only HQL/typed-IR scalar-pipeline P7 differential for six fixtures and 12 Storage executions; focused scalar target 2/2, HQL2 383/0/1 across 33 targets; no runtime/contract/schema/transport change; broad exact-oracle and P8/P13 gates remain open | working-tree | ATHER |
| 0.2.54b | 2026-10-03 | beta | Extend D7 with differential-proven single plain-ASCII label on zero-hop HQL1 node-ID scans; adapter 10/10, HQL2 382/0/1 across 33 targets and separate P6/compatibility 194/0/0; preserve fail-closed remaining forms and broad P8/P13 gates | working-tree | ATHER |
| 0.2.53b | 2026-10-03 | beta | Add test-only HQL/typed-IR Values/UnionAll P7 differential for 169 nullable bag pairs and 338 Storage executions; HQL2 382/0/1 across 33 targets and P6/compatibility 194/0/0; no contract/runtime/schema/transport change; retain broad semantic and acceptance gates | working-tree | ATHER |
| 0.2.52b | 2026-10-03 | beta | Record merged PR #196 hosted status: 10 displayed checks pass, five fail, one skips; worker bootstrap fails across OSes and Windows Rust failure at 15m16 has no retrievable detail; local History/Change 16/16 and HQL2 380/0/1 unchanged; worker approval and P8/P13 remain open | working-tree | ATHER |
| 0.2.51b | 2026-10-03 | beta | Extend test-only P7 ChangeScan differential to Edge/Row/Vector/Annotation for HQL/typed IR; seven events match sequence, operation and subject identity; History/Change 16/16 and HQL2 380/0/1 across 32 targets; broader semantic, hosted/review and P8/P13 gates remain open | working-tree | ATHER |
| 0.2.50b | 2026-10-03 | beta | Add test-only P7 ChangeScan Node-revision differential for HQL/typed IR; History/Change 15/15 and HQL2 379/0/1 across 32 targets; other event source kinds, hosted checks/review and broad P8/P13 remain open | working-tree | ATHER |
| 0.2.49b | 2026-10-03 | beta | Add WAL-derived P7 HistoryScan differentials for all five supported kinds: Node, Edge, Row, Vector and Annotation; History/Change 15/15, Annotation source 7/7, P7 130/130, HQL2 379/0/1, P6/schema-v6/compatibility 194/0/0; Artifact HistoryScan, hosted review/checks and broad P8/P13 remain open | working-tree | ATHER |
| 0.2.48b | 2026-10-03 | beta | Add test-only P7 Vector HistoryScan oracle profile and WAL-derived HQL/typed-IR differential; P7 130/130, History/Change 14/14, HQL2 377/0/1, P6/schema-v6/compatibility 194/0/0; hosted checks/review and broad P8/P13 remain open | working-tree | ATHER |
| 0.2.47b | 2026-10-03 | beta | Implement and verify the narrow ChangeScan ACL/budget contract; ACL 11/11, History/Change 14/14, HQL2 376/0/1 and selected P6/schema-v6 43/0/0; broad P8/P13, transport, hosted CI and review remain open | working-tree | ATHER |
| 0.2.46b | 2026-10-03 | beta | Add HQL/typed-IR Row HistoryScan/ChangeScan parity for retained insert/update revisions and exact property hydration (1/1); refresh explicit HQL2 sweep to 374/0/1 across 32 targets; broad P8/P13, transport, CI and independent-review gates remain open | working-tree | ATHER |
| 0.2.45b | 2026-10-03 | beta | Implement HQL2/IR `tx_as_of` using one no-fallback frontier across scans, operators, hydration and `Snapshot.tx`; record focused 56/56, History/Change 14/14, HQL2 373/0/1 and P6/compatibility 194/0/0; broad P8/P13, transport, CI and review remain open | working-tree | ATHER |
| 0.2.44b | 2026-10-03 | beta | Specify the full tx_as_of runtime acceptance and record the current rejection/current-frontier gap; implementation pending under the accepted P8 contract | working-tree | ATHER |
| 0.2.43b | 2026-10-03 | beta | Implement HQL/typed-IR Vector HistoryScan with compact H2-D11 tuple ID, P6 vector floor and owner-node ACL; record 12/12 focused, 367/0/1 across 31 HQL2 targets and 194/0/0 across 11 P6/compatibility targets; retain tx_as_of and broad acceptance gates | working-tree | ATHER |
| 0.2.42b | 2026-10-03 | beta | Synchronize approved Vector HistoryScan identity, source floor and same-lease owner ACL in P8; implementation and HQL/IR parity verification pending | working-tree | ATHER |
| 0.2.41b | 2026-10-03 | beta | Implement HQL JOIN TABLE lowering to the existing RowScan/Join contract for all four kinds; bare JOIN defaults to Inner; HQL, typed IR and independent P7 match duplicate, SQL-NULL and JSON-null result bags; record 365/0/1 across 31 HQL2 targets; broad oracle, review, shared-runtime, resource/cancellation and P8/P13 gates remain open | working-tree | ATHER |
| 0.2.40b | 2026-10-03 | beta | Add and pass storage-backed typed-IR Join P7 differential for all four kinds, preserving duplicate/NULL/unmatched semantics; record 364/0/1 across 30 HQL2 targets; retain HQL JOIN boundary and broad oracle, review, shared-runtime, resource/cancellation and P8/P13 gates | working-tree | ATHER |
| 0.2.39b | 2026-10-03 | beta | Add and pass storage-backed HQL/typed-IR P7 aggregate differential over 121 nullable bags; record 363/0/1 across 29 HQL2 targets; retain broad oracle, review, shared-runtime, resource/cancellation and P8/P13 gates | working-tree | ATHER |
| 0.2.38b | 2026-10-03 | beta | Add and pass storage-backed HQL/typed-IR P7 scalar differential over 81 nullable bags; record 362/0/1 across 28 HQL2 targets and 190/0/0 across 11 compatibility targets; retain broad oracle, review, shared-runtime, resource/cancellation and P8/P13 gates | working-tree | ATHER |
| 0.2.37b | 2026-10-02 | beta | Verify full locked/offline Rust suite and default/no-default strict all-target Clippy; `probe_vs_recall` remains NOT_RUN; box optional error detail internally with unchanged JSON; full P8/P13/review gates remain open | working-tree | ATHER |
| 0.2.36b | 2026-10-02 | beta | Implement and verify approved H2-D11 R6b markerless WAL recovery; crash 17/17, migration 19/19 and selected 40-target aggregate pass; fixture-only and full P8/P13/review gates remain open | 0135c29 | ATHER |
| 0.2.35b | 2026-10-02 | beta | Implement D7's one-hop endpoint-ID string equality filter after legacy/HQL2 differential; record 9/9 adapter tests and 361/0/1 across 27 HQL2 targets plus 190/0/0 across 11 compatibility targets; retain shared-runtime, independent review and P8/P13 gates | working-tree | ATHER |
| 0.2.34b | 2026-10-02 | beta | Record D7's conditional one-hop endpoint-ID string equality extension after test-only legacy/HQL2 differential; implementation and focused verification pending | working-tree | ATHER |
| 0.2.33b | 2026-10-02 | beta | Extend D7 with differential-proven one-hop HQL1 forms and pre-parse resource reservation; record 8/8 focused tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 separate compatibility targets; retain shared-runtime, independent review and P8/P13 gates | working-tree | ATHER |
| 0.2.32b | 2026-10-02 | beta | Implement D7's actor-scoped HQL1 allowlist; record 5/5 focused tests and 354/0/1 across 27 root HQL2 targets; retain remaining shared-runtime, P8/P13 and independent-review gates | working-tree | ATHER |
| 0.2.31b | 2026-10-02 | beta | Record accepted actor-scoped HQL1 adapter via `query_v2`, exact zero-hop ID projection allowlist and fail-closed contract; differential verification pending; retain full P8/P13 gates | working-tree | ATHER |
| 0.2.30b | 2026-10-02 | beta | Record edge-property-before-SHORTEST regression, 10/10 completion tests, 349/0/1 across 26 root HQL2 targets and separate 190/0/0 across 11 P6/schema-v6/compatibility targets; independent static review found no concrete defect; retain broad P8/P13 gates | working-tree | ATHER |
| 0.2.29b | 2026-10-02 | beta | Implement approved P8 D1-D5; record 9 focused passes, budget/no-partial cases and 528/0/1 across 37 explicit targets; retain independent review and broad P8/P13 acceptance gates | working-tree | ATHER |
| 0.2.28b | 2026-10-02 | beta | Synchronize owner-approved P8 completion addendum D1-D6; authorize bounded text profiles, provenance-preserving ContextPack, Sequence properties and contextual literals; implementation evidence pending | working-tree | ATHER |
| 0.2.27b | 2026-09-30 | beta | Implement HQL/typed-IR Sequence node ID and conjunctive labels from the authorized P6 graph snapshot; record 7 focused passes and 338/0/1 across 25 HQL2 targets; retain ACL-hidden fixture and full P8 gates | working-tree | ATHER |
| 0.2.26b | 2026-09-30 | beta | Record delegated contract for HQL/typed-IR Sequence node ID and conjunctive labels under the P6 snapshot; runtime verification pending; keep node/edge properties and Compact constraints fail-closed | working-tree | ATHER |
| 0.2.25b | 2026-09-30 | beta | Record passing test-only HQL root Match differential against independent P7 graph bag, including parallel edges and relation filtering; 1/1 focused and 331/0/1 across 24 root targets; broad exact-oracle and P8 gates remain open | working-tree | ATHER |
| 0.2.24b | 2026-09-30 | beta | Record passing exact-KNN and Original Rerank HQL/typed-IR differentials against independent P7 ranking; 2/2 tests and 330/0/1 across 23 root targets; broad exact-oracle and P8 gates remain open | working-tree | ATHER |
| 0.2.23b | 2026-09-30 | beta | Record 1/1 test-only exact-KNN differential for HQL and typed IR against independent P7 ranking, including deterministic ties and missing originals; 329/0/1 across 23 root targets; broad exact-oracle and P8 gates remain open | working-tree | ATHER |
| 0.2.18b | 2026-09-30 | beta | Implement exact typed-IR root Match anchors with strict RecordRefV2 parameter/literal validation, no direct lookup, pre-SHORTEST filtering, 12/12 Match plus 6/6 value tests and 319/0/1 across 22 HQL2 targets; retain constraints and P8 gates | working-tree | ATHER |
| 0.2.19b | 2026-09-30 | beta | Implement typed DecimalU64/no-cast binding for structural HQL unsigned parameters; record 50 focused passes and 323/0/1 across 22 HQL2 targets; retain remaining P8 gates | working-tree | ATHER |
| 0.2.20b | 2026-09-30 | beta | Implement HQL2 implicit ORDER BY null placement as NULLS LAST independent of direction, preserve explicit placement and typed-IR contract; record 58 focused passes and 325/0/1 across 22 targets | working-tree | ATHER |
| 0.2.21b | 2026-09-30 | beta | Freeze typed HQL `%`/Query IR `rem` semantics for checked I64/F64 and null propagation; implementation pending | working-tree | ATHER |
| 0.2.22b | 2026-09-30 | beta | Implement checked HQL `%` and typed-IR `rem` for same-type I64/F64, NULL propagation, zero-divisor errors and integer overflow; record 79/79 focused and 328/0/1 across 22 targets; retain remaining P8 gates | working-tree | ATHER |
| 0.2.17b | 2026-09-30 | beta | Freeze root Match anchor semantics as full non-null RecordRefV2 identity predicates scoped to the pinned P6 snapshot; require no-probe unmatched behavior and apply anchors before SHORTEST; runtime pending focused verification | working-tree | ATHER |
| 0.2.16b | 2026-09-30 | beta | Implement HistoryScan/ChangeScan with exact revision hydration, tombstones, exclusive floors, recursive reference ACL and no-partial budget failures; record 318/0/1 across 22 HQL2 targets; retain LexicalMatch, ContextPack, constrained patterns and P8 gates | working-tree | ATHER |
| 0.2.15b | 2026-09-29 | beta | Clarify exclusive ChangeScan floor boundary so migration baselines are history, not fabricated feed events; runtime and P8 gates remain open | working-tree | ATHER |
| 0.2.14b | 2026-09-29 | beta | Freeze delegated HistoryScan/ChangeScan contracts over durable revisions, including cursor floors and typed results; runtime remains open | working-tree | ATHER |
| 0.2.13b | 2026-09-29 | beta | Implement exact HQL/IR KNN and Original Rerank on P6 lease-bound original vectors; record 307 passed, 0 failed and 1 ignored across 21 root HQL2 targets; retain remaining P8 gates | working-tree | ATHER |
| 0.2.12b | 2026-09-29 | beta | Fix RowScan after_image property hydration without changing UUIDv4 entity identity; 21 root HQL2 targets reported 296 passed, three vector operator failures and one ignored | working-tree | ATHER |
| 0.2.11b | 2026-09-29 | beta | Record RowScan property-hydration failure and after_image root cause; preserve UUIDv4 row identity and approval gate | working-tree | ATHER |
| 0.2.10b | 2026-09-29 | beta | Implement structural HQL/IR root Match and deterministic SHORTEST; fail closed for anchors and constrained patterns, retain six operator and P8 gates | working-tree | ATHER |
| 0.2.9b | 2026-09-29 | beta | Record IR-declared Vector parameter decoding and fail-closed space/dimension checks; keep source reads, HQL inference, Knn/Rerank and P8 gates open | working-tree | ATHER |
| 0.2.8b | 2026-09-29 | beta | Implement structural HQL/IR Sequence Expand under the P6 graph lease; record 206 passes across 21 targets and retain pattern-constraint/P8 gates | working-tree | ATHER |
| 0.2.7b | 2026-09-29 | beta | Record constrained HQL/IR Expand semantics, separate AnnotationPut evidence fixture/oracle, and 199 passing tests across 20 targets; P8 remains partial | working-tree | ATHER |
| 0.2.6b | 2026-09-29 | beta | Implement and verify HQL/IR AnnotationLookup for frozen/live targets, evidence exclusion and optional/required behavior; 194 passed across 19 targets, P8 remains partial | working-tree | ATHER |
| 0.2.5b | 2026-09-29 | beta | Record typed per-operator field hydration, dynamic names, aligned stable row keys and 188 selected passing tests; retain remaining source/operator and P8 gates | working-tree | ATHER |
| 0.2.4b | 2026-09-29 | beta | Record four lease-bound source scans, annotation reference authorization, budgeted payload access, typed entity properties and LEFT JOIN nullability; retain selective typed hydration and full P8 gates | working-tree | ATHER |
| 0.2.3b | 2026-09-29 | beta | Record annotation write/preflight and ACL policy-event v2 evidence; annotation reads and P8 source adapters remain open | working-tree | ATHER |
| 0.2.2b | 2026-09-29 | beta | Record vector revision write/projection/reopen and fail-closed ingress tests; source adapters and P8 remain open | working-tree | ATHER |
| 0.2.1b | 2026-09-29 | beta | Record 19/19 fixture-verified schema-v6 migration/recovery tests without enabling HQL2 source adapters or transport surfaces | working-tree | ATHER |
| 0.1.0b | 2026-09-28 | candidate | Freeze concrete choices for review before P8 production source | working-tree | ATHER |
| 0.1.1b | 2026-09-28 | candidate | Address independent review with grant/lifetime/locking contracts, ReadView methods, request policy matrix and exact result/type/error definitions | working-tree | ATHER |
| 0.1.1b | 2026-09-28 | beta | Owner approved with "approve"; begin implementation and verification without claiming P8 completion | working-tree | ATHER |
| 0.1.2b | 2026-09-28 | beta | Truth-sync scalar/core implementation limits and distinguish remaining storage and surface gates | working-tree | ATHER |
| 0.2.0b | 2026-09-28 | beta | Truth-sync required database_id and UUIDv4 RecordRef identity plus approved H2-D11 revision, annotation, history and schema-v6 dependencies | working-tree | ATHER |
