# Software Requirements Document (SRD): Multi-Dimensional Temporal Queries

## 1. Introduction
The **Multi-Dimensional Temporal Queries** module is the foundational step of GenesisDB Mark VII. It introduces the dimension of "Time" to the knowledge graph. By enabling Time-Travel Queries, AI agents can retrieve historical states of knowledge, allowing them to reason about how concepts have evolved or what the accepted truth was at a specific moment in the past.

## 2. Functional Requirements

### FR1: Historical State Retrieval (Time-Travel)
- **Requirement:** The engine must support querying the graph as it existed at a specific timestamp.
- **Logic:** Filter nodes and edges where the requested timestamp falls within their `valid_from` and `valid_to` lifespan.

### FR2: HQL Temporal Expansion
- **Requirement:** Introduce the `AS OF` keyword to the HQL grammar.
- **Syntax:** `TRAVERSE FROM "NodeA" DEPTH 2 REL ANY AS OF "2026-01-01T12:00:00Z"`

### FR3: Immutable Versioning
- **Requirement:** Mutations to existing nodes/edges must not overwrite historical data. Instead, they must trigger a "Retraction and Re-insertion" pattern, capping the old state's `valid_to` and creating a new state with an updated `valid_from`.

---

# Technical Design Document (TDD): Temporal Engine Implementation

## 1. Architecture: Bitemporal Data Model
GenesisDB already contains rudimentary bitemporal fields (`valid_from`, `valid_to`) in the `EdgeOutput` struct. Mark VII will operationalize these fields and extend them to `NodeOutput`.

## 2. Data Structure Updates (`src/lib.rs`)

### 2.1 NodeOutput Expansion
```rust
pub struct NodeOutput {
    // ... existing fields ...
    pub valid_from: String,
    pub valid_to: Option<String>,
}
```

## 3. Query Execution Logic

### 3.1 AST Expansion (`src/query/ast.rs`)
- Add `as_of: Option<String>` to `Search`, `Traverse`, and `Hybrid` variants in `HqlCommand`.

### 3.2 Traversal Filtering (`src/lib.rs`)
In the `neighbors` and `hybrid_search` methods:
- Parse the `as_of` string into a UNIX timestamp.
- For every node and edge encountered during traversal, verify:
  `valid_from <= as_of < valid_to` (if `valid_to` is present).
- If the check fails, exclude the entity from the result path.

## 4. Implementation Plan
- **Step 1:** Add temporal fields to `NodeInput` and `NodeOutput`. Update `add_node` to initialize `valid_from`.
- **Step 2:** Update `hql.pest` grammar to support `AS OF <string_lit>`.
- **Step 3:** Refactor `ast.rs` to parse the `as_of` timestamp.
- **Step 4:** Implement temporal filtering logic in `hybrid_search` and `neighbors`.
- **Step 5:** Add `test_temporal_queries` to verify time-travel traversal.

---
**Please review and approve this documentation (SRD & TDD). I will generate the code once approved.**
