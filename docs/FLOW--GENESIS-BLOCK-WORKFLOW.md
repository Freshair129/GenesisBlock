# FLOW--GENESIS-BLOCK-WORKFLOW

## 1. Engine Initialization
The boot sequence of a GenesisDB instance.

1. **Path Resolution:** Locate \genesis-graph.lock\ and acquire OS-level file lock.
2. **Snapshot Load:** If \.bin\ exists, deserialize the full state using Bincode.
3. **Index Rehydration:** Iterate through \metadata_arena\ and rebuild the HNSW graph in-memory.
4. **WAL Replay:** Scan the trailing JSONL log for events after the last snapshot.

## 2. Node/Edge Ingestion
The synchronous path for adding knowledge.

1. **Write Guard:** Acquire \parking_lot::RwLock\ (Write mode).
2. **Alignment Calculation:** Determine the next 64-byte boundary in \ector_arena\.
3. **Physical Write:** \extend_from_slice\ to Arenas + Update HNSW graph.
4. **Persistence:** Append \Event\ to the WAL before releasing the lock.

## 3. Hybrid Query Lifecycle
The multi-stage discovery process.

1. **Search Phase:** Concurrent HNSW semantic lookup.
2. **Expansion Phase:** BFS structural traversal from semantic seeds.
3. **Ranking Phase:** Calculate K-Impact for all candidates using the blended formula.
4. **Delivery:** Slicing and zero-copy return via FFI or JSON.

## 4. Maintenance (Compaction)
Optimization of on-disk footprint.

1. **Tombstone Removal:** Filter out retracted edges (\alid_to\ is set).
2. **De-fragmentation:** Pack arenas and rewrite the JSONL log.
3. **Atomic Commit:** Write new binary snapshot and prune the old log.
