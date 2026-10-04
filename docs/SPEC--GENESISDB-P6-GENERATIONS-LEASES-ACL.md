---
doc_id: SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL
owner: GenesisBlockDB Engineering
version: 0.5.33b
created_at: "2026-09-22T22:55:00+07:00,ATHER,working-tree"
last_update: "2026-10-04T15:03:58+07:00,ATHER"
status: beta
attributes:
  domain: storage-correctness
  scope: p6-generation-lease-temporal-acl
  approval: owner-approved-contract-and-architecture-correction
  risk: HIGH
  complexity: C-3
---

# P6 — Durable generations, read leases, temporal binding and ACL

This specification records the owner-approved P6 contract and Astra architecture correction
for the existing Rust storage core. It follows the Wave A serialized commit/read boundary and
Wave B journal/snapshot compatibility model. Completion remains subject to Verify, independent
Review and Final gates. This embedded-core change does not claim MVCC, transport authentication,
tenant isolation for records without namespace metadata, or deployment readiness.

## Requirements

1. WHEN a generation is published THEN serialize writers, flush HNSW, write a no-fold snapshot,
   validate its component manifest, then append a signed receipt to the durable WAL.
2. IF any required component write, replacement, manifest, byte-count or digest check fails THEN
   fail the candidate without replacing the authoritative snapshot marker or publishing a receipt.
3. WHEN recovery detects P6 snapshot integrity failure THEN discard all candidate snapshot state
   and replay the complete WAL; never retain a partial snapshot load.
4. WHEN a lease is issued THEN bind a published generation, owner token, monotonic expiry, fence,
   access context and temporal selectors.
5. IF owner, expiry, revocation, generation or ACL revision is invalid THEN reject without live fallback.
6. IF ACL is Enforced THEN raw unscoped Storage reads fail closed; scoped reads require a grant.
7. WHEN policy changes THEN use expected-revision CAS and a signed, versioned WAL event that
   survives fold/snapshot/replay and revokes leases pinned to an older policy revision.
8. IF schema or signed event is unsupported THEN refuse open rather than skip enforcement state.
9. IF a temporal selector is unsupported or conflicts with a request selector THEN return an
   explicit error rather than silently using current state.
10. WHEN compaction replaces journal history THEN include the current published generation and
    non-default access policy as signed folded materializations. During replay, verify signatures on
    GenerationPublished and AccessPolicyChanged before application; an invalid or unknown signature
    makes the handle recovery-required and denies subsequent operations.
11. WHEN a P6 snapshot is instant-loaded THEN verify the snapshot's generation and access-policy
    materializations against the latest signature-verified matching WAL events at or before its
    frontier. A mismatch rejects the snapshot and recovers from the complete WAL; a non-default
    policy without verified WAL provenance makes recovery fail closed. This check includes legacy
    JSONL, folded/base and active WAL sources.
12. WHEN an Enforced caller invokes a raw HQL read entrypoint THEN return
    ACCESS_CONTEXT_REQUIRED before parsing or validating the query.
13. WHEN `state.json` is absent THEN preflight WAL schema authority before replay; select
    schema 6 only from a valid signed local `Schema6ActivationV1` and its complete proof,
    otherwise preserve schema-5-only replay or fail with `RECOVERY_REQUIRED` on v6 evidence.

## Architecture

P6 composes with the Wave A commit boundary. Publication is a durable statement about a fully
applied frontier, not a copy of the current frontier counter.

    Writer -> Commit boundary -> Durable WAL mutation -> Projection/graph/vector apply
      -> release writer boundary
    Read pin -> Commit boundary -> HNSW flush -> no-fold snapshot -> component-manifest validation
      -> signed GenerationPublished WAL event -> opaque ReadLease
    ReadLease -> validate owner/expiry/fence/generation/ACL -> scoped ReadView -> authorized result

The snapshot covers the data frontier before its receipt. The receipt is at
publication_seq = wal_frontier + 1 and does not mutate graph, vector or relational user data.
All publication steps execute under the existing commit/read boundary. A later write advances
the frontier; a new pin publishes a new generation and no reader silently moves to live state.

Cold-open schema selection adds a separate authority gate before projection replay:

    state marker missing -> WAL framing/signature preflight -> activation + linked proof
      -> select schema -> replay all WAL -> validate/rebuild projection -> expose Storage
    v6-only event without valid activation OR conflicting proof -> RECOVERY_REQUIRED

## Public Rust contract

### Generation and lease

- GenerationInfo fields: generation_id, wal_frontier, publication_seq, txn_frontier,
  history_horizon, acl_revision and component_manifest_sha256.
- publish_generation returns Result<GenerationInfo>, flushes HNSW, persists and validates a
  no-fold snapshot, then appends the signed GenerationPublished event. Reuse is valid only while
  the stable WAL frontier equals the recorded publication sequence.
- ReadLease is opaque outside the engine. It binds generation, storage owner, monotonic expiry,
  storage fencing epoch, access context, temporal selectors and lease id.
- pin_generation(access, temporal, ttl) publishes lazily when no matching generation exists.
- validate_lease reports owner mismatch, expiry, revocation, stale generation or ACL revision
  change using the stable error prefixes in the Errors section.
- revoke_lease advances the storage-wide fencing epoch, invalidating every outstanding lease for
  that Storage handle.
- with_read_lease holds the commit boundary through the callback, validates before and after it,
  and has no live-state fallback. ReadView exposes only the operations listed below.
- Storage::open generates a new owner token. Leases are not persisted and do not survive reopen.
  A non-positive TTL is rejected.

### Temporal selectors

TemporalRead is { as_of: Option<String>, tx_as_of: Option<u64> }.

