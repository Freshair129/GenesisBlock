---
id: ADR--GENESISDB-CSR-MUTATION-STRATEGY
phase: 2
type: adr
status: stable
vault_id: GKS-CORE
tier: process
source_type: learned
title: "ADR: Chunked-CSR with Slack Space for High-Throughput Graph Mutations"
tags: [architecture, genesisdb, storage, csr, performance]
aliases: [genesisdb-csr-mutation]
created_at: 2026-05-30T02:00:00+07:00
crosslinks:
  references: [GENESIS--BACKEND-ENGINE]
attributes:
  domain: storage-engine
---

# ADR--GENESISDB-CSR-MUTATION-STRATEGY

## Context
GenesisDB v1.x relies on a Compressed Sparse Row (CSR) layout for ultra-low latency graph traversals. While standard CSR guarantees $O(1)$ read complexity by keeping adjacency lists contiguous in memory, it suffers from catastrophic $O(E)$ time complexity on writes (mutations), requiring massive memory relocations for every new edge. Given our operational target of 25,000 Ops/sec, standard CSR is inviable.

## Decision
We adopt a **Chunked-CSR with Slack Space (Over-provisioning) Strategy** combined with **Out-of-Place Relocation**.

### 2.1 The Chunked-CSR Mechanism
1.  **Slack Space Allocation:** When a node is created or an adjacency list grows, the `EdgeArena` allocates contiguous space equal to `ceil(current_degree * 1.5)`. This 50% slack space allows subsequent edge inserts to be pure $O(1)$ memory assignments.
2.  **Out-of-Place Relocation:** When a node's adjacency slack is exhausted:
    *   A new block of size `ceil(new_degree * 1.5)` is allocated at the *tail* of the `EdgeArena`.
    *   Existing edges are `memcpy`'d to the new location.
    *   The `NodeArena`'s `start_offset` pointer is updated atomically.
    *   The old memory block is marked as a "hole" in a Free List.

### 2.2 Fragmentation & Garbage Collection
*   Relocation causes external fragmentation in the `EdgeArena`.
*   **Compaction GC:** When fragmentation exceeds 20% of the arena size, a background thread (during the next WAL compaction cycle) performs an `in-place defragmentation`. It collapses holes by sliding active edge blocks to the left and updating pointers in the `NodeArena`.

## Consequences
*   **Positive:** Retains $O(1)$ read performance (cache-friendly contiguous arrays) while elevating write throughput to amortized $O(1)$. Meets the 25k Ops/sec benchmark target.
*   **Negative:** The 50% slack space policy increases the in-memory footprint overhead (Memory Amplification). A 500M edge graph will require ~7.5GB of RAM instead of ~5GB.

---
### Related Links
- **Orchestrator:** [[GENESIS--BACKEND-ENGINE]]
- **Scalability Proof:** [[ADR--GENESISDB-SCALABILITY-VALIDATION]]
- **Performance Report:** [[AUDIT--GENESIS-DB-LDBC-LITE-REPORT]]
