---
doc_id: SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL
owner: GenesisBlockDB Engineering
version: 0.5.46b
created_at: "2026-09-22T22:55:00+07:00,ATHER,working-tree"
last_update: "2026-10-06T02:01:28+07:00,Codex"
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
9. IF a temporal selector is unsupported, conflicts with a request selector, falls below the
   generation history horizon, or exceeds the pinned generation WAL frontier THEN reject it
   before source access rather than silently using current state.
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
  WAL frontier. A below-horizon selector returns TEMPORAL_BEYOND_HORIZON; a selector above the
  pinned frontier is rejected before source access. The generation publication receipt is at
  `wal_frontier + 1` and is not itself a data frontier or a legal transaction-time selector.
  Both bounds apply to direct lease pinning and every scoped read path; a query-layer check
  against the current live sequence does not replace the pinned-generation upper-bound check.
- HQL2 `EXPLAIN` and typed-IR Plan requests carrying `tx_as_of` are not exempt from the upper
  bound merely because they do not return source rows. Validate against the same data frontier
  the request would bind: the current published generation's `wal_frontier` when its receipt is
  the observed frontier, otherwise the candidate generation's current commit frontier. Plan-only
  validation must not publish a generation or mutate WAL/state files.
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
  matches and new revision equals `expected_revision.checked_add(1)`. If increment overflows, reject
  with `ACCESS_POLICY_REVISION_CONFLICT`; same-revision policy replacement is never valid. Disabled bootstrap requires
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
source/ACL passes 7/7, the fresh HQL2 sweep passes 404/0/1 across 39 targets,
and the prior separate P6/schema-v6/compatibility group remains 194/0/0 (not
rerun here). Hosted validation of this source change,
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
   AccessPolicyChanged authority validation also reconstructs the CAS chain: each ordinary
   signed transition's `expected_revision` must equal the previously reconstructed policy
   revision, and its policy revision must be exactly one greater. Per-event shape and increment
   checks alone are insufficient. A trusted signed folded/base policy materialization may anchor
   a higher starting revision only when recognized by the existing fold format; every subsequent
   ordinary transition remains contiguous. The final reconstructed policy must equal the
   snapshot policy.
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
| Signed non-folded ACL event breaks the reconstructed CAS chain | Reject snapshot; complete WAL replay must reject the same gap rather than accepting the snapshot policy |
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
source changes. The temporal-frontier and snapshot ACL-chain RED lanes own disjoint test files
and may be authored in parallel; they do not permit parallel source edits.

Out of scope: REST/N-API/FFI/SDK contract changes, write authorization, transport identity,
namespace/entity migration, ACL group syntax, retention changes, MVCC, backup/restore redesign,
P7 oracle/planner/HQL2, deployment, merge and unrelated refactoring.

Acceptance commands:

    cargo test --no-default-features --test p6_generation_tests --test p6_lease_tests --test p6_visibility_tests --test wave_a_commit_tests --test temporal_queries_tests --test tx_as_of_wp22_tests --test governance_tests
    cargo test --locked --offline --no-default-features --test p6_snapshot_authority_tests --test p6_lease_tests --test hql2_history_change_tests
    cargo fmt --check
    cargo check --no-default-features
    npm run docs:validate
    git diff --check

Verify records exact local results. Review is independent and read-only; source findings return
to the serialized owner and require Verify to rerun. Final checks scope, evidence categories,
regressions and WIP preservation. Passing the gates supports only the P6 owner decision; it does
not approve P7, merge, release, deployment or external readiness.

### Bounded P6 Review Remediation DAG — 2026-10-05

The initial broad P6 Review Gate returned FAIL with two findings: the transaction selector was not
bounded by the pinned generation frontier on every entry path, and instant-load ACL authority
validation did not enforce continuity between signed CAS revisions. Those corrections passed an
independent Verify rerun. A subsequent broad Review rerun returned FAIL on a third bounded path:
Plan-only HQL/typed-IR returns before generation pinning and could accept the publication receipt.
That Plan-only correction has a RED/GREEN regression and local validation. The corrected tree now
has fresh independent Verify PASS and broad Review PASS; Review noted one bounded coverage
limitation for the stale-publication Plan fallback, recorded in the current gate status below. The
added finding and prevention criteria are recorded in
`.brain/rca/RCA--P6-PLAN-EXPLAIN-TEMPORAL-BYPASS.md`; the original temporal and ACL RCAs remain
at `.brain/rca/RCA--P6-TEMPORAL-SELECTOR-FRONTIER.md` and
`.brain/rca/RCA--P6-SNAPSHOT-ACL-CAS-CHAIN.md`.