- as_of must parse as RFC3339 and binds to valid_at where the operation supports it.
- tx_as_of must satisfy generation history_horizon <= tx_as_of <= the pinned generation's
  WAL frontier. A below-horizon selector returns TEMPORAL_BEYOND_HORIZON; a future selector is
  rejected before source access.
- The P6 history_horizon governs retained node/edge history. H2-D11 adds per-source
  row_history_floor, annotation_history_floor and vector_history_floor. A read below its selected
  source floor returns HISTORY_UNAVAILABLE before source access; it never substitutes current data.
- HQL2/typed-IR `Storage::query_v2` selects transaction frontier S as `tx_as_of` when supplied,
  otherwise the pinned generation frontier; every revision-backed source, property hydration,
  graph/vector/annotation operator and result `Snapshot.tx` must use this same S. The catalog,
  policy revision and lease remain bound to the validated generation; there is no current-state
  fallback. HQL2 HistoryScan uses S as its transaction upper bound and the selected valid instant
  as its valid-time point. It enumerates retained revisions (including retractions) without
  applying `tx_to` as a current-visibility filter, but enforces the relevant source floor first.
  Supported sources are Node/Edge (graph floor), Row (row floor), Vector (vector floor) and
  Annotation (annotation floor). A Vector logical ID is the compact JSON encoding of the ordered
  `(owner_id, collection_id)` pair used by H2-D11; the exact current P6 ACL is checked through its
  owner node under the same lease. Artifact history remains unsupported until a durable artifact
  source and floor are approved.
- HQL2 ChangeScan uses an exclusive `after_seq` and selected frontier S as its inclusive upper bound.
  It fails before source access if the cursor is below any participating graph/row/vector/annotation
  floor; equality is valid and only events strictly after the floor are returned. Migration
  baselines at `F` are available through HistoryScan at `F`, not synthesized into a ChangeScan
  whose cursor is `F`. Missing floors or unsupported revision kinds fail closed. It applies the current lease ACL
  to each exact subject, including recursive Edge endpoints, Vector owner, and Annotation
  target/evidence references. It returns no partial feed on budget exhaustion.
- Query IR inherits the same pinned `valid_at` and selected transaction frontier as HQL2; conflicting
  request selectors fail.
- Neighbors and hybrid search may inherit as_of but reject tx_as_of for P6.
- The legacy `execute_hql` and relational query operations, `node_view` and `node_versions` still
  reject temporal leases unless their own operation contracts support those selectors. This does
  not prohibit the separate P8 HQL2/typed-IR `Storage::query_v2` binding above.

### Scoped read operations

The original P6 ReadView operations are node_view(id), node_versions(id, at_seq), neighbors(seed, args,
is_inferred), hybrid_search(args), execute_query_ir(request), execute_hql(query), and
query_relational(query). Each returns the existing result type wrapped in Result.

The owner-approved [P8 boundary](SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY.md)
adds crate-private typed catalog/scan/hydrate/vector operations without exposing
Storage. `hql2_catalog`, revision-bound Node/Edge/Row/Annotation scans, and the
current internal property-hydration path are implemented under the continuous
commit guard. Exact KNN and Original Rerank also read original vector revisions
through `hql2_vectors` under that same lease. HistoryScan/ChangeScan execute on
the same lease and source-floor boundary over exact retained schema-v6
revisions. Vector HistoryScan uses the H2-D11 compact JSON `(owner_id,
collection_id)` identity, vector source floor and existing owner-node ACL under
the same lease. Storage-backed HQL/typed-IR HistoryScan bags for Node, Edge,
Row, Vector and Annotation now match independent P7 references assembled from
WAL revision facts and captured frontiers/valid-time windows. The HistoryScan/
ChangeScan target passes 17/17, including five exclusive-after/inclusive-through
P7 windows and selected transaction frontiers; Annotation source passes 7/7,
the 35-target HQL2 regression sweep passes 394/0/1, and the separate 11-target
P6/schema-v6/compatibility group passes 194/0/0. Broader
P6/P8 acceptance, transport parity and independent review remain open. Explicit
HQL2/IR `tx_as_of` is specified above and its cross-source
runtime path now selects one frontier S across source scans, operators,
hydration and result metadata while retaining the pinned P6 generation and
current ACL. Five focused HQL2 targets pass 56/56, the 35-target HQL2 sweep
passes 394/0/1 and the earlier separate 11-target P6/schema-v6/compatibility sweep
passes 194/0/0. Broader P6/P8 acceptance, transport parity and independent
review remain open. Property access uses
binder-issued `FieldIdV2` and aligned `ExecBatchV2` batches, including
row-dependent names, without retaining full payloads in query rows. Source
cursors are opaque and bound to one lease, source and database. Row scans use
the authorized catalog.
Annotation scans check the Annotation grant and every normalized target/evidence
reference at the same frontier and valid time; missing or unauthorized
references hide the whole annotation. Hydration rechecks annotation references
before loading its body and reserves payload budget before reading JSON. Plan-only
catalog access does not pin or publish a generation. Executed queries pin one
lease and validate it after execution and before returning the encoded boundary
result. Existing seven operations and their grant semantics are unchanged.

For HQL2 Sequence ID/label/property predicates, the internal graph snapshot
uses exact authorized node/edge revisions at the pinned frontier and valid
time. It never looks up a requested ID directly or widens the namespace grant.
Node/edge properties are selectively hydrated only after P6 visibility and
their candidate work is charged to the query budget; equality is exact JSON,
missing values do not match, and filters run before SHORTEST. Optional Expand
preserves its input and null-extends new aliases. This remains a crate-private
P6-bound query capability, not a public point-read operation. The D1-D4 focused
P8 target passes 9/9; the explicit 26-root-HQL2 sweep passes 348/0/1.

