---
id: ALGO--SYMBOLS-CALL-GRAPH-TRAVERSAL
phase: 2
type: algo
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Call graph traversal algorithm with cycle and depth guards
tags:
  - msp
  - symbol-graph
  - algo
  - algorithm
crosslinks: {"implements":["ADR--SYMBOLS-PROCESS-TRACING"]}
created_at: 2026-05-12T15:52:00.000+07:00
---

# ALGO — Call graph traversal

## Inputs
- `seedSymbolId`: The starting symbol ID.
- `maxDepth`: Maximum number of hops (default 8).
- `direction`: `down` (callees) or `up` (callers).

## Algorithm
1. Initialize `visited` set (symbol IDs).
2. Initialize `results` list (TracePaths).
3. **Recursive Step** (`currentId`, `currentDepth`, `path`):
    - If `currentDepth > maxDepth` → return.
    - If `currentId` is in `path` → mark as `cycle` and return.
    - Fetch all outgoing `CALLS` edges from `currentId` (if direction is `down`) or incoming (if `up`).
    - For each edge:
        - Add `edge.dst_id` to `path`.
        - Recurse with `edge.dst_id`, `currentDepth + 1`.
4. Return all unique paths discovered.

## Complexity
- Worst case: O(V + E) where V is vertices and E is edges within the depth-limited subgraph.

## Edge cases
- **Empty Graph**: Return empty result.
- **Self-Recursion**: Truncate after first hop, mark as cycle.
- **Unresolved Edges**: Stop traversal at that branch, mark target as "unresolved symbol".

## Source
- `[[ADR--SYMBOLS-PROCESS-TRACING]]`
