# Functional Specification: Collaborative WAL & Decentralized Sync (Mark V)

## 1. Objective
Transform GenesisDB from a siloed engine into a **Collaborative Knowledge Substrate**. This feature allows multiple GenesisDB instances (Agents) to synchronize their Write-Ahead Logs (WAL) in a peer-to-peer fashion, ensuring that "Axioms" and "Knowledge Atoms" discovered by Agent A are securely propagated to Agent B.

## 2. Technical Approach: The Gossip Protocol
We will implement a lightweight **Gossip Protocol** to handle real-time synchronization.

### 2.1 Event Propagation
- When a `MASTER` or `SPEC` tier node is added, the engine broadcasts a `SyncEvent` to all registered peer addresses.
- Peers validate the event using their own **Axiomatic Guards** before ingestion.

### 2.2 Merkle Tree Reconciliation (Anti-Entropy)
To handle cases where an agent was offline, we will use **Merkle Trees** for state verification:
1.  Each agent maintains a hash tree of its WAL entries.
2.  During a periodic "Sync Handshake," agents compare the root hashes of their Merkle Trees.
3.  If hashes differ, they traverse the tree to identify the specific missing or conflicting segments and exchange only those fragments.

## 3. Data Structures

### `SyncPeer` Struct
```rust
pub struct SyncPeer {
    pub id: String,
    pub addr: String, // e.g., "127.0.0.1:8081"
    pub public_key: Vec<u8>, // For Axiomatic Signature verification
}
```

### `SyncEvent` Enum
```rust
pub enum SyncEvent {
    ProposeMutation(Event),
    AcknowledgeMutation(u32), // Hash of the event
    RequestFragment(u32), // Merkle node hash
}
```

## 4. Proposed Changes
- **src/lib.rs:**
    - Add `peers: DashMap<String, SyncPeer>`.
    - Implement `pub fn register_peer(&self, peer: SyncPeer)`.
    - Update `persist` logic to optionally trigger a gossip broadcast.
- **src/sync/mod.rs (New):**
    - Implement the Gossip listener (UDP/TCP).
    - Implement Merkle Tree generation for the WAL.

## 5. Security: Axiomatic Signatures
To prevent malicious agents from injecting false knowledge:
- Mutations to `MASTER` tier nodes must be cryptographically signed.
- Agents will reject any `MASTER` sync event with an invalid signature.

## 6. Implementation Roadmap
1.  **Step 1:** Define the `SyncPeer` and `SyncEvent` types.
2.  **Step 2:** Implement the Merkle Tree hashing logic for the existing JSONL WAL.
3.  **Step 3:** Implement a basic Gossip listener in `src/sync`.
4.  **Step 4:** Add a test case `test_collaborative_sync` simulating two local DB instances.

Please review and approve this Collaborative Sync Specification. I will generate the code once approved.
