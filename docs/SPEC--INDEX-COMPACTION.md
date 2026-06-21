# Software Requirements Document (SRD): Index Compaction & Cleanup (Mark IX, Step 3)

## 1. Introduction
As GenesisBlockDB scales, frequent retractions and TTL expirations leave "holes" in the memory arenas and stale entries in the HNSW graph. **Mark IX Step 3** introduces **Autonomic Compaction**, a background process that reclaims physical memory and optimizes search structures by pruning dead data.

## 2. Functional Requirements

### FR1: Arena Compaction
- The system must periodically shrink **each vector collection's** arena and metadata by removing data belonging to retracted or expired nodes. Since the multi-collection migration (ADR--GENESISDB-MULTI-COLLECTION) there is no single global arena — compaction iterates every entry in `collections` and compacts `VectorCollection.arena` / `.metadata` independently.

### FR2: HNSW Optimization
- The HNSW index must be fully rebuilt during compaction to ensure that the internal graph structure is optimized for currently live nodes only, improving query precision and speed.

### FR3: Adjacency Pruning
- Empty entries in `in_idx` and `out_idx` DashMaps must be removed to prevent map bloat.

---

# Technical Design Document (TDD): Compaction Engine

## 1. Implementation Logic

### 1.1 `Storage::perform_index_compaction()`
0.  **Flush the async index queue** (`flush_index()`) first — compaction reassigns arena ids, so a pending HNSW insert must not target a stale id (ADR--GENESISDB-ASYNC-INDEXING).
1.  **Identify Live Set:** Collect all node IDs currently present in the `nodes` DashMap.
2.  **Compact each collection's Arena** (loop over `collections`):
    - Create a new temporary arena + metadata for the collection.
    - Iterate the collection's old `metadata`; if a node is in the **Live Set**, copy its vector and metadata to the new arena.
    - Rebuild the collection's `node_to_arena` mapping (replaces the former global `u32_to_arena_id`) and reset its `count`.
3.  **Rebuild HNSW:**
    - Trigger `rehydrate_hnsw_index()` — rebuilds **every** collection's HNSW from its compacted arena.
4.  **DashMap Cleanup:**
    - Remove keys from `in_idx` and `out_idx` that no longer exist in the `nodes` map.

## 2. Integration
Compaction will be triggered:
- Manually via `Storage::compact()` API.
- Automatically by the `Autonomic Substrate` when the "Entropy Ratio" (Deleted Nodes / Total Nodes) exceeds a threshold (e.g., 20%).

---

## 3. Definition of Done (DoD)
1.  [ ] `perform_compaction()` logic implemented.
2.  [ ] **Memory Reclamation Test:** Add 1000 nodes -> Retract 900 -> Run Compaction -> Verify the default collection's `arena.len()` reflects only 100 nodes (`compaction_tests`).
3.  [ ] **Integrity Test:** Verify that search results are still accurate after a full compaction.
4.  [ ] Documentation updated in `MASTER-SPEC--GENESIS-DB.md`.

---
**Please review and approve this Specification. I will begin the implementation once approved.**
