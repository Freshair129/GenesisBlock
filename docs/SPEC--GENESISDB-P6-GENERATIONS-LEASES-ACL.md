---
doc_id: SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL
owner: GenesisBlockDB Engineering
version: 0.2.1b
created_at: "2026-09-22T22:55:00+07:00,ATHER,working-tree"
last_update: "2026-09-28T04:30:00+07:00,ATHER"
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
- tx_as_of must be at or above the pinned generation history_horizon; otherwise return
  TEMPORAL_BEYOND_HORIZON.
- Query IR inherits pinned selectors; conflicting request selectors fail.
- Neighbors and hybrid search may inherit as_of but reject tx_as_of for P6.
- node_view, node_versions, HQL and relational query reject a lease with either selector until
  their contracts can bind those selectors without fallback.

### Scoped read operations

The original P6 ReadView operations are node_view(id), node_versions(id, at_seq), neighbors(seed, args,
is_inferred), hybrid_search(args), execute_query_ir(request), execute_hql(query), and
query_relational(query). Each returns the existing result type wrapped in Result.

The owner-approved [P8 boundary](SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY.md)
adds crate-private typed catalog/scan/hydrate/vector operations without exposing
Storage. At the current scalar checkpoint only `hql2_catalog` is implemented;
the other three require proven identity/schema/source adapters before enabling.
Its catalog is query-private, namespace-authorized and borrowed under the
continuous commit guard. Plan-only catalog access does not pin or publish a
generation. Executed queries still pin one lease and validate it after execution
and before returning the encoded boundary result. Existing seven operations and
grant semantics are unchanged.

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
- AccessResource variants are Namespace, Node, Edge, Collection and Table(namespace, table).
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
| Neighbors, HQL, Query IR and retrieval-composed reads | Namespace(default) Read |
| Hybrid search | Namespace(default) Read; Collection alone is insufficient because results include nodes |
| Unsupported namespace/resource combination | Deny; never widen scope implicitly |

Disabled preserves existing direct-read compatibility. Enforced denies raw unscoped reads. No
claim is made that exact Edge or Collection grants provide a query surface beyond this matrix.

## Snapshot integrity and compatibility

1. P6 advances disk SCHEMA_VERSION from 4 to 5. This is a reader-compatibility change, not a
   data migration. The new engine continues to read v4 snapshots without a P6 manifest through
   the legacy load/replay path.
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
6. New-reader preflight rejects schema above its supported version. Older readers reject schema
   5; journal-only databases reject an unknown complete signed GenerationPublished or
   AccessPolicyChanged event rather than skip it.
7. state.json is the final commit marker and is not included in the component digest, so its P6
   fields require independent authority validation. Before instant load, scan all journal sources
   through the snapshot frontier, verify signatures on covered P6 events, and require the stored
   generation and access policy to match their latest verified WAL materializations. A mismatch
   rejects the whole snapshot and triggers complete WAL replay. If the snapshot asserts a
   non-default policy but the complete journal has no verified policy event, recovery is required;
   never reopen with the Disabled default. Legacy JSONL is part of this verification path.

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

## Stable error prefixes

GENERATION_STALE, LEASE_OWNER_MISMATCH, LEASE_EXPIRED, LEASE_REVOKED,
TEMPORAL_BEYOND_HORIZON, ACCESS_CONTEXT_REQUIRED, ACCESS_DENIED,
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

## CHANGELOG

| Version | Date | Status | Summary | Commit | Agent |
|---|---|---|---|---|---|
| 0.2.1b | 2026-09-28 | beta | Record the owner-approved P8 catalog/read extension; only catalog is implemented and legacy grants remain unchanged | working-tree | ATHER |
| 0.2.0b | 2026-09-23 | beta | Bind instant-loaded P6 state to signed WAL materializations; require HQL ACL checks before parsing | working-tree | ATHER |
| 0.1.2b | 2026-09-23 | beta | Classify fail-closed lookup/list accessors and separate operational/transport reads from record ACL | working-tree | ATHER |
| 0.1.1b | 2026-09-23 | beta | Clarify signed fold materialization, replay verification and fail-closed legacy read accessors | working-tree | ATHER |
| 0.1.0b | 2026-09-22 | beta | Initial owner-approved P6 generation, lease, temporal, ACL and fail-closed snapshot contract | working-tree | ATHER |
