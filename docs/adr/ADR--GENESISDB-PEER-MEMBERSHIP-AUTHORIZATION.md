---
proposed_id: ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION
doc_id: ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION
version: "1.0.0"
type: adr
status: accepted
owner: GenesisBlockDB Architecture
aliases:
  - ADR
phase: 39
tier: process
cluster: security
role: "Architecture decision record"
enforcement_state: active
decided_at: 2026-10-16T00:00:00.000Z
decided_by: repository owner authorization
---

# ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION

## Decision

This ADR defines the implementation contract for Codex Security issues
[#236](https://github.com/Freshair129/GenesisBlock/issues/236) and
[#237](https://github.com/Freshair129/GenesisBlock/issues/237). The repository
owner authorized the implementation to choose and apply the required trust
policy.

GenesisBlock uses a **closed, operator-configured roster**. Plain `open()` is
self-only and starts no gossip listener. Multi-node use must open through
`openWithMembership` (NAPI) or configure `Storage` with the same roster before
starting gossip. The roster pins each peer's derived ID, Ed25519 public key,
numeric UDP endpoint, and `masterAuthority` role. A local peer is a master by
default for single-node compatibility; multi-node deployments must set the
local role explicitly. Unknown identities, key changes, and endpoints not in
the roster are rejected. Membership is immutable for a running `Storage`;
enrollment, removal, or key rotation requires an operator-configured restart.

Every gossip message is carried in a signed envelope bound to the sender ID and
the roster digest. The receiver checks the Ed25519 signature, exact configured
source endpoint, and matching roster digest before dispatch. Heartbeats update
only the live endpoint cache; they never create membership or replace a pinned
key or address. Signed event origins may differ from the transport sender so
authorized members can relay events, but both the relay and each event signer
must be roster members.

Every roster member may originate ordinary graph sync. Only a member with
`masterAuthority` may create or change `MASTER` state or submit consensus
proposals that promote graph nodes to `MASTER`. Because a `NodeRetract` event
does not carry authenticated pre-state labels, all node retractions require
master authority. Consensus votes are counted only for roster members, and
the electorate is the fixed roster including the local member. A proposal
commits only with a strict majority of that roster. A singleton roster retains
the existing local self-vote behavior.

Already durable events remain readable on reopen and WAL replay is not
retroactively filtered against today's roster. The roster controls new network
and public reconciliation ingress.

The scan was performed at 5d19e7bd895a0794749f0cd87cb14caab350b42b. The
post-PR-238 mainline review at
edb06f32189193d0f636707acd528b69e0dbb147 confirmed the same relevant trust
boundaries remain in source. This is a source review; it does not claim runtime
reachability or exploit reproduction.

## Parent and peer contracts

- [Master specification](../MASTER-SPEC--GENESIS-DB.md), sections 7 and 8,
  keeps generic governance primitives separate from each client's authority
  model and separates synchronization conflict rules from semantic policy.
- [P2P gossip specification](../SPEC--P2P-GOSSIP-PROTOCOL.md) describes
  discovery and signed state exchange.
- [Cryptographic swarm specification](../SPEC--CRYPTOGRAPHIC-SWARM.md)
  describes persisted identity and event signatures.
- [Collaborative sync specification](../SPEC--COLLABORATIVE-SYNC.md) requires
  peer-side governance validation.
- [P6 contract](../SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md) explicitly
  places transport synchronization outside the record-read ACL and requires a
  separate peer-authorization contract.
- [P6 peer-locality addendum](../SPEC--GENESISDB-P6-PEER-LOCALITY-ADDENDUM.md)
  keeps P6 control metadata local and does not approve peer enrollment.
- [Consensus vote-signature ADR](ADR--GENESISDB-CONSENSUS-VOTE-SIGNATURES.md)
  and [proposal-auth ADR](ADR--GENESISDB-CONSENSUS-PROPOSAL-AUTH.md) define
  signature checks but predate an authenticated membership source.
- [Security policy](../../SECURITY.md) states that governance enforcement
  belongs in the engine.

## Source-backed baseline

1. Storage opens with an empty peer map and has no configured authenticated
   member roster. Local peer identity is derived from its public key, but
   incoming heartbeat identity is not checked against that derivation.
2. A heartbeat inserts its claimed peer ID and advertised key into the
   discovery map. The map is bounded and expires entries, but those controls do
   not establish membership authorization.
3. Event and vote signature verification resolves remote keys from that map.
   This verifies possession of the corresponding private key relative to the
   advertised key; it does not prove authorized membership.
4. Consensus quorum uses the discovered-peer map as its electorate. A
   discovery entry therefore affects both signature resolution and quorum
   calculation without an independent admission decision.
5. PushDelta checks event signatures, then calls the shared reconciliation
   path. The production progress-receipt verifier has no trusted responder key
   and fails closed for cursor advancement; that cursor rule does not itself
   authorize or reject event application.
6. Reconciliation applies remote Node events without the local direct-write
   governance guard. Public reconciliation and nested event forms also need
   coverage at the shared engine boundary.

Source locations at the reviewed mainline include peer admission in
src/lib.rs:19859-19876, key resolution in src/lib.rs:15484-15490, quorum in
src/lib.rs:15645-15650, PushDelta in src/lib.rs:19987-20020, reconciliation
signature checks in src/lib.rs:19162-19174, and remote Node application in
src/lib.rs:19196-19246.

## Security invariants

- Discovery state and advertised keys are not membership authorization.
- A valid signature establishes key possession and event integrity; it does
  not establish the signer's authority to participate or mutate a data class.
- Event authorization is enforced in shared engine ingress paths before
  durable or in-memory side effects, including public reconciliation and nested
  batch or transaction forms.
- A compacted revision checkpoint's source proof is authorized against the
  sequential batch state accumulated before that checkpoint. Proof validation
  reads a copy of the overlay so the proof cannot itself advance authorization
  state for later events.
- Consensus voter eligibility and quorum membership come from the pinned
  roster, not from a discovery cache.
- P6 cursor-receipt trust remains a separate boundary. A trusted cursor receipt
  must not be treated as blanket authorization for transferred events.
- Client-specific authority meaning remains client-owned under the master
  specification. The core must not infer a client's business authority from a
  tier label or a cryptographic signature alone.

## Operational requirements

- Provision peer identity by comparing `localIdentity()` peer IDs and public
  keys out of band, then configure the same roster on every member.
- Assign numeric, reachable UDP endpoints and a fixed local bind address. DNS
  names, dynamic discovery, and endpoint changes during a running process are
  unsupported.
- Configure matching `masterAuthority` roles on every node. A roster digest
  mismatch prevents the peers from exchanging gossip.
- Compaction emits only the current protected node or retained retraction and
  binds it to the original authorized source event in a detached proof carried
  inside `SignedEvent.signature`. The relay signature covers both the current
  projection and proof. The proof matches the exact node content and logical
  clock (embedding is transported separately); it cannot authorize a changed
  payload or higher clock. A nested source `Batch` or `Transaction` is verified
  but never replayed, so stale sibling changes cannot reappear during sync.
  The proof also preserves an exact authorized USER transition needed to clear
  a peer's older MASTER state or tombstone. Schema-v6 compacted `Node`, `Edge`,
  and `VectorMaterialized` rows carry the exact signed, revision-bearing source
  transaction; the relay signature binds that proof to the base projection.
  Receivers verify the source signature, revision-to-payload binding, and
  projection match before accepting the compacted row. When a declassified USER
  node has later USER edits, compaction emits the exact MASTER-authorized
  transition first, followed by the separately revision-bound current state.
  Protected nodes and all retractions still require MASTER proof. If compaction
  cannot recover and verify the origin event, the fold fails closed with
  `COMPACTION_MASTER_AUTHORITY_PROOF_MISSING` or
  `COMPACTION_BASE_EVENT_PROOF_MISSING`.
  Backup and compaction preflight preserve the same fold boundary: retained
  pre-base sources establish the authorization history, base materializations
  establish the state at the frontier, and post-base HISTORY or active events
  follow in frame order. Segment kind alone does not classify HISTORY as
  pre-base.
- Compacted proof envelopes use a compressed V2 representation and a `b64:`
  JSON signature encoding, while readers continue to accept legacy byte-array
  signatures. A validated schema-5-to-6 migration may receive a local
  revision-bound attestation for the exact migrated projection; the original
  MASTER proof remains independently required for protected state. Edge-derived
  `impact` is recomputed after edge sync. Gossip limits each signed event to
  54,000 JSON bytes and each sync delta to 55,000 bytes.
- `openWithMembership` rejects a multi-node configuration when
  `localMasterAuthority` is omitted; single-node mode defaults it to `true`.
- Deploy roster, role, key, and endpoint changes to all members as one
  operator-coordinated cluster change. Runtime enrollment and key rotation are
  not supported.
- Relay consensus proposals by exporting the signed proposal at the proposer,
  importing it at each voter, collecting each `signVote` signature, and
  submitting those signatures to the proposal owner. Exported envelopes clear
  vote state; import rechecks the signed event, roster membership, and MASTER
  role. Automatic outbound proposal gossip is not implemented.
- A member's first vote for a proposal is durably immutable for that configured
  roster. The signed vote binds the roster digest and event digest; vote locks
  are fsynced to `consensus-votes.jsonl` in the database directory. Duplicate
  deliveries of the same choice are idempotent and conflicting choices remain
  rejected after restart. Pending proposal objects must be imported again after
  restart, which restores durable vote locks from the database directory.
- A quorum commit writes a prepare receipt before its WAL event and a completion
  receipt after application in `consensus-commits.jsonl`. Re-import after restart
  restores completed state; a prepared receipt with a matching WAL event closes
  the crash gap without appending the event again. Receipts survive compaction.
- The vote and commit ledgers are append-only and have fail-closed limits of
  64 MiB, 100,000 records, and 4 KiB per record. They are included in database
  backups; see [Operations](../OPERATIONS.md) for the migration requirement at
  the limit.
- Maintenance pruning reports failed node retractions in the process log when
  local role does not permit the operation.
- Older peers that send raw `GossipMessage` JSON are rejected; all peers must
  use the authenticated packet protocol.

## Verification and closure criteria

The regression suite covers:

- identity-to-key binding and rejection of unconfigured identities;
- endpoint, packet-signature, and roster-digest verification;
- discovered but unauthorized peers failing event ingress and voter/quorum
  eligibility;
- authorized peer sync and voting continuing to work;
- MASTER authorization at every supported ingress, including public
  reconciliation and nested event variants, with no partial WAL or memory
  side effects on rejection;
- compaction followed by sync bootstrap preserving the original MASTER signer
  for live MASTER nodes and retained retractions;
- nested `MASTER` transactions, batches, and retractions being rejected before
  WAL or memory side effects;
- fixed-roster quorum and singleton self-vote behavior.

The findings close only after focused and full Rust tests, the relevant server
build, independent bypass review, and exact-head CI pass. Existing WAL records
remain intact; no proof of concept or reproduction steps are included in this
public-facing record.

## Changelog

| Version | Date | Status | Summary |
|---|---|---|---|
| 1.0.0 | 2026-10-16 | accepted | Adopt a closed static roster, signed gossip envelopes, MASTER-role enforcement, and fixed-roster quorum for security fixes #236 and #237. |
| 1.0.1 | 2026-10-16 | accepted | Require compacted revision source proofs to use the accumulated batch authorization overlay. |