The view carries its engine-validated lease, access context and selectors, but never exposes
Storage. In Enforced mode Result-returning direct data reads fail with ACCESS_CONTEXT_REQUIRED;
legacy Option/Vec data accessors fail closed with None/empty where their existing return type has no
error channel. ReadView authorizes then dispatches through engine-private read primitives. A
separately captured Storage reference does not inherit ReadView authorization.

The fail-closed legacy accessors are `get_u32`, `find_fuzzy_id`, `node_view` and `node_view_u32`
(None), plus `list_collections` and `get_meta_history` (empty). Operational telemetry/frontier and
capability metadata are outside the record-read ACL. Transport synchronization APIs are also outside
this read boundary and require a separate peer-authorization contract.

`execute_hql_read_only` and `execute_hql_read_only_with_budget` are raw read boundaries. Their ACL
guard precedes HQL parsing so an Enforced caller cannot distinguish malformed, unsupported or
supported read syntax without an AccessContext.

## Access policy contract

- AccessContext is { principal: String, namespace: String }.
- AccessPolicy contains revision, Disabled/Enforced mode and a list of AccessGrant values.
- AccessGrant is an exact principal/action/resource triple.
- AccessAction variants are Read, Write and ManagePolicy. P6 enforces Read and policy
  administration only; write authorization is out of scope.
- AccessResource variants are Namespace, Node, Edge, Collection, Table(namespace, table) and,
  under approved H2-D11, Annotation(namespace). The serialized policy-event schema is versioned;
  readers that do not understand Annotation reject the newer schema/event before replay.
- PolicyAdminActor wraps AccessContext.
- replace_access_policy(actor, expected_revision, policy) succeeds only when persisted revision
  matches and new revision is exactly expected_revision + 1. Disabled bootstrap requires
  principal local-owner; in Enforced mode the actor needs ManagePolicy on its namespace.
  Accepted changes are signed, versioned and journaled.
- Policy replacement invalidates leases pinned to the prior ACL revision. Policy state is
  restored by WAL replay or a validated snapshot/fold. An instant-loaded snapshot is accepted only
  when its exact policy matches the latest signature-verified AccessPolicyChanged materialization
  covered by the snapshot frontier (or the revision-0 Disabled default when no such event exists).
- The Rust in-process caller is trusted to assert the actor. REST/N-API/FFI identity binding and
  API-key middleware integration are not part of P6.

Existing graph, edge and vector records have no namespace field. Their P6 logical namespace is
default; P6 does not migrate entity ownership metadata. Authorization is:

| Operation | Required grant |
|---|---|
| Point node read | Exact Node(id) Read or Namespace(namespace) Read |
| Relational query | Namespace(namespace) Read, or Read grants for base and every joined table |
| Annotation read | Read(Annotation(namespace)) and Read access to every target/evidence resource in the same lease; hide the whole annotation if any reference is unauthorized |
| Neighbors, HQL, Query IR and retrieval-composed reads | Namespace(default) Read |
| Hybrid search | Namespace(default) Read; Collection alone is insufficient because results include nodes |
| Unsupported namespace/resource combination | Deny; never widen scope implicitly |

Disabled preserves existing direct-read compatibility. Enforced denies raw unscoped reads. No
claim is made that exact Edge or Collection grants provide a query surface beyond this matrix.
The current HQL2 query boundary additionally requires the existing namespace-wide
query grant. That grant remains broad for same-namespace target/evidence
references, but it does not replace `Annotation(namespace)` Read when an
annotation itself is the source or ChangeScan subject. HQL2 does not yet expose
a query path authorized only by exact per-record grants.

The owner-approved P8 completion addendum preserves this grant and the
authorization-before-parse order. Its ACL fixture asserts that an actor with
only an exact Node(Read) grant receives the same `FORBIDDEN/authorize` result
for malformed and valid HQL/IR requests before parsing; it does not assert
that an existing hidden ID and absent ID both return `NO_MATCH`. The focused
ACL target passes 9/9. The explicit 37-target HQL2/P6/schema-v6/compatibility
sweep passes 528/0/1. P6 authorization remains lease- and namespace-bound; no
exact-grant-only query surface or transport parity is added, and broader
P6/P8 acceptance remains open.

H2-D11 implementation checkpoint: `Annotation(namespace)` is accepted by policy
validation and signed policy-event schema v2; the version is retained across
replay, snapshot validation and compaction. P8 AnnotationScan, lookup, history,
hydration and ChangeScan subject visibility require that explicit grant in the
same ReadView, in addition to the query's namespace grant. Target/evidence
references are checked recursively; the broad namespace grant still covers
same-namespace references. A regression verifies Namespace-only denial and
combined-grant success for AnnotationScan and ChangeScan. It does not claim an
exact-grant-only HQL query surface. Write authorization remains out of scope.
ChangeScan budget policy addendum: namespace-only actors retain access to other
readable ChangeScan subjects, while Annotation subjects still require explicit
Annotation(Read). When that grant is absent, Annotation revisions must be
excluded before candidate counts/bytes are charged to caller-selected budgets or
materialized. This does not widen authorization to Annotation and does not claim
timing noninterference. The hidden-revision threshold regression passes with
ACL 11/11; the expanded History/Change target passes 17/17, Annotation
source/ACL passes 7/7, HQL2 passes 394/0/1 across 35 targets and the separate P6/schema-v6/
compatibility group passes 194/0/0. Hosted validation of this source change,
independent review and broader P6/P8 acceptance remain open.