See the [P6 review remediation dependency graph](P6-REVIEW-REMEDIATION-DAG.html) for the
parallel test lanes, isolated-worktree integration point, serialized source order and gate chain.

Execution and integration order (completed implementation stages and remaining gates):

1. Update this contract, the peer addendum's gate-scope statement, and both RCAs. This doc set is
   owner-approved; no further contract expansion is included.
2. Author only tests in disjoint files: (a) direct lease/HQL2 execution and Plan-only temporal-
   frontier regressions in `p6_lease_tests.rs`, `hql2_history_change_tests.rs`, and existing HQL2
   Explain coverage; (b) signed snapshot ACL chain/fold/reopen regressions in the new
   `p6_snapshot_authority_tests.rs` target. Keep EXPLAIN non-publishing and assert no WAL/state
   mutation for Plan-only boundary cases.
3. Both RED test lanes and the later Plan-only RED regression are integrated in the isolated P6
   worktree. One serialized source owner implemented ACL-chain validation, temporal upper-bound
   checks for execution, and non-mutating Plan validation in `src/lib.rs`.
4. Run the fresh focused and P6 acceptance Verify gates using E: for Cargo output and temporary files.
   A Verify failure returns to the serialized source owner; a Review finding also returns there
   and requires Verify to rerun.
5. After Verify passes, run an independent read-only Review. Only Review PASS permits Astra 6
   Final. Any failed gate stops P6 closeout; no P7, merge, release or deployment follows.

The prior local 45/0 and 104/0 records predate these newly identified regressions. They remain
historical and do not substitute for the fresh Verify gate.

### ACL revision-overflow Review finding — 2026-10-05

The independent Review Gate found that the shared transition predicate used saturating increment:
for `expected_revision == u64::MAX`, `expected_revision.saturating_add(1)` remains `u64::MAX` and
allows a changed policy without advancing its revision. The public replacement path and signed-event
shape validation use the same saturating rule, so all three boundaries must reject an absent checked
successor. The finding and required RED/GREEN coverage are documented in
`.brain/rca/RCA--P6-ACL-REVISION-OVERFLOW.md`.

The writer and signed-event regressions reproduced the overflow acceptance before the source fix.
The serialized correction now uses checked increments at mutation, signed-event validation, and
snapshot/replay transition boundaries. Fresh independent Verify passed on the corrected tree:
P6 acceptance 49/49, focused remediation 34/34, plus `cargo check`, format, docs validation (0/240),
and diff check. At this checkpoint the new independent Review rerun was pending; see the current
broad P6 gate status below for its superseding verdict. This enforces the approved CAS contract
without an API expansion.

## Current broad P6 gates — 2026-10-05

Fresh independent Verify passed on the exact corrected worktree: seven-target P6 acceptance
49/49; focused snapshot/lease/history remediation 34/34; `cargo check --locked --offline
--no-default-features`; `cargo fmt --all -- --check`; `npm run docs:validate` (0 violations in
240 files); and `git diff --check`. The independent Verify observed the same HEAD and working-tree
status before and after, used E: for build and temporary output, and excluded inherited peer receipt
WIP from this task's change attribution. These are local results, not hosted CI or deployment
evidence.

The independent read-only Review Gate returned **PASS** on the corrected tree. It confirmed the
ACL `checked_add(1)` boundaries and MAX regressions, temporal Plan/execution bounds, and snapshot
ACL continuity/replay checks. One non-blocking limitation remains: the stale-publication Plan
fallback does not have a dedicated regression fixture, although Review found the candidate-frontier
path clear in the implementation. Review relied on the supplied Verify receipts and did not rerun
the suites. Astra 6 Final returned **PASS_WITH_LIMITATIONS** and approved this reviewed local P6
slice at HEAD `ba488dd1e22cd24fdd2c7238a1c5de34f6f9fd1e`. Astra accepted the bounded Plan fallback
coverage limitation because its guarded candidate-frontier selection matches generation pinning;
it made no edits and reran no tests. Current-patch hosted/cross-platform CI and release/deployment
qualification remain NOT_RUN. No P7, merge, release, deployment, or external readiness is approved;
inherited user WIP remains preserved.

## Initial Independent P6 Review Status — 2026-10-05

The initial broad P6 Review was FAIL on the two bounded findings above. The earlier
`PASS_WITH_LIMITATIONS` Review and Astra 6 Final results documented in the peer addendum apply
only to the receipt-authority sub-slice; they are not broad P6 approval. Broad P6 Final is
NOT_RUN at that initial review checkpoint.

