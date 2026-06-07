# Software Requirements Document (SRD): Index Compaction & Cleanup (Mark IX, Step 3)

## 1. Introduction
As GenesisDB scales, frequent retractions and TTL expirations leave "holes" in the memory arenas and stale entries in the HNSW graph. **Mark IX Step 3** introduces **Autonomic Compaction**, a background process that reclaims physical memory and optimizes search structures by pruning dead data.

## 2. Functional Requirements

### FR1: Arena Compaction
- The system must periodically shrink the `vector_arena` and `metadata_arena` by removing data belonging to retracted or expired nodes.

### FR2: HNSW Optimization
- The HNSW index must be fully rebuilt during compaction to ensure that the internal graph structure is optimized for currently live nodes only, improving query precision and speed.

### FR3: Adjacency Pruning
- Empty entries in `in_idx` and `out_idx` DashMaps must be removed to prevent map bloat.

---

# Technical Design Document (TDD): Compaction Engine

## 1. Implementation Logic

### 1.1 `Storage::perform_compaction()`
1.  **Identify Live Set:** Collect all node IDs currently present in the `nodes` DashMap.
2.  **Compact Arenas:**
    - Create new temporary `vector_arena` and `metadata_arena`.
    - Iterate through the old `metadata_arena`; if a node is in the **Live Set**, copy its vector and metadata to the new arenas.
    - Update `u32_to_arena_id` mapping.
3.  **Rebuild HNSW:**
    - Trigger `rehydrate_hnsw_index()` using the newly compacted arenas.
4.  **DashMap Cleanup:**
    - Remove keys from `in_idx` and `out_idx` that no longer exist in the `nodes` map.

## 2. Integration
Compaction will be triggered:
- Manually via `Storage::compact()` API.
- Automatically by the `Autonomic Substrate` when the "Entropy Ratio" (Deleted Nodes / Total Nodes) exceeds a threshold (e.g., 20%).

---

## 3. Definition of Done (DoD)
1.  [ ] `perform_compaction()` logic implemented.
2.  [ ] **Memory Reclamation Test:** Add 1000 nodes -> Retract 900 -> Run Compaction -> Verify `vector_arena.len()` reflects only 100 nodes.
3.  [ ] **Integrity Test:** Verify that search results are still accurate after a full compaction.
4.  [ ] Documentation updated in `MASTER-SPEC--GENESIS-DB.md`.

---
**Please review and approve this Specification. I will begin the implementation once approved.**
