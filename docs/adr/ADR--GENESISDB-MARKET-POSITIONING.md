# ADR--GENESISDB-MARKET-POSITIONING

## 1. Status
**Accepted / Active**

## 2. Context
Following a rigorous CTO-level review of the Phase 8-12 audits and the current source codebase, it was identified that while the architecture (Interior Mutability, Lock Sharding, WAL) is structurally sound, the implementation was incomplete (stubbed query methods) during the benchmark phase. Furthermore, attempting to compete directly with mature Enterprise Graph/Vector databases (Neo4j, Qdrant, TigerGraph) requires features currently out of scope (Distributed clustering, robust query planners, replication).

## 3. Decision
We are officially pivoting the market positioning of GenesisBlockDB.
**OLD POSITIONING:** "Enterprise-Grade Hybrid Semantic-Graph Database"
**NEW POSITIONING:** "Embedded analytics / agent-memory graph + vector engine"
(refined from "Local Hybrid Knowledge Engine for AI Agents").

**Comparator set (refined 2026-06-21):** nearest peers are **Kuzu, DuckDB
(graph extension), RocksDB + graph layer** (embedded graph); **Chroma / LanceDB**
(embedded vector); Neo4j / Qdrant are well-known *references*, not the category.

**Measurement status (be honest about which comparators are actually measured):**

| Comparator        | Category        | Head-to-head status            |
|-------------------|-----------------|--------------------------------|
| Chroma            | embedded vector | ✅ measured (P15, P21)         |
| Qdrant            | server vector   | ✅ measured (P20)              |
| LanceDB           | embedded vector | ✅ measured (P27)              |
| Neo4j             | server graph    | ✅ measured (P23)              |
| Kuzu              | embedded graph  | ✅ measured (P26)              |
| DuckDB + graph    | embedded graph  | ✅ measured (P28)              |
| RocksDB + graph   | embedded graph  | ✅ measured (P29)              |
| LadybugDB         | embedded graph+vec (Kuzu fork) | ✅ measured (P30) |

**All named comparators are now measured (P15–P30)** — including LadybugDB, the
Kuzu fork that sits squarely on this project's niche (embedded graph+vector for
agentic memory). Future systems added
to this list must carry a ⏳ pending marker with a P-number until their
head-to-head audit lands — do not cite a named-but-unmeasured peer as evidence.

> **Evidence update (2026-06-21):** the re-benchmarking this ADR called for is
> done for four comparators — vector vs Chroma/Qdrant (recall-latency frontier),
> graph traversal 10k–1M, Neo4j head-to-head (embedded 7–185× on k-hop), Kuzu
> head-to-head (embedded↔embedded), and governance / incremental-K-Impact cost
> proofs, plus LanceDB embedded-vector (P27), DuckDB+graph (P28), RocksDB+graph
> (P29) and **LadybugDB** (P30, the Kuzu fork on this project's exact niche)
> head-to-heads — **all named comparators measured.** See
> `REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md` (audits P14–P30). The
> "oversell" figures (P12) are formally retracted; current claims are measured.

## 4. Rationale
By targeting the "Local Knowledge Engine" market, GenesisBlockDB competes against systems like Chroma, LanceDB, and local GraphRAG implementations. In this arena, GenesisBlockDB's unique strengths provide a massive competitive advantage:
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
