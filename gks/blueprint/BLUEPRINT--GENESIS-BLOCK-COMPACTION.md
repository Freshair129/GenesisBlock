---
id: BLUEPRINT--GENESIS-BLOCK-COMPACTION
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Genesis Block Log Compaction
tags: [msp, gks, genesis-block, storage, rust, plan]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - SPEC--GENESIS-GRAPH-BACKEND
    - ADR--GENESIS-BLOCK-STORAGE-LAYOUT
linked_symbols:
  - file: packages/gks/native/genesis-block/src/lib.rs
created_at: 2026-05-21T12:00:00+07:00
---

# BLUEPRINT — Genesis Block Log Compaction

## 1. Problem Statement

The current Genesis Block storage (Phase 3.2) uses an append-only JSONL log. Every node/edge update or retraction adds a new line. Over time, the log accumulates "stale" data (superseded properties or retracted edges), leading to:

1. Increased disk usage.
2. Slower cold-start times (more lines to replay into memory).
3. Higher memory pressure during the replay phase.

## 2. Proposed Solution: Log Compaction

Implement a "Compaction" routine in the native Rust engine that rewrites the JSONL log to contain only the minimal set of events required to reconstruct the current state of the graph.

### 2.1 Technical Requirements

- **Atomic Swap:** Compaction must not corrupt the database if interrupted.
- **Lock Integrity:** Must respect the `genesis-graph.lock` established in P3.3.
- **Memory-to-Disk:** Leverage the fact that the `Storage` struct in memory already represents the "Current Truth".

### 2.2 Compaction Logic (Rust)

1. **Prepare:** Open a temporary file `genesis-graph.jsonl.tmp` in the same directory as the database.
2. **Serialize Nodes:** Iterate through `storage.nodes` (HashMap) and write each as an `Event::Node` line.
3. **Serialize Edges:** Iterate through `storage.edges` (HashMap).
   - **Filter:** Skip edges that are fully retracted (where `valid_to` is NOT null) UNLESS they are needed for historical auditing (Phase 4.1 default: remove retracted edges).
   - Write remaining edges as `Event::Edge` lines.
4. **Finalize:**
   - Flush and close the `.tmp` file.
   - Use `std::fs::rename` to replace the original `genesis-graph.jsonl` with the `.tmp` file.
   - Verify the original file is replaced atomically.

## 3. Integration & API

Add a new method to the `GenesisDatabase` class:

```typescript
// index.d.ts
compact(): Promise<void>
```

Add a synchronous variant for internal use:

```rust
// lib.rs
pub fn compact_sync(&mut self) -> Result<()>
```

## 4. Triggers

- **Manual:** Exposed via `msp-graph compact` or the `compact()` method.
- **Auto (Future):** Threshold based on `(log_line_count / current_element_count) > threshold`.

## 5. Verification Plan

- **Unit Test:** Add 100 edges, retract 50, run `compact()`, verify file size decreases and data integrity remains.
- **Benchmark:** Compare replay time of a dirty log vs a compacted log.
