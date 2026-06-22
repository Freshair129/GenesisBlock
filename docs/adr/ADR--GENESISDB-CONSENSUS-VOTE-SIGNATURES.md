---
proposed_id: ADR--GENESISDB-CONSENSUS-VOTE-SIGNATURES
type: adr
status: accepted
aliases:
  - ADR
phase: 37
tier: process
cluster: governance
role: "Architecture decision record"
enforcement_state: active
proposed_at: 2026-06-22T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-CONSENSUS-VOTE-SIGNATURES

## Context

Multi-agent consensus promotes `USER` data to a `MASTER` axiom once a proposal
clears quorum (`approvals > peers/2`). Each peer has an ed25519 keypair and its
public key is shared with others via the gossip `Heartbeat` (stored in
`SyncPeer.verifying_key`). The wire `GossipMessage::ConsensusVote` already carried
a `signature` field.

**But `submit_vote` never checked it.** It recorded `votes[peer_id] = approve` for
whatever `peer_id` the caller supplied, and the gossip handler explicitly dropped
the signature (`signature: _`). Any caller — local or remote — could therefore
**forge votes on behalf of arbitrary peers** and push a proposal to quorum,
defeating the governance guard that protects MASTER-tier axioms. The
`ConsensusProposal.quorum_signatures` map meant to hold vote proof was unused.

(`reconcile_state` already verified *event* signatures against
`SyncPeer.verifying_key`, so the verification primitives and key distribution were
in place — only the vote path was unguarded.)

## Decision

Authenticate every vote before counting it.

1. **Canonical vote payload** binds the decision to the proposal and voter:
   `vote_payload(proposal_id, voter_peer_id, approve) = "VOTE|{id}|{peer}|{approve}"`.
   Binding all three prevents replaying a vote onto another proposal or flipping
   approve↔reject.
2. **`sign_vote(proposal_id, approve) -> Vec<u8>`** — a voter signs the payload
   with its own key (`signer = local_peer_id`), producing a detached ed25519
   signature to send to the proposal-holder.
3. **`submit_vote(proposal_id, peer_id, approve, signature)`** now, before
   recording:
   - resolves the voter's public key — this node's own key for a self-vote, else
     the key registered for `peer_id` (gossip); **unknown peer → reject**;
   - verifies the signature over the canonical payload — **malformed or
     non-matching → reject** (typed error, vote not counted);
   - on success records the vote **and** retains the signature in
     `quorum_signatures` as proof.
4. **Gossip handler** passes the received `signature` through (no longer dropped).
5. **Surfaces updated:** NAPI `submitVote(.., signature)` + `signVote`; REST
   `POST /v1/consensus/vote` body gains `signature`, new `POST
   /v1/consensus/sign-vote`. A bad signature is a `400` (client error).

## Consequences

### Positive
- Votes are unforgeable: only a peer holding the private key matching a registered
  public key can cast a vote that counts. Closes a governance-bypass on MASTER
  promotion.
- `quorum_signatures` now carries verifiable proof of each counted vote (auditable
  quorum).
- Reuses the existing key distribution (`SyncPeer.verifying_key`) and the
  verification pattern already used by `reconcile_state`.

### Negative / Trade-offs
- **Breaking API change:** `submit_vote` / `submitVote` / `POST
  /v1/consensus/vote` require a `signature`. Callers must obtain one via
  `sign_vote` / `signVote` / `POST /v1/consensus/sign-vote`.
- A peer must be known (its key registered via gossip) before its vote can be
  verified — votes from not-yet-discovered peers are rejected until the
  Heartbeat that carries their key arrives.

## Alternatives Considered
| Alternative | Reason Rejected |
|---|---|
| Trust `peer_id` as asserted (status quo) | The vulnerability — any caller forges votes. |
| Sign only `(proposal_id, approve)` without the voter id | A signature could be lifted and re-submitted under a different `peer_id`. Binding the voter prevents it. |
| Verify at the gossip layer only | The embedded/NAPI/REST `submit_vote` path would stay unguarded; verification belongs in the engine, like the governance guard. |

## Verification
- `tests/consensus_vote_sig_tests.rs` (5): authentic self-vote reaches quorum;
  unknown-peer rejected; tampered signature rejected; signature bound to the
  choice (approve-signature can't cast a reject); malformed signature is a typed
  error, not a panic.
- Full `cargo test` (30 binaries) + `npm test` (7) green; `index.d.ts`
  regenerated (`submitVote` + `signVote`).

### Outcome (measured 2026-06-22)
Shipped. Consensus votes are ed25519-verified against registered peer keys before
counting; forged/replayed/flipped votes are rejected.

---
### Related Links
- **Governance:** [[ADR--GENESISDB-GOVERNANCE-LOGIC]]
- **Peer keys / sync:** `SyncPeer.verifying_key`, `reconcile_state` event verification

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-22 | Proposed & accepted & shipped: ed25519 verification of consensus votes (`sign_vote` + signature-checked `submit_vote`); binds proposal+voter+choice; closes a vote-forgery governance bypass. Breaking: vote API gains `signature`. |
