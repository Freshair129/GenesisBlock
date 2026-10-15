---
proposed_id: ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION
doc_id: ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION
version: "0.1.0"
type: adr
status: candidate
owner: GenesisBlockDB Architecture
aliases:
  - ADR
phase: 39
tier: process
cluster: security
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-10-16T00:00:00.000Z
proposed_by: Codex
---

# ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION

## Status and approval boundary

This candidate records a source-verified security boundary for Codex Security
issues [#236](https://github.com/Freshair129/GenesisBlock/issues/236) and
[#237](https://github.com/Freshair129/GenesisBlock/issues/237). It selects no
peer-enrollment, membership-management, key-rotation, MASTER-authority, or
quorum policy. It is not an approved implementation contract. Code changes
that depend on those policies remain gated until an owner-approved decision
defines them.

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

## Security invariants for a future approved contract

- Discovery state and advertised keys are not, by themselves, membership
  authorization.
- A valid signature establishes key possession and event integrity; it does
  not establish the signer's authority to participate or mutate a data class.
- Event authorization must be enforced in shared engine ingress paths before
  durable or in-memory side effects, including public reconciliation and nested
  batch or transaction forms.
- Consensus voter eligibility and quorum membership must be derived from the
  approved authorization contract, not from an unauthenticated discovery
  cache.
- P6 cursor-receipt trust remains a separate boundary. A trusted cursor receipt
  must not be treated as blanket authorization for transferred events.
- Client-specific authority meaning remains client-owned under the master
  specification. The core must not infer a client's business authority from a
  tier label or a cryptographic signature alone.

## Decisions still required

An owner-approved successor contract must define:

- whether discovery is open and how discovered peers become authorized;
- who can admit, revoke, and rotate a member identity, and how that authority
  is authenticated and persisted;
- which authorized members may submit ordinary sync data and which, if any,
  may originate or approve MASTER state;
- the quorum electorate, membership-change timing, and single-node/self-vote
  behavior;
- compatibility and recovery behavior for existing databases and peers.

These decisions are recorded here as unresolved; this candidate does not choose
among them. The previously declined trust-contract selection is not inferred
from approval of this documentation update.

## Verification and closure criteria

Implementation work may start only after an owner-approved contract is
available. Its regression suite must cover:

- identity-to-key binding and authorized admission, revocation, and key
  rotation according to the approved policy;
- discovered but unauthorized peers failing event ingress and voter/quorum
  eligibility;
- authorized peer sync and voting continuing to work;
- MASTER authorization at every supported ingress, including public
  reconciliation and nested event variants, with no partial WAL or memory
  side effects on rejection;
- reopen/replay behavior and the approved membership-change and self-vote
  semantics.

The fix is complete only after focused tests, the full Rust suite, the relevant
server build, independent bypass review, and exact-head CI pass. Issues #236
and #237 remain open until these criteria are met. No proof of concept or
reproduction steps are included in this public-facing record.

## Changelog

| Version | Date | Status | Summary |
|---|---|---|---|
| 0.1.0 | 2026-10-16 | candidate | Record source-backed peer trust gaps, required security invariants, and owner decisions still needed; no trust or membership policy selected. |
