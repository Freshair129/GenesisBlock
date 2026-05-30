---
id: ADR--GENESISDB-SCALABILITY-VALIDATION
phase: 3
type: adr
status: proposed
vault_id: GKS-CORE
tier: process
source_type: learned
title: "ADR: Architectural Validation of 500M Edge Scalability in Embedded Environments"
tags: [architecture, genesisdb, scalability, memory, hardware]
aliases: [genesisdb-scalability-proof]
attributes:
  domain: systems-engineering
---

# ADR--GENESISDB-SCALABILITY-VALIDATION

## 1. Context
GenesisDB claims a scalability target of **50 Million Nodes** and **500 Million Edges** within a single-machine embedded environment. For this to be defensible, we must provide a mathematical proof of memory consumption and an architectural strategy to overcome the limits of standard 64-bit pointers.

## 2. Decision
We implement **Pointer Compression** and **Arena-based Addressing** to achieve high density.

### 2.1 Pointer Compression (32-bit Internal IDs)
To avoid the 8-byte overhead of native 64-bit pointers, GenesisDB uses **Internal Arena Indices** (u32).
*   **Node/Edge Addressing:** By using 32-bit unsigned integers for internal links, we can address up to $2^{32} \approx 4.2$ Billion entities while consuming only 4 bytes per link.
*   **Storage Calculation:**
    *   **Edge Metadata:** 10 bytes (Target Node ID + Rel Type + Temporal).
    *   **Adjacency Entry:** 4 bytes (Internal Index).
    *   **Total Per Edge:** ~14 bytes.
    *   **Scale at 500M Edges:** $500 \times 10^6 \times 14 \text{ bytes} \approx 7 \text{ GB}$.

### 2.2 Memory-Aligned Node Arenas
Nodes are stored in a contiguous `NodeArena`. 
*   **Layout:** Each node metadata block is exactly 32 bytes (aligned to cache lines).
*   **Scale at 50M Nodes:** $50 \times 10^6 \times 32 \text{ bytes} = 1.6 \text{ GB}$.

### 2.3 Verified Envelope
A system with **50M Nodes and 500M Edges** will have a baseline in-memory footprint of **~8.6 GB** (excluding property values and hash index overhead). This is comfortably within the reach of modern workstations and cloud instances with 32GB - 64GB RAM.

## 3. Status
**Proposed**

## 4. Consequences
*   **Positive:** Proves that GenesisDB is "Systemically Fit" for large-scale cognitive graphs without requiring distributed clusters.
*   **Negative:** Hard limit of 4.2 Billion entities due to 32-bit addressing. Transitioning to 64-bit later would double the memory footprint.
