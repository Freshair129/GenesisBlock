---
id: ADR--GENESISDB-TEMPORAL-MODEL
phase: 2
type: adr
status: stable
vault_id: GKS-CORE
tier: process
source_type: learned
title: "ADR: Full Bi-Temporal Graph Support with Value-History Arenas"
tags: [architecture, genesisdb, temporal, versioning, storage]
aliases: [genesisdb-temporal-model]
created_at: 2026-05-30T03:00:00+07:00
crosslinks:
  references: [GENESIS--BACKEND-ENGINE]
attributes:
  domain: storage-engine
---

# ADR--GENESISDB-TEMPORAL-MODEL

## Context
Current GenesisDB versions implement bi-temporal metadata primarily on edges (`valid_from`, `valid_to`). However, a truly robust cognitive engine requires **Temporal Node Properties** to track the evolution of concepts and attributes over time without destructive overwrites. Standard graph databases often struggle with "Property Versioning," leading to data duplication or loss of historical context.

## Decision
We implement a **Full Bi-Temporal Graph Model** using **Value-History Arenas (VHA)** and **Version Chains**.

### 2.1 Value-History Arena (VHA)
Node properties are no longer stored as single JSON blobs. Instead, the `NodeArena` stores a `tail_ptr` to a Version Chain in a dedicated `ValueArena`.
*   **Version Chain Entry:** `(Timestamp, ValueOffset, PrevPtr)`.
*   **Logical Immutability:** Updating a property appends a new entry to the `ValueArena` and updates the node's `tail_ptr`. The old state remains physically accessible.

### 2.2 Bi-Temporal Dimensions
The engine supports three distinct time axes:
1.  **Logical Time (Valid-Time):** When the knowledge was considered "true" in the AI agent's domain.
2.  **Epistemic Time (Snapshot-Time):** The specific "as-of" view used during a reasoning traversal.
3.  **Transaction Time (Recorded-Time):** When the database accepted the write.

### 2.3 Snapshot Semantics
Traversals are parameterized with an `epistemic_at` timestamp. The engine's iterators perform a **Point-in-Time Point-Lookup**:
*   For edges: Filter where `valid_from <= epistemic_at < valid_to`.
*   For node properties: Follow the Version Chain starting from `tail_ptr` until the first entry where `recorded_at <= epistemic_at`.

## Consequences
*   **Positive:** Enables 100% reproducible reasoning. An agent can query "What did I believe about CONCEPT-X at T=Yesterday?" and get the exact graph topology AND properties.
*   **Negative:** Increased storage overhead. Binary snapshots will grow linearly with the frequency of property updates. Requires a strict **Historical Pruning Policy** for long-running systems.

---
### Related Links
- **Orchestrator:** [[GENESIS--BACKEND-ENGINE]]
- **Storage Strategy:** [[ADR--GENESISDB-CSR-MUTATION-STRATEGY]]
- **Scalability Proof:** [[ADR--GENESISDB-SCALABILITY-VALIDATION]]