## P6 Review Remediation Verify — 2026-10-05

Both approved corrections were implemented after the disjoint RED tests reproduced the findings:

- Temporal RED: the generation was `F=3` with publication/live sequence `F+1=4`. Direct
  `pin_generation` accepted the receipt and `u64::MAX`; scoped `ReadView` returned node history at
  `F+1`; HQL and typed IR both accepted receipt sequence 4. Inclusive generation horizon/frontier
  and selector conflict/unsupported checks passed.
- ACL RED: three snapshot authority tests passed, while a correctly re-signed, locally shaped
  `0->1, 2->3` ordinary policy chain was accepted by instant snapshot load. WAL-only replay already
  rejected the gap. The folded revision-2 baseline plus contiguous revision-3 transition passed.

After implementation, local deterministic Verify is PASS:

1. The latest independent Verify rerun of the exact seven-target P6 acceptance command passed
   **49/49**. The earlier in-worktree **45/45** record is retained as historical; 49/49 is the
   current-tree result and current test inventory.
2. The focused remediation command
   `cargo test --locked --offline --no-default-features --test p6_snapshot_authority_tests --test p6_lease_tests --test hql2_history_change_tests`
   passed **31/31**.
3. The expanded durability/query regression matrix passed **129/129** across 14 targets:
   `crash_simulation_tests`, `governance_tests`, `hql2_history_change_tests`,
   `journal_format_tests`, `journal_migration_tests`, `p6_generation_tests`, `p6_lease_tests`,
   `p6_peer_authority_tests`, `p6_snapshot_authority_tests`, `p6_visibility_tests`,
   `schema6_migration_tests`, `temporal_queries_tests`, `tx_as_of_wp22_tests`, and
   `wave_a_commit_tests`.
4. `cargo test --locked --offline --no-default-features --lib` passed **7/7**.
5. `cargo check --locked --offline --no-default-features`, `cargo fmt --all -- --check`,
   `npm run docs:validate` (0 violations in 240 files), and `git diff --check` passed.

Cargo target output and process temporary files were directed to
`E:\CodexBuilds\P6-review-remediation-main` and
`E:\CodexTemp\P6-review-remediation-main`. These are local fixture results. Independent Verify
passed with the counts above. The broad Review rerun is FAIL on the Plan-only temporal path and
also noted the stale 45/45 documentation count; this version reconciles the latter to the
independent 49/49 result while preserving the prior record. Astra 6 Final is NOT_RUN pending a
passing Review rerun; no merge, P7, release, deployment, or external readiness is claimed.

## P6 Review Rerun Status at the 32-test checkpoint — 2026-10-05

The broad Review rerun returned FAIL because `ExplainV2::Plan` returns before `pin_generation`,
while the HQL2 catalog stamp exposes the live commit sequence that may be the publication receipt.
That finding was reproduced RED and corrected without publishing or writing from Plan mode. Fresh
independent Verify on the corrected current tree passed the exact seven-target P6 acceptance
(49/49), focused remediation (32/32), format, docs validation (0 violations in 240 files), diff
check and `cargo check`. At that checkpoint the independent Review rerun was pending; the current
broad P6 gate status above supersedes that status. Astra 6 Final remains NOT_RUN.

## Plan-only temporal remediation — local verification — 2026-10-05

The Plan-only regression used a generation data frontier `F=3` and publication receipt `F+1=4`.
Before the source change, HQL `EXPLAIN` and typed-IR Plan both accepted selector 4; the RED test
also confirmed the inclusive data frontier and the no-file/no-WAL-write invariant. After the
change, both Plan paths reject the receipt and preserve the same no-publication/no-write behavior.

The shared generation-bound predicate now validates Plan selectors against the currently
published generation's data frontier, or the candidate generation frontier if that publication
is stale, without calling `pin_generation`. Executed queries continue to validate against their
pinned generation.

Local results at this checkpoint after this correction:

1. The exact seven-target P6 acceptance command passed **49/49**.
2. The exact three-target focused remediation command passed **32/32**; the four-target combined
   snapshot/lease/history/execution check passed **47/47**.
3. The expanded 14-target durability/query matrix passed **130/130**; the library tests passed
   **7/7** and `cargo check --locked --offline --no-default-features` passed.
4. Fresh independent Verify passed `cargo fmt --all -- --check`, `npm run docs:validate` (0
   violations in 240 files), `git diff --check`, and `cargo check --locked --offline
   --no-default-features`.

