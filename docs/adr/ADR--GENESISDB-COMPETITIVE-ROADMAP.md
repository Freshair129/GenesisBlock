# ADR--GENESISDB-COMPETITIVE-ROADMAP

## 1. Status
**Draft / Proposed**

## 2. Context
The Phase 8 audit revealed a significant performance gap between GenesisDB (~3.7k vec/sec) and industry leaders like Qdrant/Weaviate (50k - 200k vec/sec). Additionally, current memory usage (~2.3 KB/record) limits scalability to SF100 levels. We must choose between maintaining the current developer-friendly architecture or performing a radical overhaul for competitive performance.

## 3. Comparison of Architectural Paths

### Path A: Rich-Object Prototype (Current)
*   **Characteristics:** String-based IDs, JSON-heavy metadata, Coarse-grained \RwLock\.
*   **Pros:** 
    *   Highly readable and debuggable.
    *   Zero-config schema changes (JSON-first).
    *   Easy integration with JavaScript via NAPI-RS.
*   **Cons:** 
    *   High memory overhead (String pointers + JSON fragmentation).
    *   Serial bottleneck (Lock contention).
    *   50x slower than dedicated Vector DBs.

### Path B: High-Density Sharded Engine (Target)
*   **Characteristics:** ID Interning (\String\ -> \u32\), Bit-packed metadata, Sharded Locks (16+ shards), SIMD-accelerated HNSW.
*   **Pros:** 
    *   **Scale:** Can handle SF100 (320M nodes) on a single workstation.
    *   **Speed:** Target 50,000+ TPS and 50,000+ QPS.
    *   **Efficiency:** Reduce memory footprint to < 500 bytes per node.
*   **Cons:** 
    *   Extreme implementation complexity (Manual memory management).
    *   Harder to debug (Internal IDs vs User IDs).
    *   Requires strict schema definitions (Loss of JSON flexibility).

## 4. Decision
We will transition to **Path B: High-Density Sharded Engine** in Phase 9. 

## 5. Strategic Trade-off Matrix

| Metric | Path A (Prototype) | Path B (Competitor) | Gain/Loss |
|---|---|---|---|
| **Vector Speed** | 3,698 vec/s | 50,000+ vec/s | **+13.5x** |
| **Max Scale** | ~2M Nodes | 320M+ Nodes | **+160x** |
| **Memory/Node** | ~2.3 KB | < 0.5 KB | **-78% Cost** |
| **Dev Velocity** | High | Low (Complex) | **-40% Speed** |

## 6. Implementation Strategy
1.  **Phase 9:** Implement ID Interning (u32 mapping) and Lock Sharding.
2.  **Phase 10:** Integrate SIMD (AVX-512) for distance calculations.
3.  **Phase 11:** Performance Audit against Qdrant/Neo4j.

## 7. Consequences
This shift marks the transition of GenesisDB from a "GKS Internal Tool" to a "Production-Grade Database Engine". Development will be slower and more rigorous, requiring deep systems programming skills.
