---
proposed_id: ADR--GENESISDB-BENCHMARK-SUITE
type: adr
status: candidate
aliases:
  - ADR
phase: 2
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-05-30T19:37:59.707Z
proposed_by: agent
---

# ADR--GENESISDB-BENCHMARK-SUITE

## Context
To compare GenesisDB against established engines like Neo4j or TigerGraph, we require a benchmarking methodology that is scientifically sound, reproducible, and immune to "marketing bias." Generic random graphs do not accurately simulate the "scale-free" nature of human knowledge systems, and simple average latencies hide catastrophic tail-latency issues (P99).

## Decision
We implement a **Rigorous Native Benchmarking Suite** integrated into the Rust toolchain.

### 2.1 Synthetic Topology Generation
Instead of Erdős–Rényi (pure random) graphs, we use the **Barabási–Albert (BA) Preferential Attachment Model**.
*   **Rationale:** Real-world knowledge graphs (like GKS) exhibit a power-law degree distribution (hubs and authorities). BA graphs provide a realistic stress test for traversal algorithms and K-Impact hotspots.

### 2.2 Measurement Framework (Criterion.rs)
We adopt **Criterion.rs** as the primary measurement engine.
*   **Statistical Rigor:** Automatically detects outliers and provides 95% confidence intervals.
*   **Cold/Warm Cache Simulation:** Tests are executed in two phases:
    1.  **Cold-Start:** Measured after OS page cache invalidation.
    2.  **Warm-Steady-State:** Measured after 10,000 warm-up iterations.

### 2.3 Key Performance Indicators (KPIs)
The suite must report:
*   **Traversal Latency (P50, P95, P99):** At depths of $D=1, 2, 3$ hops.
*   **Mutation Throughput:** Measured in Edges-per-Second (EPS) under the "Chunked-CSR" model.
*   **Memory Efficiency:** Bytes-per-Edge (BPE) ratio.

## Consequences
*   **Positive:** GenesisDB performance claims become peer-reviewable by external database engineers. Provides immediate feedback during CI on any "latency regression" caused by new logic.
*   **Negative:** Generating large BA graphs (e.g., 10M nodes) for a test run consumes significant time and RAM during the setup phase.

---
### Related Links
- **Orchestrator:** [[GENESIS--BACKEND-ENGINE]]
- **Performance Report:** [[AUDIT--GENESIS-DB-LDBC-LITE-REPORT]]
- **Scalability Proof:** [[ADR--GENESISDB-SCALABILITY-VALIDATION]]
