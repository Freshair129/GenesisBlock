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

1. **Canonical vote payload** binds the proposal ID, the SHA-256 digest of its
   complete signed event, voter ID, configured roster digest, and approve/reject
   choice. The proposal digest uses canonical event bytes plus signer ID and event signature;
   domain separation and length-prefixing prevent ambiguous encodings.
2. **`sign_vote(proposal_id, approve) -> Result<Vec<u8>>`** — a voter signs the
   payload with its own key (`signer = local_peer_id`) only after that proposal
   is stored locally. An unknown proposal returns an error.
3. **`submit_vote(proposal_id, peer_id, approve, signature)`** now, before
   recording:
   - resolves the voter's public key — this node's own key for a self-vote, else
     the key pinned in the operator roster; **unknown member → reject**;
   - locks the stored proposal, recomputes its signed-event digest, and verifies
     the signature over the canonical payload — **unknown proposal, malformed,
     or non-matching signature → reject** (vote not counted);
   - on success records the vote **and** retains the signature in
     `quorum_signatures` as proof.
4. **Gossip handler** passes the received `signature` through (no longer dropped).
5. **Surfaces updated:** NAPI `submitVote(.., signature)` + `signVote`; REST
   `POST /v1/consensus/vote` body gains `signature`, new `POST
   /v1/consensus/sign-vote`. A bad signature is a `400` (client error).

## Consequences

### Positive
- Votes are unforgeable and event-bound: only an authorized roster member holding
  the matching private key can approve the exact signed event the receiver will
  commit. Closes vote-forgery and proposal-substitution paths on MASTER promotion.
- `quorum_signatures` now carries verifiable proof of each counted vote (auditable
  quorum).
- Reuses the event-signature verification pattern while resolving voter keys
  from the immutable membership roster.

### Negative / Trade-offs
- **Breaking API change:** `submit_vote` / `submitVote` / `POST
  /v1/consensus/vote` require a `signature`. Callers must obtain one via
  `sign_vote` / `signVote` / `POST /v1/consensus/sign-vote`.
- A voter must be enrolled in the fixed roster, and the proposal must be present
  locally before `sign_vote` can produce a signature.

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

### Security boundary follow-up (2026-10-16)

This ADR establishes vote-signature integrity. The accepted
[peer-membership authorization ADR](ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION.md)
adds the separate voter-eligibility boundary: keys and quorum membership come
from the operator-pinned roster. The 2026-06-22 vote-signature tests remain
evidence for signature verification; roster membership and fixed quorum are
covered by the security regression tests.

### Vote-to-proposal binding follow-up (2026-10-16)

Votes also bind the digest of the complete `SignedEvent` stored for the proposal.
This prevents an approval for one event from being reused when a different event
is received under the same proposal ID. The voter signs only a locally stored
proposal, and the receiver recomputes the digest while holding the consensus
commit lock before counting the vote. Regression coverage includes a same-ID
event substitution attempt.

### Vote replay and proposal relay follow-up (2026-10-16)

Each member may cast only one choice per proposal and configured roster. The
signed payload also binds the roster digest. A durable vote-lock ledger is
fsynced before a local signature is returned or a remote vote is acknowledged;
the signer rejects a conflicting second choice and receivers treat an identical
retry as idempotent. The ledger stores the choice, proposal digest, roster digest,
and signature in the database directory. Importing a proposal after restart restores its durable
vote locks, so an old conflicting signature cannot become a new first vote.
Pending proposal objects remain in memory and must be imported again after
restart.

Before a quorum event enters the WAL, `consensus-commits.jsonl` records a durable
prepare receipt; after the event is applied, a completion record closes it. If
the process stops between those steps, re-import checks the exact signed event
in the retained WAL and completes the receipt before reporting the proposal as
committed. Completed receipts remain across journal compaction, preventing
re-import from appending the event twice. On Windows, ledger files use
write-through opens and file-handle sync; on Unix, the ledger file and parent
directory are synced before acknowledgement.

The vote and commit ledgers are append-only, with no automatic pruning because
old signed proposals and ballots can be replayed. Each ledger is capped at
64 MiB, 100,000 records, and 4 KiB per record. At the cap, the engine fails
closed before accepting more votes or beginning another consensus commit; an
operator must plan a database migration instead of deleting history.

The REST and NAPI surfaces now expose proposal export/import. Export returns the
signed proposal with vote state cleared. Import uses the same roster, signature,
and MASTER-role validation as gossip admission, so applications can relay the
exact proposal before voters sign it. Automatic outbound proposal gossip is not
implemented.

---
### Related Links
- **Governance:** [[ADR--GENESISDB-GOVERNANCE-LOGIC]]
- **Peer keys / sync:** `SyncPeer.verifying_key`, `reconcile_state` event verification

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.2 | 2026-10-16 | Bind votes to the digest of the complete signed event and require the proposal to be stored before signing; resolve voter keys from the fixed roster. |
| 0.1.1 | 2026-10-16 | Clarify that signature verification proves key possession relative to the resolved key, not authenticated voter membership; link candidate peer authorization contract. |
| 0.1.0 | 2026-06-22 | Proposed & accepted & shipped: ed25519 verification of consensus votes (`sign_vote` + signature-checked `submit_vote`); binds proposal+voter+choice; closes a vote-forgery governance bypass. Breaking: vote API gains `signature`. |