The expanded 14-target matrix (130/130) and library target (7/7) are additional local results at
this checkpoint. Cargo target and temporary output remained on E:. The later 34-test focused
Verify and broad Review PASS are recorded under Current broad P6 gates above. Astra 6 Final remains
NOT_RUN. No merge, P7, release, deployment, or external readiness is claimed.

## Bounded P6 Core Verification — 2026-10-04

The exact acceptance command above passed all 45 tests across its seven targets
with exit code 0. An expanded P6 durability matrix passed all 104 tests across
12 targets with exit code 0: `crash_simulation_tests`, `governance_tests`,
`journal_format_tests`, `journal_migration_tests`, `p6_generation_tests`,
`p6_lease_tests`, `p6_peer_authority_tests`, `p6_visibility_tests`,
`schema6_migration_tests`, `temporal_queries_tests`, `tx_as_of_wp22_tests` and
`wave_a_commit_tests`. The seven-target acceptance set is a subset of the
expanded matrix; these counts are not additive.

`cargo check --no-default-features`, `cargo fmt --all -- --check`,
`npm run docs:validate` (0 violations in 240 files) and `git diff --check`
passed. Cargo used `--offline --locked --no-default-features`; build and test
temporary files were directed to E: because C: had no free space. These are
local results only. Independent Review and Final remain pending; broader
P6/P8/HQL2 acceptance, transport parity, hosted CI, release and deployment
remain open or NOT_RUN. No merge or P7 work is authorized by these results.
Cargo output also included `database or disk is full` / `Error code 13` while
the relevant commands returned exit code 0 and the test targets reported zero
failures. Its source was not established; retain this as an environment
limitation rather than attributing it to the P6 implementation.

## CHANGELOG

Version diff 0.5.45b -> 0.5.46b: re-integrate the P6 closeout on mainline `4b78596` while
preserving the D7/P7 ChangeScan differential and both temporal test lanes. Local Verify on the
integrated tree passes the exact seven-target P6 acceptance (49/49), focused snapshot/lease/
history tests (35/35), `cargo check --locked --offline --no-default-features`, format, strict core
Clippy, docs validation (0/241), and diff check. Astra's prior Final applies to the previously reviewed P6
slice; it was not rerun on this rebased HEAD. Current hosted checks remain pending. No P7, release,
deployment, or external-readiness scope is added.

Version diff 0.5.44b -> 0.5.45b: record Astra 6 Final PASS_WITH_LIMITATIONS for the reviewed local
P6 slice at the verified HEAD. Preserve the accepted stale-publication Plan fallback fixture
limitation and state that hosted/cross-platform CI and release/deployment remain NOT_RUN. No P7,
merge, release, or deployment authorization.

Version diff 0.5.43b -> 0.5.44b: record corrected-tree independent Verify PASS (P6 49/49,
focused 34/34, check/format/docs/diff PASS) and independent broad Review PASS. Preserve Review's
bounded stale-publication Plan fallback fixture limitation; Astra 6 Final remains NOT_RUN. No P7,
merge, release, deployment or external readiness.

Version diff 0.5.36b -> 0.5.37b: extend the approved generation-frontier contract to non-mutating
HQL EXPLAIN and typed-IR Plan validation, record the independent 49/49 Verify result and the
subsequent Plan-only Review finding, add its RCA and update the temporal execution DAG. The Plan
regression and correction remain pending; Astra 6 Final is NOT_RUN. No P7 or merge.

Version diff 0.5.35b -> 0.5.36b: implement the generation-frontier and snapshot ACL CAS-chain
corrections with a shared policy transition validator; record temporal RED 3/10 plus HQL2 RED
1/17, ACL snapshot RED 1/4, exact P6 Verify 45/45, focused remediation 31/31, expanded matrix
129/129 across 14 targets, lib 7/7, and local compile/format/docs/diff passes. Independent Verify
attestation, broad Review and Astra 6 Final remain pending; no P7 or merge is authorized.

Version diff 0.5.34b -> 0.5.35b: record the broad P6 Review failure and two bounded RCA-backed
corrections; specify the pinned-generation transaction frontier and contiguous signed ACL CAS
chain, define disjoint parallel RED-test lanes and serialized `src/lib.rs` integration, and keep
Verify, independent Review, and Astra 6 Final as required gates. Implementation and all new gate
results remain pending; no P7, merge, release, or deployment is authorized.

Version diff 0.5.33b -> 0.5.34b: record the bounded P6 core acceptance results:
45/0 across the exact seven-target acceptance command and 104/0 across the
expanded 12-target durability matrix, plus local check/format/docs/diff gates.
The result is local only; independent Review, Final and broader P6/P8 gates
remain open. No runtime contract or behavior changed.

