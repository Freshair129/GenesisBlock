---
status: current
---

# FLOW--GENESIS-BLOCK-WORKFLOW

## 1. Engine Initialization
The boot sequence of a GenesisBlockDB instance.

1. **Path Resolution:** Locate `genesis-graph.wal` and acquire the OS-level file lock / identity.
2. **Snapshot Load:** If a snapshot exists, load per-collection `vec_<name>.bin` / `meta_<name>.bin` (driven by the `collections` manifest in `state.json`), plus `nodes.bin` / `edges.bin`. Legacy single-space `vector.bin` / `meta.bin` migrates into the `default` collection.
3. **WAL Replay:** If no snapshot, replay the trailing JSONL log; node embeddings are staged into their `collection` (auto-provisioned from the embedding dim if missing).
4. **Index Rehydration:** For **each collection**, iterate its `metadata` and rebuild that collection's HNSW graph in-memory (synchronous, once).

## 2. Node/Edge Ingestion
The write path for adding knowledge.

1. **Resolve collection:** Pick the target `VectorCollection` (the node's `collection`, or `default`) and validate the embedding length against its dim — a mismatch is a typed error.
2. **Stage vector:** `extend_from_slice` the (Cosine-normalized if applicable) f32 vector into the collection arena and push its `NodeMetadata`. Synchronous and durable once the WAL acks; this is the source of truth for search.
3. **Enqueue index job:** Hand the HNSW insert to the async indexing thread (`enqueue_one` / `enqueue_batch`). The writer does **not** build the graph (ADR--GENESISDB-ASYNC-INDEXING) — vectors are eventually searchable.
4. **Edges:** Key the edge by `Storage::edge_key(id)` (u64 hash), insert into `edges` + `out_idx` / `in_idx`; no string is interned for edges.
5. **Persistence:** Append the `Event` to the WAL (group-committed) before the call returns; embedding + `collection` are persisted for replay.

## 3. Hybrid Query Lifecycle
The multi-stage discovery process.

1. **Resolve + validate:** Pick the query's `collection` (default if unset); validate the query length against the collection dim.
2. **Search Phase:** HNSW semantic lookup within that collection's index.
3. **Expansion Phase:** BFS structural traversal from semantic seeds via `out_idx` / `in_idx`.
4. **Ranking Phase:** Blend vector similarity with K-Impact.
5. **Delivery:** Slice to `k` and return via FFI or JSON.

## 4. Maintenance (Compaction)
Optimization of memory + on-disk footprint.

1. **Flush index:** Drain the async indexing queue so arena-id reassignment cannot race a pending insert.
2. **Tombstone Removal:** Filter out retracted/expired nodes and edges.
3. **De-fragmentation:** Per collection, pack the arena + metadata and rebuild `node_to_arena`; rewrite the JSONL log.
4. **Atomic Commit:** Write the new snapshot (per-collection files + manifest) and prune the old log.
