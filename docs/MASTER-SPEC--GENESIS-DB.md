# GenesisBlock DB: Master Specification (v2.0.0)

## 1. Abstract
GenesisBlock DB (GenesisDB) is a high-performance, embedded hybrid semantic-graph database engine written in Rust. It is designed as the "Shadow Brain" for human-machine collaboration, providing a unified substrate for structured graph relationships, unstructured vector embeddings, and bitemporal event sourcing.

## 2. Core Architecture

### 2.1 Storage Model
GenesisDB uses a **Log-Structured Merge-Friendly** architecture based on a Write-Ahead Log (WAL).
- **Primary Log:** `genesis-graph.wal` (JSONL format) stores all mutation events.
- **Persistence:** High-durability append-only logic with batched group commits.
- **In-Memory State:**
    - `DashMap<u32, NodeOutput>`: Primary node storage.
    - `DashMap<u32, EdgeOutput>`: Primary edge storage.
    - `Adjacency Indices`: Forward (`out_idx`) and Backward (`in_idx`) mapping for $O(1)$ traversal.

### 2.2 Semantic Hybrid Indexing
GenesisDB bridges lexical and semantic search via a dual-indexing strategy:
1.  **Lexical Index (Trigrams/Bigrams):** Thai-aware tokenization that strips combining marks (vowels/tones) to provide high-recall fuzzy matching for terms like "บ้าน" vs "บาน".
2.  **Vector Index (HNSW):** Hierarchical Navigable Small Worlds index for high-dimensional vector proximity search.
3.  **Neural Bridge:** Multi-lingual support via language centroids and mean-centering, allowing English queries to match Thai contexts.

### 2.3 Graph Retrieval Layer (GRL)
The GRL implements the **Context Scaling Tier (H0-H5)** protocol to govern agent context acquisition:
- **Resolver:** Maps tiers (H0=Self, H1=Neighbors, H2=Feature, H3=Module, H4=Arch, H5=System) to graph hops.
- **Budget Manager:** Estimates token usage and automatically compresses results to **SuperNodes** if the agent's `BUDGET` is exceeded.
- **Orchestrator:** Combines vector anchors with tiered graph expansion in a single reasoning pipeline.

## 3. Data Model & Bitemporality

### 3.1 Node Schema
| Field | Type | Description |
| :--- | :--- | :--- |
| `id` | String | Unique Identifier (supports Trigram indexing). |
| `labels` | Vec<String> | Governance and classification tags. |
| `props` | JSON | Arbitrary metadata payload. |
| `impact` | f64 | Reasoned importance score (K-Impact). |
| `embedding`| Vec<f64> | 1536-dim vector (optional). |
| `valid_from`| RFC3339 | Logical start time. |
| `valid_to` | Option<RFC3339>| Logical end time (for retractions/supersessions). |
| `expires_at`| Option<RFC3339>| TTL expiration time. |
| `caused_by` | Option<String> | Link to triggering event (Causality Chain). |
| `clock` | LogicalClock | Lamport timestamp (CRDT support). |

### 3.2 Bitemporal Philosophy
GenesisDB follows an **immutable-by-default** update pattern. Updates use the `supersede_node` logic:
1.  The existing node version is marked with `valid_to = now`.
2.  A new node version is inserted with `valid_from = now` and the updated properties.
3.  The `caused_by` field links the mutation to its causal context (e.g., an ADR document).

## 4. Reasoning & Autonomic Substrate

### 4.1 K-Impact Model
Node importance is calculated via the formula:
$$ R(n) = (DD(n) \cdot 0.5) + (AS(n) \cdot 0.3) + (SC(n) \cdot 0.2) $$
- **DD (Dependency Depth):** Incoming link count.
- **AS (Axiomatic Strictness):** Governance tier weight.
- **SC (Stability Confidence):** Reliability of the data source.

### 4.2 Structural Insight Engine
The autonomic maintenance loop runs periodically to:
1.  **Community Detection:** Group nodes into clusters using the Label Propagation Algorithm (LPA).
2.  **SuperNode Generation:** Synthesize cluster centroids and themes.
3.  **Gap Detection:** Identify semantically close but physically disconnected knowledge clusters.
4.  **Vector Drift:** Track the movement of cluster centroids over time to detect "Semantic Shift".

## 5. Governance & Consensus

### 5.1 Axiomatic Guards
Access control is enforced via governance tiers:
- **MASTER (Tier 0):** Read-only for external agents. Mutated only via consensus.
- **SPEC (Tier 1):** Mandatory temporal fields and audit logs.
- **ADR (Tier 2):** Architecture decisions linking to SPEC.
- **USER (Tier 3):** Unstructured user data.

### 5.2 Multi-Agent Consensus
The `ConsensusProposal` protocol allows agents to vote on promoting USER data to MASTER axioms based on quorum and semantic verification.

## 6. Distributed Synchronization (CRDT)

GenesisDB ensures eventual consistency across distributed agents using:
- **Lamport Timestamps:** `LogicalClock { time, peer_id }` for global event ordering.
- **LWW (Last-Write-Wins):** Deterministic conflict resolution during `reconcile_state`.
- **Clock Jump:** Local clocks synchronize with the maximum known global time upon replication.

## 7. HQL (Hybrid Query Language)

GenesisDB exposes a specialized language for reasoning over graph and vector data.

### 7.1 Search (Lexical/Vector)
```sql
SEARCH ~target SIMILAR TO [v1, v2, ...] K 5 LANGUAGE "th" AS OF "2026-01-01T00:00:00Z"
```

### 7.2 Traverse (Graph)
```sql
TRAVERSE FROM seed DEPTH 2 REL INFER(depends_on) AS OF "..."
```

### 7.3 Hybrid (Ranked Context)
```sql
MATCH target SIMILAR TO [...] ALPHA 0.4 LANGUAGE "en"
```

## 8. Deployment & Connectivity
- **Rust Core:** Compiled to native binary or CDYLIB.
- **NAPI-RS:** Exposes asynchronous bindings for Node.js/TypeScript.
- **Axum REST Server:** Provides a standalone `/v1/` API for remote agent interaction.
- **Obsidian Bridge:** Integration with the Obsidian "Shadow Sync" plugin for human visualization.
