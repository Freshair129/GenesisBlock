# Software Requirements Document (SRD): Causality Chains (Event Sourcing)

## 1. Introduction
The **Causality Chains** module (Mark VII, Step 2) transforms GenesisDB from a state-based database into an event-sourced reasoning engine. It enables the system to not only track *when* knowledge changed (achieved in Step 1 via Temporal Queries) but *why* it changed. By explicitly linking state mutations to the events, documents, or agents that caused them, the engine achieves absolute auditability and explainability.

## 2. Functional Requirements

### FR1: Causality Tracking (The "Why" Field)
- **Requirement:** Every mutation (Node/Edge addition, modification, or retraction) must record the ID of the event or document that triggered it.
- **Implementation:** Introduce a `caused_by` field to all write operations and output structs.

### FR2: Event Sourcing API
- **Requirement:** Agents must be able to query the lineage of a specific node.
- **Query:** "Show me the chain of events that led to the current state of Concept A."

### FR3: Supersede Logic (Immutable Updates)
- **Requirement:** True updates must not exist. An "update" is a retraction of the old state (setting `valid_to`) and an insertion of a new state, with both states sharing the same core identity but different temporal lifespans and `caused_by` pointers.

---

# Technical Design Document (TDD): Event Sourcing Implementation

## 1. Architecture: The Audit Trail
The Write-Ahead Log (WAL) already acts as a primitive event store. This phase elevates that by embedding the causality directly into the graph topology, allowing the HNSW index to serve audit queries.

## 2. Data Structure Updates (`src/lib.rs`)

### 2.1 Struct Expansion
```rust
pub struct NodeInput {
    // ... existing fields ...
    pub caused_by: Option<String>, // ID of the ADR, SPEC, or Agent
}

pub struct NodeOutput {
    // ... existing fields ...
    pub caused_by: Option<String>,
}

pub struct EdgeInput {
    // ... existing fields ...
    pub caused_by: Option<String>,
}

pub struct EdgeOutput {
    // ... existing fields ...
    pub caused_by: Option<String>,
}
```

## 3. Core Logic: The `supersede_node` Method
To fully realize Event Sourcing, we must introduce a native "update" method that respects immutability.

```rust
impl Storage {
    pub fn supersede_node(&self, id: String, new_props: Value, caused_by: String) -> Result<NodeOutput> {
        // 1. Find active state of `id`.
        // 2. Cap its `valid_to` to Utc::now().
        // 3. Insert new NodeOutput with updated props, `valid_from` = Utc::now(), and `caused_by` = caused_by.
        // 4. Persist both events to WAL.
    }
}
```

## 4. Implementation Plan
- **Step 1:** Add `caused_by` to `NodeInput`, `NodeOutput`, `EdgeInput`, and `EdgeOutput`.
- **Step 2:** Implement the `supersede_node` method in `src/lib.rs` to enforce immutable versioning.
- **Step 3:** Expose `supersede_node` via NAPI.
- **Step 4:** Add a REST endpoint `/v1/node/supersede` to `src/main.rs`.

---
**Please review and approve this documentation (SRD & TDD). I will generate the code once approved.**