P8 AnnotationLookup also runs inside the same ReadView and applies the same
target/evidence authorization check before returning an annotation reference.
Lookup matches annotation targets only (never evidence); frozen targets match
their exact revision while live targets resolve by logical identity at pinned
S,V. Optional lookup retains an unmatched input with typed Null; required
lookup drops it. Duplicate input paths retain multiplicity.

Exact KNN and Original Rerank use the same lease-bound vector read. Before
loading payloads, the ReadView checks the namespace grant, exact Node read
authorization, database/namespace/owner revision and temporal visibility at
the pinned WAL frontier and valid time. It loads only the schema-v6 original
vector revision whose owner revision, collection and H2-D11 collection-space
fingerprint agree. HNSW, quantized data and sidecars do not satisfy this read.
KNN omits candidates without originals; Original Rerank fails closed if any
candidate is missing one. Existing query authorization still requires the
namespace-wide query grant; this does not enable exact-grant-only query access
or a history-enumeration surface.

## Snapshot integrity and compatibility

1. The existing P6 reader transition from schema 4 to 5 remains supported, including v4 snapshots
   without a P6 manifest through the legacy load/replay path. The owner-approved H2-D11 target
   advances schema 5 to 6. A v6 reader may open a v5 database for legacy APIs without auto-
   migrating it; HQL2 storage capabilities remain unavailable until explicit migration. Older
   readers reject schema 6 before replay or exposing data.
2. A P6 manifest records its version, snapshot frontier and sorted { path, bytes, sha256 } entries
   for every required storage component except state.json. A canonical sorted manifest digest is
   exposed by GenerationInfo.
3. The manifest covers graph and retirement artifacts, projection SQLite, and configured
   collection vector/metadata/rerank/calibration sidecars required by the collection manifest.
   Safe relative names and the required-file set are checked.
4. Recovery validates safe paths, required entries, frontier agreement, byte lengths and SHA-256
   digests before mutating memory. On any failure, discard all candidate snapshot state and replay
   the complete WAL; partial snapshot state is never retained.
5. Every required component write and rename error fails the snapshot. state.json remains the
   final commit marker and is replaced last. A crash before marker replacement leaves the prior
   snapshot authoritative or causes complete WAL replay.
6. New-reader preflight rejects schema above its supported version. A schema-6 header with
   upgrade_state=in_progress keeps normal reads/writes closed; recovery resumes that migration or
   returns RECOVERY_REQUIRED. The migration writes its marker before v6-only journal events, then
   appends idempotent baseline chunks and rebuilds projections. Snapshot publication keeps the
   marker in_progress through append and verification of the signed GenerationPublished receipt;
   only then does the final marker become ready. The receipt covers the final migration-commit
   frame (`publication_seq = wal_frontier + 1`) and the verified post-migration snapshot digest.
   Every ordinary reopen independently verifies marker identity/manifest, complete chunks and
   commit digest, and receipt; state.json alone is not authority. Compaction preserves or re-emits
   the signed migration authority records so fold followed by WAL-only cold reopen reconstructs
   identical revision/registry state. Migration events are verified as local-signer authority on
   replay, excluded from peer delta/anti-entropy export, and recursively rejected at peer and
   application ingress, including nested batches. Unknown complete signed control events fail
   closed. The repo's committed baseline is schema 5 and the migration remains schema-5-to-6; any
   pre-release schema-6 build is outside the rollback compatibility matrix. Exact envelope and
   source-coordinate details are in ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS R6a v0.4.0b
   Owner approval of ADR R6a v0.4.0b was received 2026-09-29. The migration and
   recovery path passes 19 dedicated temporary-fixture tests; no existing user
   database has been migrated, and this evidence is not release qualification.
7. state.json is the final commit marker and is not included in the component digest, so its P6
   fields require independent authority validation. Before instant load, scan all journal sources
   through the snapshot frontier, verify signatures on covered P6 events, and require the stored
   generation and access policy to match their latest verified WAL materializations. A mismatch
   rejects the whole snapshot and triggers complete WAL replay. If the snapshot asserts a
   non-default policy but the complete journal has no verified policy event, recovery is required;
   never reopen with the Disabled default. Legacy JSONL is part of this verification path.
8. A missing/unreadable `state.json` does not imply schema 5. Before replay or projection
   mutation, preflight the available WAL sources and verify the signed, local-only
   `Schema6ActivationV1` against the database identity. A fresh activation is written before
   schema-6 operations are exposed. A migration activation binds the complete migration chunk
   set/commit, source identity/frontier/floors and matching signed generation receipt. If valid,
   rebuild using schema 6. If no activation exists and all events are schema-5-compatible, replay
   as schema 5. Any v6-only event without valid activation, incomplete/tampered proof, unknown
   control event or ambiguous empty non-fresh directory returns `RECOVERY_REQUIRED`, never a
   schema downgrade or partial view. Existing torn-tail handling may discard only the final
   incomplete frame; it does not waive activation proof. Fold retains/re-emits activation and
   linked migration proof. The contract is in H2-D11 ADR R6b; no user database migration is
   authorized by it.

## Failure behavior and recovery

