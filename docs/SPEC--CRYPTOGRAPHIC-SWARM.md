# Software Requirements Document (SRD): Swarm Hardening & Cryptographic Identity (Mark X)

## 1. Introduction
GenesisDB currently allows any network participant to join the Gossip swarm and propose mutations. **Mark X** introduces a cryptographic trust layer. By assigning every agent a unique ed25519 identity, the system can verify the origin of knowledge atoms and enforce strict governance rules based on cryptographic proofs.

## 2. Functional Requirements

### FR1: Peer Identity (ed25519)
- Every GenesisDB instance must generate a unique asymmetric keypair upon first initialization.
- The `PeerID` will be derived from the SHA-256 hash of the public key.

### FR2: Signed Mutations
- All mutations (`Node`, `Edge`, `Batch`) created by an agent must be digitally signed using its private key.
- Receiving peers must verify the signature before applying the mutation to their local state.

### FR3: Axiomatic Quorum (Master Tier Enforcement)
- Transitions to the **MASTER** tier (Axioms) must be accompanied by a multi-signature proof proving that a quorum of peers has approved the change.

---

# Technical Design Document (TDD): Cryptographic Substrate

## 1. Data Structures (`src/lib.rs`)

### 1.1 Identity Storage
```rust
pub struct Storage {
    // ...
    pub private_key: ed25519_dalek::SigningKey,
    pub public_key: ed25519_dalek::VerifyingKey,
}
```

### 1.2 `SignedEvent` Wrapper
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedEvent {
    pub event: Event,
    pub signature: Vec<u8>,
    pub signer_peer_id: String,
}
```

## 2. Implementation Logic

### 2.1 Identity Generation
On `Storage::open`, check if `identity.bin` exists. If not, generate a new ed25519 keypair and save it.

### 2.2 Signing Pipeline
In `Storage::persist`, automatically wrap the `Event` into a `SignedEvent` using the local private key.

### 2.3 Verification Pipeline
In `Storage::reconcile_state`, for every received event:
1.  Lookup the `signer_peer_id` in the `peers` DashMap.
2.  Verify the signature against the event bytes.
3.  Reject if verification fails.

---

## 3. Definition of Done (DoD)
1.  [ ] ed25519 identity generation and persistence implemented.
2.  [ ] All Gossip messages and WAL entries are digitally signed.
3.  [ ] **Forgery Test:** Attempt to push a mutation to Peer B using Peer A's ID but an invalid signature -> Verify rejection.
4.  [ ] **Quorum Test:** Verify that MASTER tier promotion fails without enough valid signatures.

---
**Please review and approve this Mark X Architecture Blueprint. I will begin the implementation once approved.**
