# ADR--PHASE-11-INDUSTRIAL-HARDENING

## 1. Status
**Proposed / Active**

## 2. Context
Previous phases demonstrated high performance on synthetic or small datasets (10k nodes). However, professional database certification requires proof of stability and consistent performance on full-scale industry-standard datasets (LDBC SNB SF0.1) under realistic operational stress (crashes, mixed workloads, and long-duration runs).

## 3. Decision
We will prioritize **Industrial Resilience** over new features (like Distributed Mode) for Phase 11. This phase will serve as the 'Final Stress Audit' for the Mark II engine.

## 4. Key Performance Indicators (KPIs)
- **Scale:** Full LDBC SNB SF0.1 (327K Nodes / 1.5M Edges).
- **Latency Distribution:** Capture Mean, P50, P95, and P99 for Hybrid Queries. Target P99 < 10ms on full dataset.
- **Mixed Workload:** 80% Read / 20% Write concurrent ratio.
- **Crash Recovery:** 100% data consistency after an ungraceful shutdown (Kill -9).
- **Memory Leakage:** Zero heap growth over a 1-hour sustained stress cycle.

## 5. Implementation Strategy
1. **Full-Scale Loader:** Update benchmark scripts to utilize the full SF0.1 dataset.
2. **Concurrent Stressor:** Implement a multi-threaded 'Stressor' in Rust that mixes HQL queries with bulk writes.
3. **Crash Simulation:** Automated script to interrupt the engine during heavy writes and verify WAL replay integrity.
4. **Instrumentation:** Use \criterion\ for statistical latency analysis and \jemalloc-ctl\ or OS tools for memory tracking.

## 6. Consequences
- **Pro:** Final confirmation of 'Production Ready' status.
- **Pro:** Identification of tail-latency bottlenecks (P99).
- **Con:** Delayed progress on Distributed Mode.

## 7. Strategic Importance
This phase transforms GenesisDB from a "high-speed engine" into a "reliable infrastructure component" that can be trusted with mission-critical knowledge.