| Condition | Required behavior |
|---|---|
| HNSW flush or component snapshot error | Fail publication; do not append GenerationPublished |
| Invalid or incomplete P6 manifest | Reject snapshot as a unit and replay complete WAL |
| P6 snapshot fields disagree with signed WAL materialization | Reject snapshot and replay complete WAL |
| Non-default snapshot ACL has no verified WAL provenance | Return RECOVERY_REQUIRED; never default to Disabled |
| Publication event append is uncertain | Follow Wave A recovery-required/reopen behavior; do not claim publication |
| Lease owner, expiry, fence or generation invalid | Return an explicit error; never switch to live state |
| ACL revision changes after pin | Revoke the old-revision lease and deny its next use |
| Policy expected revision mismatch | Return ACCESS_POLICY_REVISION_CONFLICT; append no event |
| Schema/event unsupported | Refuse open before partial read or policy bypass |
| H2-D11 migration is in progress or incomplete | Block all normal reads/writes; resume the same migration or return RECOVERY_REQUIRED |
| Markerless WAL has v6-only events without valid activation proof | Return RECOVERY_REQUIRED before projection replay; never select schema 5 |

## Stable error prefixes

GENERATION_STALE, LEASE_OWNER_MISMATCH, LEASE_EXPIRED, LEASE_REVOKED,
TEMPORAL_BEYOND_HORIZON, HISTORY_UNAVAILABLE, ACCESS_CONTEXT_REQUIRED, ACCESS_DENIED,
ACCESS_POLICY_REVISION_CONFLICT, SNAPSHOT_MANIFEST_INVALID and SNAPSHOT_COMPONENT_INVALID.

## Scope, risk and acceptance

Complexity is C-3 and risk is HIGH because P6 changes durable event compatibility, recovery,
snapshot atomicity and the core read boundary. Only the serialized src/lib.rs owner may implement
source changes. The three RED tests have disjoint paths and may be authored in parallel; they do
not permit parallel source edits.

Out of scope: REST/N-API/FFI/SDK contract changes, write authorization, transport identity,
namespace/entity migration, ACL group syntax, retention changes, MVCC, backup/restore redesign,
P7 oracle/planner/HQL2, deployment, merge and unrelated refactoring.

Acceptance commands:

    cargo test --no-default-features --test p6_generation_tests --test p6_lease_tests --test p6_visibility_tests --test wave_a_commit_tests --test temporal_queries_tests --test tx_as_of_wp22_tests --test governance_tests
    cargo fmt --check
    cargo check --no-default-features
    npm run docs:validate
    git diff --check

Verify records exact local results. Review is independent and read-only; source findings return
to the serialized owner and require Verify to rerun. Final checks scope, evidence categories,
regressions and WIP preservation. Passing the gates supports only the P6 owner decision; it does
not approve P7, merge, release, deployment or external readiness.

The D7 actor-scoped HQL1 adapter accepts one-hop ordering only by the projected
endpoint ID and an exact same-alias string ID equality predicate on labeled or
unlabeled zero-hop scans, each after legacy/HQL2 differential evidence. Focused
adapter, ordering and labeled-filter targets pass 11/11, 2/2 and 2/2; the latter
includes match, miss, wrong-label exclusion and a backslash/Unicode ID. The
explicit HQL2 sweep passes 394/0/1 across 35 targets. The selected P6 peer group
passes 45/0/0 across seven targets; the prior broader 11-target P6/schema-v6/
compatibility result remains 194/0/0 and was not rerun here. No P6 grant, ACL,
lease, schema or migration behavior changed. Broad P6/P8 and review gates remain
open.

## CHANGELOG

Version diff 0.5.32b -> 0.5.33b: synchronize the conditional D7 extension for
one exact same-alias string ID predicate on labeled zero-hop scans. The
legacy/HQL2 differential covers match, miss, wrong-label exclusion and
backslash/Unicode handling; focused adapter/order/labeled-filter targets pass
11/11, 2/2 and 2/2; HQL2 passes 394/0/1 across 35 targets. Selected P6 peer
tests pass 45/0/0 across seven targets. PR #212 baseline reports 10 checks
passed, five failed and one skipped. P6 contract/ACL/lease/schema/migration
behavior is unchanged; this change has no hosted checks yet and broad gates
remain open.

Version diff 0.5.31b -> 0.5.32b: synchronize the test-only HQL/typed-IR
ChangeScan P7 cursor-window differential across five exclusive-after/
inclusive-through bounds, including empty bounds and selected transaction
frontiers. History/Change passes 17/17, HQL2 passes 392/0/1 across 34 targets,
and the separate P6/schema-v6/compatibility sweep remains 194/0/0. PR #210
hosted checks pass 11, fail four worker markerless-identity jobs and skip one.
No P6 contract, ACL, lease, schema or migration behavior changed; broader
P6/P8, transport and review gates remain open.

Version diff 0.5.30b -> 0.5.31b: record D7's differential-proven one-hop
`ORDER BY` on the projected endpoint ID (default/ASC/DESC), with the 2/2
ordering target, 385/0/1 across 34 HQL2 targets, and 93/0/0 across 11 selected
P6/schema-v6/ACL targets. No P6 contract, ACL, lease, schema or migration
behavior changed; broader P6/P8, transport and independent-review gates remain
open.

Version diff 0.5.29b -> 0.5.30b: synchronize D7's differential-proven
single-label zero-hop HQL1 extension and its 10/10 focused adapter result.
HQL2 remains 382/0/1 across 33 targets and P6/schema-v6/compatibility remains
194/0/0 across 11. No P6 contract, ACL, schema or migration behavior changed;
broader P6/P8, transport and independent-review gates remain open.

Version diff 0.5.28b -> 0.5.29b: synchronize the HQL2/P7 regression record
with the test-only HQL/typed-IR `Values`/`UnionAll` differential against P7 for
169 nullable bag pairs and 338 Storage executions. HQL2 passes 382/0/1 across
33 targets; P6/schema-v6/compatibility remains 194/0/0 across 11. No P6
contract, ACL, schema or migration behavior changed. Broad P6/P8/transport and
independent review remain open.

