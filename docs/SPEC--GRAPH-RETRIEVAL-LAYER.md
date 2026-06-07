# Software Requirements Document (SRD): Graph Retrieval Layer (GRL)

## 1. Introduction
The **Graph Retrieval Layer (GRL)** transforms GenesisDB from a hybrid database into a **Cognitive Retrieval Engine (CRE)**. Instead of raw queries, AI agents interact with the GRL via the **Context Scaling Tier (H0-H5)** protocol. The GRL acts as an intelligent orchestrator that resolves the optimal knowledge radius (Hops), prioritizes high-impact nodes, and compresses context to fit within the agent's token budget.

## 2. Functional Requirements

### FR1: Context Resolver (Tier Mapping)
- The system must map semantic tiers to physical graph hops:
    - **H0 (Self):** 0 Hops. Only the target node.
    - **H1 (Neighbors):** 1 Hop. Direct imports/exports and parent/child.
    - **H2 (Feature):** 2 Hops. Local functional grouping.
    - **H3 (Module):** 3 Hops. Integration-level context.
    - **H4 (Architecture):** 4 Hops. High-level system design.
    - **H5 (Enterprise):** 5 Hops. Full knowledge base scan.

### FR2: Hybrid Semantic Expansion
- Retrieval must combine **Vector Similarity** (find relevant concepts) with **Graph Traversal** (expand into related knowledge) to build a coherent Sub-graph.

### FR3: Reasoning-Based Ranking
- Results must be ranked using a multi-factor scoring engine:
    - `Score = (SemanticSimilarity * 0.5) + (K-Impact * 0.3) + (GraphProximity * 0.2)`

### FR4: Context Budget & Compression
- The GRL must estimate token costs. If the retrieved graph exceeds the agent's `token_budget`, the system must automatically pivot to **SuperNode Retrieval** (Themes) instead of individual atoms to preserve high-level context.

---

# Technical Design Document (TDD): Cognitive Retrieval Engine

## 1. Data Structures (`src/lib.rs`)

### 1.1 `ContextPackage`
```rust
#[napi(object)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContextPackage {
    pub nodes: Vec<NodeOutput>,
    pub edges: Vec<EdgeOutput>,
    pub super_nodes: Vec<SuperNode>,
    pub token_estimate: u32,
    pub reasoning_path: String, // Traceability: "Found via Vector -> Expanded 2 Hops"
}
```

### 1.2 `ScalingTier` Enum
```rust
#[napi]
pub enum ScalingTier {
    H0 = 0, // Self
    H1 = 1, // Neighbors
    H2 = 2, // Feature
    H3 = 3, // Module
    H4 = 4, // Architecture
    H5 = 5, // Enterprise
}
```

## 2. Retrieval Pipeline Logic

### 2.1 `retrieve_context` Implementation
1.  **Input:** `seed_id/query_vector`, `tier`, `budget`.
2.  **Phase 1 (Identify):** Run HNSW search to find the "Anchor Nodes".
3.  **Phase 2 (Expand):** Perform BFS traversal starting from Anchors up to `tier.hops()`.
4.  **Phase 3 (Rank):** Apply the multi-factor scoring formula to the resulting set.
5.  **Phase 4 (Evaluate):** Estimate tokens (Approx 4 chars per token).
6.  **Phase 5 (Prune/Compress):** 
    - If `cost > budget`: Replace low-impact nodes with their cluster's `SuperNode`.
7.  **Phase 6 (Pack):** Return the `ContextPackage`.

## 3. HQL Grammar Update (`hql.pest`)
```pest
context = { ^"CONTEXT" ~ ^"FOR" ~ target ~ ^"TIER" ~ tier ~ (^"BUDGET" ~ budget)? }
tier = { "H0" | "H1" | "H2" | "H3" | "H4" | "H5" }
```

---

## 4. Change Risk Assessment
- **Risk:** **HIGH**
- **Impact:** Significant architectural shift in how data is retrieved. affects public API.
- **Complexity:** **C-3** (Architecture-Driven)

## 5. Definition of Done (DoD)
1.  [ ] `retrieve_context` logic implemented and exposed via NAPI/REST.
2.  [ ] HQL `CONTEXT` command successfully parsed and executed.
3.  [ ] **H1 Verification:** Retrieving H1 context for a node returns only its direct neighbors.
4.  [ ] **Budget Verification:** Retrieving context with a tiny budget triggers SuperNode fallback.
5.  [ ] **Ranking Verification:** High-impact nodes are prioritized even if semantically secondary.

---
**Please review and approve this Specification. I will begin the implementation once approved.**
