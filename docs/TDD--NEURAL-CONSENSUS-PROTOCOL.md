# Software Requirements Document (SRD): Multi-Agent Neural Consensus

## 1. Introduction
The **Multi-Agent Neural Consensus** module is the pinnacle of Mark VI. It enables a decentralized network of GenesisDB instances to operate as a single **Collective Brain**. It ensures that high-authority knowledge (Axioms) is only accepted into the network after passing a multi-peer validation process.

## 2. Functional Requirements

### FR1: Semantic Voting
- **Requirement:** Agents must be able to "vote" on the validity of a proposed mutation from a peer.
- **Logic:** When Agent A proposes a new `MASTER` tier node, Peer Agents (B, C, D) perform a semantic consistency check using their local `get_ranked_context`. If the new node contradicts existing high-impact axioms, the peer votes "Reject".

### FR2: Merkle-Proof Verification
- **Requirement:** Peers must use Merkle Proofs to verify the integrity of synchronized WAL segments.
- **Goal:** Zero-trust synchronization. An agent can prove it has a specific piece of knowledge without sending the entire database.

### FR3: Consensus Threshold (Axiomatic Quorum)
- **Requirement:** Define a quorum (e.g., >50% of active peers) required to promote a `USER` note to a `MASTER` axiom.
- **Goal:** Prevent single-agent hallucinations from polluting the collective truth.

## 3. Security Requirements
- **Cryptographic Signatures:** Every proposal must be signed by the originating agent's private key.
- **Slash Protocol (Future):** Agents providing consistently conflicting/false data are blacklisted from the `SyncPeer` registry.

---

# Technical Design Document (TDD): Neural Consensus Protocol

## 1. Architecture: The Gossip-Verify-Ingest Cycle
The consensus protocol operates as an extension of the `Collaborative WAL`.

## 2. Data Structures

### 2.1 Proposal Object
```rust
pub struct ConsensusProposal {
    pub proposal_id: Uuid,
    pub event: Event,
    pub signature: Vec<u8>,
    pub votes: DashMap<String, bool>, // PeerID -> Vote
}
```

## 3. Algorithm: Semantic Consistency Check
1.  **Ingest Proposal:** Peer receives `SyncEvent::ProposeMutation`.
2.  **Logic Probe:** Peer executes a `SEARCH` query using the new node's embedding.
3.  **Conflict Check:** If top 3 results have `Impact > 0.8` but `lang` or `props` logically contradict the proposal, return `Vote(Reject)`.
4.  **Finalization:** Once Quorum is reached, the `is_system` flag is set to `true`, bypassing local Axiomatic Guards for ingestion into the `MASTER` tier.

## 4. Implementation Plan
- **Step 1:** Implement the `Proposal` registry and `Sign/Verify` logic in `src/lib.rs`.
- **Step 2:** Add a `semantic_verify(&self, proposal: &Event)` helper method.
- **Step 3:** Update the Gossip listener to handle voting rounds.

---
**Please review and approve this documentation (SRD & TDD). I will generate the code once approved.**