Version diff 0.5.27b -> 0.5.28b: synchronize current HQL2/P7 regression
evidence after the storage-backed ChangeScan differential covered all five
supported subject kinds. History/Change passes 16/16, HQL2 380/0/1, and the
separate P6/schema-v6/compatibility group remains 194/0/0. PR #196's hosted
snapshot has 10 passed, five failed and one skipped check; Windows Rust failed
at 15m16 with detailed logs unavailable. Worker bootstrap correction remains
approval-gated. P6 semantics, ACL and schema are unchanged.

Version diff 0.5.26b -> 0.5.27b: extend storage-backed P7 HistoryScan
differentials to all five supported kinds using WAL-derived revision facts and
captured frontiers; verify Edge endpoint, Vector owner and Annotation
target/evidence ACL dependencies. Record History/Change 15/15, Annotation
source 7/7, HQL2 379/0/1, P7 130/130 and P6/schema-v6/compatibility 194/0/0.
No runtime/schema/contract change; Artifact HistoryScan and broad P6/P8
acceptance remain open.

Version diff 0.5.23b -> 0.5.24b: enforce the already-approved distinction
between the HQL2 namespace query grant, explicit Annotation(Read) for annotation
subjects, and recursive target/evidence access under one P6 lease. The new ACL
regression passes; the complete HQL2 sweep is 376/0/1 across 32 targets and the
selected six-target P6/schema-v6 suite is 43/0/0. No contract, schema or
migration change. Current hosted CI for the prior PR head remains worker-failing
and Windows Rust-cancelled; this patch awaits hosted checks and review. Broad
P6/P8/transport gates remain open.

Version diff 0.5.22b -> 0.5.23b: record PR #194 run 37083654705 at
docs-only head 43cc6e8: four worker bootstrap checks fail and Windows Rust
fails the storage Join differential with `QUERY_BUDGET_EXCEEDED`; local Join
reproduction passes 5/5, but the budget dimension remains unconfirmed. HQL2
remains 374/0/1 across 32 targets and the separate P6/schema-v6/compatibility
group remains 194/0/0 across 11. No P6 contract or schema change; broad
P6/P8/transport/review gates remain open.

Version diff 0.5.21b -> 0.5.22b: synchronize the current HQL2 row-history
parity checkpoint to 374/0/1 across 32 targets; the separate P6/schema-v6/
compatibility group remains 194/0/0 across 11 targets. Hosted PR #194 core
checks pass, but worker bootstrap checks remain red at schema-v6 initialization;
the P6 contract is unchanged and broader P6/P8/transport/review gates stay open.

Version diff 0.5.10b -> 0.5.11b: record the delegated P6 boundary for
Sequence node-ID/label predicates: exact revision-bound authorized snapshot,
no direct lookup, indistinguishable hidden/missing IDs, and budgeted labels.
Version diff 0.5.11b -> 0.5.12b: record implementation evidence for conditional
revision-bound label loading and 338/0/1 across 25 HQL2 targets; retain the
dedicated ACL-hidden-ID fixture and broader P6/P8 gates.
Version diff 0.5.12b -> 0.5.13b: synchronize approved P8 D1, preserving the
namespace-wide query grant and requiring pre-parse `FORBIDDEN/authorize` for
exact-record-only actors.
Version diff 0.5.13b -> 0.5.14b: verify the corrected pre-parse ACL fixture,
lease-bound Sequence property hydration and no-partial budget behavior; record
9 focused passes and 528/0/1 across 37 explicit HQL2/P6/schema-v6/compatibility
targets; retain broad P6/P8/P13 gates.
Version diff 0.5.14b -> 0.5.15b: record owner-approved H2-D11 R6b schema
selection from signed WAL activation when state.json is absent; require complete
migration proof and retain fail-closed behavior for ambiguous/v6-only WAL.
The implementation now passes the 17-test crash-recovery and 19-test migration
targets plus the selected 40-target HQL2/durability/authority aggregate; tests
use temporary fixtures only.

Version diff 0.5.15b -> 0.5.16b: implement and verify signed schema-v6
activation preflight, markerless v5/v6 WAL selection, fold-preserved migration
proof and fail-closed ambiguous recovery; no user database was migrated.

Version diff 0.5.17b -> 0.5.18b: synchronize the approved H2-D11 Vector
HistoryScan identity, source-floor and owner-ACL contract with P8; runtime
implementation and differential verification remain pending.

Version diff 0.5.18b -> 0.5.19b: implement HQL/typed-IR Vector HistoryScan with
the canonical H2-D11 tuple ID and P6 vector-floor check; record 12/12 focused
history/change tests, 367/0/1 across 31 HQL2 targets and 194/0/0 across 11
P6/schema-v6/compatibility targets. Broader P8/transport/review gates remain open.

Version diff 0.5.20b -> 0.5.21b: implement P8 HQL2/IR transaction-time
selection through one no-fallback frontier S across scans, graph/vector/
annotation operators, source floors, hydration and Snapshot.tx while the
generation/current policy remain pinned; record focused 56/56, HQL2 373/0/1
and P6/compatibility 194/0/0. Broad P8/P13/transport/review gates remain open.

Version diff 0.5.19b -> 0.5.20b: reconcile P6 HQL2 temporal semantics with the
accepted P8 snapshot contract: one selected transaction frontier S must govern
all HQL/IR source reads, hydration and `Snapshot.tx`, while the P6 lease/policy
generation remains pinned and legacy operations retain their own unsupported
selectors. Runtime verification is pending.

Version diff 0.5.16b -> 0.5.17b: record final full locked/offline Rust suite
and default/no-default strict Clippy passes for the integrated HQL2/H2-D11
path; `probe_vs_recall` remains NOT_RUN and no user database migration or
broader P6/P8/P13 qualification is claimed.