Version diff 0.5.32b -> 0.5.33b: synchronize the conditional D7 extension for
one exact same-alias string ID predicate on labeled zero-hop scans. The
legacy/HQL2 differential covers match, miss, wrong-label exclusion and
backslash/Unicode handling; focused adapter/order/labeled-filter targets pass
11/11, 2/2 and 2/2; HQL2 passes 394/0/1 across 35 targets. Selected P6 peer
tests pass 45/0/0 across seven targets. PR #212 baseline reports 10 checks
passed, five failed and one skipped. PR #213 code commit checks report 40 pass,
four worker failures with cause unconfirmed, five skipped and one Windows Cargo
cancellation at the configured 15-minute job limit. P6
contract/ACL/lease/schema/migration behavior is unchanged; broad gates remain
open.

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
| 0.5.46b | 2026-10-06 | beta | Re-integrate onto mainline 4b78596; local Verify P6 49/49, focused snapshot/lease/history 35/35, cargo check/fmt/strict core Clippy/docs/diff pass; Astra Final remains limited to the predecessor slice and was not rerun on this HEAD; hosted checks pending | working-tree | Codex |
| 0.5.45b | 2026-10-06 | beta | Record Astra 6 Final PASS_WITH_LIMITATIONS for the reviewed local P6 slice; stale-Plan fallback fixture limitation accepted; hosted/cross-platform CI and release/deployment remain NOT_RUN | working-tree | Codex |
| 0.5.44b | 2026-10-05 | beta | Record corrected-tree independent Verify PASS (P6 49/49, focused 34/34, check/format/docs/diff PASS) and independent broad Review PASS; preserve the bounded stale-Plan fallback fixture limitation; Astra Final pending | working-tree | Codex |
| 0.5.43b | 2026-10-05 | beta | Record corrected-tree independent Verify PASS after checked-increment fix: P6 49/49, focused 34/34, cargo check/fmt/docs/diff pass; independent Review rerun and Astra Final pending | working-tree | Codex |
| 0.5.42b | 2026-10-05 | beta | Fix P2 ACL revision overflow with checked increment at writer, signed-event validation and snapshot/replay transition; two RED regressions pass; local P6 49/49, focused 34/34, expanded 132/132; fresh Verify/Review pending | working-tree | Codex |
| 0.5.41b | 2026-10-05 | beta | Reproduce P2 ACL revision overflow with two pre-fix RED regressions: public writer accepts changed policy at u64::MAX and signed same-revision event opens; checked-increment source fix and gates pending | working-tree | Codex |
| 0.5.40b | 2026-10-05 | beta | Record independent Review P2: saturating ACL revision increment permits same-revision changed policy at u64::MAX; add checked-increment contract and RCA; RED/source fix/Verify/Review rerun pending | working-tree | Codex |
| 0.5.39b | 2026-10-05 | beta | Record current-tree independent Verify PASS after Plan-only temporal RED/GREEN correction: P6 49/49, focused 32/32, docs/fmt/diff/check pass; independent Review and Astra Final remain pending | working-tree | Codex |
| 0.5.38b | 2026-10-05 | beta | Implement Plan-only temporal frontier validation without publication; RED/GREEN preserves no-write behavior; current local P6 49/49, focused 32/32, expanded matrix 130/130; fresh Verify/Review and Astra Final pending | working-tree | Codex |
| 0.5.35b | 2026-10-05 | beta | Record two broad P6 Review findings, temporal frontier/ACL CAS chain requirements, RCAs, parallel test lanes, serialized source order, and Verify/Review/Final gates; implementation pending | working-tree | Codex |
| 0.5.34b | 2026-10-04 | beta | Record bounded P6 core Verify: exact acceptance 45/0 across seven targets; expanded P6 durability 104/0 across 12 targets; cargo check, fmt, docs and diff checks pass; independent Review/Final and broader P6/P8 gates remain open | working-tree | ATHER |
| 0.5.33b | 2026-10-04 | beta | Sync the D7 labeled zero-hop exact string-ID filter differential; adapter 11/11, ordering 2/2, labeled-filter 2/2 including backslash/Unicode and wrong-label exclusion; HQL2 394/0/1 across 35 targets; selected P6 peers 45/0/0 across 7 targets; PR #213 code commit checks 40 pass, 4 worker failures with unconfirmed cause, 5 skipped, 1 Windows Cargo cancellation at 15-minute job limit; no P6 contract/ACL/lease/schema/migration change; broad gates open | working-tree | ATHER |
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
