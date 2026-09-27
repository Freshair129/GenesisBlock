---
doc_id: SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY
version: "0.1.2b"
created_at: "2026-09-28T01:25:00+07:00,ATHER,fc851e9"
last_update: "2026-09-28T04:30:00+07:00,ATHER"
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
This approved contract freezes the exact interface/config/result decisions that the
ADR requires before P8 source changes. It is not evidence of an implemented
parser, binder, storage adapter or endpoint. No engine version is changed.

Parent: [Master specification](MASTER-SPEC--GENESIS-DB.md).
Peers: [P6 read contract](SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md),
[G0 structural contracts](SPEC--GENESISDB-UEE-HQL2-G0-CONTRACTS.md),
[legacy typed IR](SPEC--GENESISDB-TYPED-QUERY-IR-V1.md), and
[compatibility HQL](SPEC--HQL-V2.md).

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
3. Add typed, authorized source enumeration under a P6 ReadView, with explicit
   capability errors for unavailable history/annotations/original vectors.
   The existing seven ReadView operations do not supply this contract.
   This requires a peer amendment to P6; it is not permission to expose Storage.
4. Preserve current default-namespace ownership and disk formats. Do not invent
   model fingerprints, revision identities or historical annotation data from
   current rows. The full storage-backed operator gate remains open wherever
   a source contract is missing. Durable annotation/revision extensions and
   user database migration are separate reviewed changes under ADR D11.

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
                                      `-> one P6 lease -> typed result
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
| HistoryScan | 0 | `kind: EntityKind, id: Expr, as: Symbol` |
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

`Pattern` has closed Compact and Sequence variants exactly matching the schema.
Compact: start/end/optional edge aliases, relations, direction, min/max hops,
mode and optional path alias. Sequence: start node, ordered edge/node steps,
mode and optional path alias. Inline conditions belong inside optional patterns.
Bounds are checked before execution, even on empty input. Shortest is bounded,
unweighted, one deterministic path per bound endpoint pair; no weighted mode.

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
when applicable. Stages: contract, parse, bind, authorize, plan, execute, encode.
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

`RecordRefV2 = { namespace: String, kind: EntityKind, id: String,
revision: String }`; revision is the authoritative version identity, never
inferred from a wall clock or a hash of current payload. `SourceKeyV2` is an
opaque engine-created ordered tuple `(namespace,kind,id,revision)` bound to
the lease and source; callers cannot supply a trusted cursor from JSON.
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

Initial source matrix: Values has no storage dependency; current Node/Edge/Row
scans are enabled only after their identity/schema/ACL adapters pass differential
tests; Annotation/History/Change/lexical scans and historical vectors remain
CAPABILITY_UNSUPPORTED until each enumerator can prove its declared retained
domain at S,V. This is an implementation dependency matrix, **not a reduction
of final HQL2 acceptance**. Unsupported sources fail during binding, before pin.
Adding these methods amends the P6 seven-operation whitelist only as explicitly
described here. They never expose unscoped Storage or widen grant semantics.

### Request normalization and initial policy matrix

| Request field | Initial v2 behavior |
|---|---|
| language | explicit hql.v2, or query-ir.v2; hql.v1 returns CAPABILITY_UNSUPPORTED until legacy-equivalent lowering is verified; existing v1 endpoint remains unchanged |
| explain | omitted means None; textual EXPLAIN/ANALYZE supplies the mode if the envelope is omitted; if both are supplied they must agree, otherwise BIND_ERROR; no mode silently upgrades plan-only to execution |
| namespace/temporal | textual/envelope values must agree; omitted valid_at is one UTC instant captured under the query guard; omitted transaction selector is the pinned data frontier |
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

Initial exact type spellings are `Bool`, `I64`, `F64Finite`, `Utf8`,
`Nullable<T>`, `List<T>`, and `Vector<space_id,dim,f32|f64>` (no whitespace).
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
unsupported, not interpreted as arbitrary JSON. Untrusted Entity/Path/Score/
Context parameter construction is unsupported; those types are engine-issued.

`QueryValueV2` variants: Null, Bool(bool), I64(i64), F64(f64 finite),
Utf8(String), List(Vec<QueryValueV2>), Entity(RecordRefV2),
Path{vertices:Vec<RecordRefV2>,edges:Vec<RecordRefV2>,mode:PathMode},
Vector(OriginalVectorV2), Score{value:f64,owner:RecordRefV2,source:String,
scope:WholeInputOrCandidates,metric:String}, Context(ContextPackageV2),
Json(serde_json::Value) only for explicit declared JSON-property payloads.
The boundary encodes these as closed tagged objects `{type,value}`; user JSON
properties stay explicitly tagged JSON and cannot impersonate domain values.

Initial scalar functions: `has_prop(Entity,Utf8)->Bool`,
`prop(Entity,Utf8)->Nullable<Json>`, `lower(Utf8)->Utf8`,
`length(Utf8)->I64` (Unicode scalar count). JSON numeric extraction requires an
explicit registered checked cast before numeric operations; no cast registry
entry means CAPABILITY_UNSUPPORTED. Arithmetic uses same-type checked I64 or
finite F64. Boolean operators accept Bool/Nullable<Bool>; comparisons require
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
Offset, UnionAll, Aggregate and Join. All 13 other operators are explicitly
unavailable at binding pending source/revision/space/tokenizer qualification.
HQL lowering is also partial: untyped NULL/list/object literals, parameterized
limits, implicit null ordering, remainder, quoted non-ASCII symbols and the
non-scalar stages reject explicitly rather than inventing semantics. Typed IR
nullable/list payloads and explicit scalar ordering execute. These are open
implementation tasks, not amendments reducing final acceptance.

Only the catalog ReadView extension is implemented in this checkpoint; scan,
hydrate and vectors remain gated. No REST, NAPI, FFI, SDK or MCP adapter is
added. New query execution may invoke existing P6 generation publication;
plan-only EXPLAIN cannot. SQLite's volatile shared-memory read marks are not
durable query effects; tests separately compare signed WAL and snapshot bytes.

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
- Existing HQL/v1 tests pass unchanged. Unproven legacy lowering remains on the
  compatibility path and the shared-runtime completion obligation stays open.
- Independent implementation review and verification must pass before P8 closure.
  Parser/binder coverage alone does not close the storage-backed runtime gate.

## Version diff and changelog

| From | To | Effect |
|---|---|---|
| none | 0.1.1b candidate | Proposed typed P8 API/config/result boundary, concrete policy/type mappings and P6 catalog/read extensions; no implementation approval inferred |
| 0.1.1b candidate | 0.1.1b beta | Owner approved the unchanged contract and isolated upstream integration; no push, PR merge, deployment or user database migration authorized |
| 0.1.1b beta | 0.1.2b beta | Record the partial implementation matrix and evidence link without narrowing the approved target |

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-28 | candidate | Freeze concrete choices for review before P8 production source | working-tree | ATHER |
| 0.1.1b | 2026-09-28 | candidate | Address independent review with grant/lifetime/locking contracts, ReadView methods, request policy matrix and exact result/type/error definitions | working-tree | ATHER |
| 0.1.1b | 2026-09-28 | beta | Owner approved with "approve"; begin implementation and verification without claiming P8 completion | working-tree | ATHER |
| 0.1.2b | 2026-09-28 | beta | Truth-sync scalar/core implementation limits and distinguish remaining storage and surface gates | working-tree | ATHER |