| Version | Date | Status | Summary | Commit | Agent |
|---|---|---|---|---|---|
| 0.5.33b | 2026-10-04 | beta | Sync the D7 labeled zero-hop exact string-ID filter differential; adapter 11/11, ordering 2/2, labeled-filter 2/2 including backslash/Unicode and wrong-label exclusion; HQL2 394/0/1 across 35 targets; selected P6 peers 45/0/0 across 7 targets; no P6 contract/ACL/lease/schema/migration change; PR #212 baseline 10 pass, 5 fail, 1 skipped; broad gates open | working-tree | ATHER |
| 0.5.32b | 2026-10-04 | beta | Sync test-only HQL/typed-IR ChangeScan P7 cursor-window differential across five exclusive-after/inclusive-through bounds; History/Change 17/17, HQL2 392/0/1 across 34 targets, P6/schema-v6/compatibility 194/0/0; PR #210 has 11 passed checks, four worker markerless-identity failures and one skipped; P6 contract/ACL/lease/schema/migration unchanged; broad gates remain open | working-tree | ATHER |
| 0.5.31b | 2026-10-04 | beta | Record D7's differential-proven one-hop ORDER BY on projected endpoint ID; ordering 2/2, HQL2 385/0/1 across 34 targets, selected P6/schema-v6/ACL targets 93/0/0 across 11; no P6 grant/ACL/lease/schema/migration change; broad gates remain open | working-tree | ATHER |
| 0.5.30b | 2026-10-03 | beta | Record D7's differential-proven single-label zero-hop HQL1 extension and 10/10 adapter tests; HQL2 382/0/1, P6/schema-v6/compatibility 194/0/0; no P6 contract/ACL/schema/migration change; broad gates remain open | working-tree | ATHER |
| 0.5.29b | 2026-10-03 | beta | Synchronize test-only HQL2 Values/UnionAll P7 differential evidence: 169 nullable bag pairs, 338 Storage executions, HQL2 382/0/1 across 33 targets; P6/schema-v6/compatibility 194/0/0; no P6 contract/schema change; broader gates remain open | working-tree | ATHER |
| 0.5.28b | 2026-10-03 | beta | Synchronize P6 record to current HQL2/P7 evidence: History/Change 16/16, HQL2 380/0/1, P6/schema-v6/compatibility 194/0/0; PR #196 has five failed hosted checks including worker bootstrap and an unverified Windows Rust failure at 15m16; no P6 contract/schema change | working-tree | ATHER |
| 0.5.27b | 2026-10-03 | beta | Record WAL-derived P7 HistoryScan differentials for Node/Edge/Row/Vector/Annotation under existing P6 ACL semantics; History/Change 15/15, Annotation source 7/7, HQL2 379/0/1, P7 130/130, P6/schema-v6/compatibility 194/0/0; Artifact, broad gates and hosted review remain open | working-tree | ATHER |
| 0.5.26b | 2026-10-03 | beta | Record independent P7 differential for HQL/typed-IR Vector HistoryScan using WAL-derived revisions and H2-D11 owner ACL; History/Change 14/14, HQL2 377/0/1, P6/schema-v6/compatibility 194/0/0; hosted/review and broader gates open | working-tree | ATHER |
| 0.5.25b | 2026-10-03 | beta | Implement and verify narrow ChangeScan budget rule: preserve namespace-only reads of other readable kinds; exclude Annotation subjects lacking Annotation(Read) before caller-budget accounting; ACL 11/11, History/Change 14/14, HQL2 376/0/1 and selected P6/schema-v6 43/0/0; hosted CI/review and broad P6/P8 remain open | working-tree | ATHER |
| 0.5.24b | 2026-10-03 | beta | Enforce explicit Annotation(Read) for annotation subjects in scans and ChangeScan while retaining Namespace(Read) for the query and recursive reference checks; ACL regression passes, HQL2 375/0/1, selected P6/schema-v6 suite 43/0/0; possible ChangeScan budget side channel and hosted worker/Windows Rust gates remain unresolved | working-tree | ATHER |
| 0.5.23b | 2026-10-03 | beta | Record PR #194 run 37083654705: four worker bootstrap checks and Windows Join budget check fail; local Join target passes 5/5, exact budget dimension unconfirmed; HQL2 374/0/1, P6/compatibility 194/0/0, P6 contract unchanged, broad gates open | working-tree | ATHER |
| 0.5.22b | 2026-10-03 | beta | Synchronize HQL2 row-history parity evidence to 374/0/1 across 32 targets and P6/schema-v6/compatibility to 194/0/0 across 11; hosted worker bootstrap checks fail at fresh schema-v6 initialization; P6 contract unchanged, broader gates open | working-tree | ATHER |
| 0.5.21b | 2026-10-03 | beta | Implement P8 HQL2/IR `tx_as_of` selection with one no-fallback source/hydration/operator/result frontier and per-source floor checks; record 56/56 focused, 373/0/1 HQL2 and 194/0/0 P6/compatibility; broader gates remain open | working-tree | ATHER |
| 0.5.20b | 2026-10-03 | beta | Specify P8 HQL2/IR `tx_as_of` selection as one no-fallback frontier across sources, hydration and result Snapshot; distinguish legacy ReadView rejections; runtime verification pending | working-tree | ATHER |
| 0.5.19b | 2026-10-03 | beta | Implement and verify Vector HistoryScan in HQL/typed IR using the H2-D11 tuple identity, P6 vector floor and existing owner-node ACL; focused 12/12, HQL2 367/0/1 and separate P6/compatibility 194/0/0; broader gates remain open | working-tree | ATHER |
| 0.5.18b | 2026-10-03 | beta | Synchronize approved Vector HistoryScan semantics: compact JSON `(owner_id, collection_id)` key, vector source floor and same-lease owner-node ACL; implementation/verification pending | working-tree | ATHER |
| 0.5.17b | 2026-10-02 | beta | Record full locked/offline Rust suite and both default/no-default strict Clippy passes; `probe_vs_recall` NOT_RUN; no user database migration or broader P6/P8/P13 qualification claimed | working-tree | ATHER |
| 0.5.16b | 2026-10-02 | beta | Implement and verify H2-D11 R6b markerless WAL recovery; crash tests 17/17, migration tests 19/19 and selected 40-target HQL2/durability/authority aggregate pass; fixture-only, no user database migration | 0135c29 | ATHER |
| 0.5.14b | 2026-10-02 | beta | Verify D1 exact-record-only denial before HQL/IR parsing, D4 P6-bound exact-property hydration, and 528/0/1 across 37 explicit targets; retain broad P6/P8/P13 gates | working-tree | ATHER |
| 0.5.15b | 2026-10-02 | beta | Owner-approved H2-D11 R6b: preflight signed schema activation and migration proof before markerless WAL replay | working-tree | ATHER |
| 0.5.13b | 2026-10-02 | beta | Synchronize approved P8 D1: retain namespace-wide query grant and assert exact-record-only actors are denied before HQL/IR parsing; implementation evidence pending | working-tree | ATHER |
| 0.5.12b | 2026-09-30 | beta | Implement conditional label loading from exact P6-visible node revisions; record 7 focused passes and 338/0/1 across 25 HQL2 targets; retain ACL-hidden fixture and broad P6/P8 gates | working-tree | ATHER |
| 0.5.11b | 2026-09-30 | beta | Freeze P6 boundary for delegated HQL2 Sequence node ID/labels using exact authorized node revisions and budgeted snapshot labels; runtime implementation and verification pending | working-tree | ATHER |
| 0.5.10b | 2026-09-30 | beta | Record lease-bound HistoryScan/ChangeScan runtime and 9/9 focused tests; full P6/P8 acceptance, transport parity and independent review remain open | working-tree | ATHER |
| 0.5.9b | 2026-09-29 | beta | Define source floors as minimum accepted exclusive ChangeScan cursors; exclude migration baseline events at the floor | working-tree | ATHER |
| 0.5.8b | 2026-09-29 | beta | Freeze P6 lease, source-floor and current-ACL rules for HQL2 HistoryScan/ChangeScan; runtime remains gated | working-tree | ATHER |
| 0.5.7b | 2026-09-29 | beta | Record exact KNN/Original Rerank original-vector reads under the pinned P6 lease and H2-D11 fingerprint; keep History/Change and broader P8 gates open | working-tree | ATHER |
| 0.5.6b | 2026-09-29 | beta | Record P8 AnnotationLookup lease-bound authorization and frozen/live plus optional/required semantics; retain broader P6/P8 gates | working-tree | ATHER |
| 0.5.5b | 2026-09-29 | beta | Truth-sync typed FieldIdV2/ExecBatchV2 property batches under the lease; preserve annotation reference checks and namespace-wide query grant | working-tree | ATHER |
| 0.5.4b | 2026-09-29 | beta | Record lease-bound Node/Edge/Row/Annotation ReadView scans, annotation reference checks and payload precharge; namespace-wide query grant remains required | working-tree | ATHER |
| 0.5.3b | 2026-09-29 | beta | Record schema-v6 Annotation policy-event v2 and compact/reopen coverage; target-aware annotation ReadView authorization remains open | working-tree | ATHER |
| 0.5.2b | 2026-09-29 | beta | Record H2-D11 owner-bound vector revision writes and 2/2 focused verification; Annotation ACL and P8 ReadView operators remain unimplemented | working-tree | ATHER |
| 0.5.1b | 2026-09-29 | beta | Record fixture-only migration/recovery verification (19/19); no user database migration or release qualification | working-tree | ATHER |
| 0.5.0b | 2026-09-29 | beta | Owner-approved migration gate with ordinary-reopen authority proof, fold-safe local-only records, recursive ingress rejection and exact generation receipt binding; fixture-only implementation | working-tree | ATHER |
| 0.4.0b | 2026-09-28 | candidate | Propose P6 binding for migration chunk commit, source transaction coordinates and ready-marker ordering; owner approved H2-D11 migration candidate, amended recovery details remain gated | working-tree | ATHER |
| 0.3.0b | 2026-09-28 | beta | Extend P6 for H2-D11 annotation target/evidence grants, per-source history floors and fail-closed schema-v6 migration gates; implementation not yet verified | working-tree | ATHER |
| 0.2.1b | 2026-09-28 | beta | Record the owner-approved P8 catalog/read extension; only catalog is implemented and legacy grants remain unchanged | working-tree | ATHER |
| 0.2.0b | 2026-09-23 | beta | Bind instant-loaded P6 state to signed WAL materializations; require HQL ACL checks before parsing | working-tree | ATHER |
| 0.1.2b | 2026-09-23 | beta | Classify fail-closed lookup/list accessors and separate operational/transport reads from record ACL | working-tree | ATHER |
| 0.1.1b | 2026-09-23 | beta | Clarify signed fold materialization, replay verification and fail-closed legacy read accessors | working-tree | ATHER |
| 0.1.0b | 2026-09-22 | beta | Initial owner-approved P6 generation, lease, temporal, ACL and fail-closed snapshot contract | working-tree | ATHER |
