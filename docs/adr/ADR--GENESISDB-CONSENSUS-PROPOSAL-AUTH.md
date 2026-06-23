---
proposed_id: ADR--GENESISDB-CONSENSUS-PROPOSAL-AUTH
type: adr
status: accepted
aliases:
  - ADR
phase: 38
tier: process
cluster: governance
role: "Architecture decision record"
enforcement_state: active
proposed_at: 2026-06-23T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-CONSENSUS-PROPOSAL-AUTH

## Context

[[ADR--GENESISDB-CONSENSUS-VOTE-SIGNATURES]] closed vote forgery: a vote now only
counts if it is ed25519-signed by the registered key of the voter it claims to be
from. But a review of the *rest* of the consensus path found the **proposal**
side still unauthenticated, plus three commit-path correctness bugs:

1. **Proposal payload never verified.** `propose_consensus` wrapped the caller's
   event + signature into a `SignedEvent` without verifying it and without running
   `semantic_verify`. The gossip `ConsensusPropose` handler — commented
   "Auto-verify and store" — actually inserted the proposal verbatim, no check. At
   commit, `submit_vote` applied + persisted the event without ever verifying its
   signature. A peer could inject a fabricated `MASTER` node proposal and, with a
   small/low-quorum swarm, drive it to commit with its own authentic self-vote —
   the governance bypass the vote-sig ADR aimed at, reopened through the proposal
   channel.
2. **Quorum re-applied on every late vote.** The commit block fired on *every*
   approving vote once `approvals > peers/2`, re-applying and re-`persist_signed`ing
   the event each time (WAL growth, duplicate work) — no "already committed" guard.
3. **Committed edges weren't traversable.** The `Event::Edge` commit arm inserted
   into `edges` but never called `index_edge_internal`, so `out_idx`/`in_idx`
   adjacency was not built — the edge was invisible to `TRAVERSE` until reload.
   Inconsistent with `add_edge` and the `reconcile_state` sync path.
4. **Committed vectors weren't searchable.** The `Event::Vector` arm only
   persisted; it never staged/enqueued into the live arena+HNSW, so a
   consensus-committed vector was searchable only after a restart/replay —
   asymmetric with the CRDT-sync path.

The off-by-one in the quorum denominator (`peers.len()/2` excludes this node) was
noted and corrected here too.

## Decision

Authenticate proposals end-to-end and make commit idempotent + complete.

1. **Single signature primitive.** New `verify_event_signature(&SignedEvent) ->
   bool` resolves the signer's key via `peer_verifying_key` (own key for self,
   registered key for peers) and verifies the ed25519 signature over the canonical
   `serde_json::to_vec(event)` bytes — the same bytes `persist`/`propose` sign.
   `reconcile_state` is refactored onto it (one source of truth).
2. **`propose_consensus` signs locally + governance-checks.** The proposal is
   signed with *this node's own* key (an external caller has no access to the local
   private key, so a caller-supplied signature could never be authentic — the
   `signature` param is ignored). The event must pass `semantic_verify`, else the
   proposal is rejected up front.
3. **Gossip `ConsensusPropose` verifies before storing.** A proposal whose
   embedded event signature doesn't verify against its claimed signer's registered
   key is dropped (logged), not stored.
4. **Commit re-checks, then applies once.** Before applying, `submit_vote`
   re-verifies the proposal's event signature and re-runs `semantic_verify`
   (defense-in-depth). A new `ConsensusProposal.committed` flag (`#[serde(default)]`
   for back-compat with peers on older builds) short-circuits any vote that
   arrives after commit, so the event is applied + persisted exactly once.
5. **Commit applies completely.** Edges go through `index_edge_internal`
   (+ `refresh_impacts`) so they are traversable immediately; vectors go through
   `replay_vector(index=true)` so they are searchable immediately.
6. **Quorum counts this node.** `approvals > (peers.len() + 1) / 2` — `peers`
   excludes self, so the membership denominator is peers + self.

Search-side companion fix (same review): `hybrid_search` dedupes results by node
id, since a superseded `add_vector` leaves an orphaned arena/HNSW slot until
compaction that could otherwise surface a node twice.

## Consequences

### Positive
- Proposals are unforgeable end-to-end: only an authentically signed event from a
  known peer can enter the proposal map or be committed. The MASTER-promotion
  governance guard holds across both the vote *and* proposal channels.
- `semantic_verify` is now actually enforced on the consensus path (at propose and
  at commit), not merely available as a standalone method.
- Each proposal commits once — no WAL bloat or duplicate application from late
  votes. Committed edges/vectors are immediately traversable/searchable.

### Negative / Trade-offs
- `propose_consensus` ignores its `signature` argument (kept for API stability).
  The parameter is now vestigial; a future major version may drop it.
- Lone self-promotion is still *possible* by design: a single authorized node with
  no peers can propose + self-vote to commit a MASTER axiom (the sanctioned path
  per the governance spec). Guarding that requires a separate min-quorum policy,
  deliberately deferred — the threat model here is forged/external input, which is
  closed by signature + `semantic_verify`.

## Alternatives Considered
| Alternative | Reason Rejected |
|---|---|
| Add a `sign_event` API so callers sign proposals | External callers don't hold the local private key; signing in the engine is simpler and strictly more secure. |
| Verify proposals only at the gossip layer | The NAPI/REST `propose_consensus` path would stay unguarded; verification belongs in the engine. |
| Require ≥1 remote approval (block lone self-promote) | Breaks single-node consensus commits; the governance threat is external forgery, already closed. Left as a future policy knob. |
| Fix the orphaned vector slot at `add_vector` time | Invasive (arena/metadata rewrite). Dedup-by-node-id at search is localized and compaction reclaims the slot. |

## Verification
- `tests/consensus_commit_tests.rs` (3): consensus-committed edge is traversable;
  consensus-committed vector is searchable; a post-quorum vote does not re-persist
  (WAL line count stable, `committed` guard).
- `tests/add_vector_tests.rs` (+1): re-adding a vector for the same node/collection
  does not surface the node twice in search.
- `tests/consensus_vote_sig_tests.rs` (5) still green (propose signs internally;
  quorum `(peers+1)/2` keeps single-node quorum at threshold 0).
- Full `cargo test` (24 binaries) green. NAPI surface unchanged (no `index.d.ts`
  regen; `npm test` unaffected — no JS consensus tests).

### Outcome (measured 2026-06-23)
Shipped. Proposals are ed25519-verified on propose, on gossip receipt, and at
commit; `semantic_verify` gates both ends; commit is idempotent and applies edges
and vectors completely.

---
### Related Links
- **Builds on:** [[ADR--GENESISDB-CONSENSUS-VOTE-SIGNATURES]]
- **Governance:** [[ADR--GENESISDB-GOVERNANCE-LOGIC]]
- **Edge keys:** [[ADR--GENESISDB-EDGE-NUMERIC-KEYS]]
- **Vector spaces:** [[ADR--GENESISDB-ADD-VECTOR]]

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-23 | Proposed & accepted & shipped: proposal authentication (verify on propose/gossip/commit), `semantic_verify` enforced on the consensus path, `committed` guard for once-only apply, edges/vectors committed completely (adjacency-indexed / staged), quorum denominator includes self, search dedupe by node id. |
