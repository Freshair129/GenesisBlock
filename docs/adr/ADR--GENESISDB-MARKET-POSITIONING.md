# ADR--GENESISDB-MARKET-POSITIONING

## 1. Status
**Accepted / Active**

## 2. Context
Following a rigorous CTO-level review of the Phase 8-12 audits and the current source codebase, it was identified that while the architecture (Interior Mutability, Lock Sharding, WAL) is structurally sound, the implementation was incomplete (stubbed query methods) during the benchmark phase. Furthermore, attempting to compete directly with mature Enterprise Graph/Vector databases (Neo4j, Qdrant, TigerGraph) requires features currently out of scope (Distributed clustering, robust query planners, replication).

## 3. Decision
We are officially pivoting the market positioning of GenesisDB.
**OLD POSITIONING:** "Enterprise-Grade Hybrid Semantic-Graph Database"
**NEW POSITIONING:** "Local Hybrid Knowledge Engine for AI Agents"

## 4. Rationale
By targeting the "Local Knowledge Engine" market, GenesisDB competes against systems like Chroma, LanceDB, and local GraphRAG implementations. In this arena, GenesisDB's unique strengths provide a massive competitive advantage:
- **Zero-Config Embedded Nature:** NAPI-RS integration allows it to run seamlessly inside Node.js/TypeScript environments (like Obsidian or standalone Agent runtimes).
- **Dual-Track Schema:** Native support for both Markdown (Human PKM) and internal Binary (Machine Speed).
- **Hybrid Core:** Combining Graph traversals with Semantic search natively, a feature most local vector databases lack.
- **Performance:** Even as an "Early Alpha Engine", its theoretical throughput outpaces standard local SQLite/Chroma setups for complex relational-semantic queries.

## 5. Next Steps
1.  **Code Integrity Restoration:** Remove all stubbed methods (\query\, \hybrid_search\, \execute_hql\) and fully implement their logic within the new Interior Mutability architecture.
2.  **Re-Benchmarking:** Conduct the \scientific_audit.rs\ again, focusing on true end-to-end latency of the fully implemented query methods, acknowledging that P50 will reflect actual traversal + SIMD calculation time, not just lock acquisition.
3.  **Agent Integration Focus:** Shift Phase 13 focus from Multi-Node Clustering to direct integration with Local AI Agent frameworks (e.g., LangChain, MSP Orchestrators).

## 6. Consequences
- We acknowledge the "Oversell" of previous benchmarks and commit to "Extraordinary Evidence for Extraordinary Claims".
- The project scopes down from "Replacing Neo4j" to "Empowering Local AI".
