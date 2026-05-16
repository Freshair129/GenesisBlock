---
id: ADR--SYMBOLS-PROCESS-TRACING
phase: 2
type: adr
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Execution flow traversal strategy and constraints
tags:
  - msp
  - symbol-graph
  - adr
  - decision
crosslinks: {"references":["CONCEPT--SYMBOLS-PROCESS-TRACING"]}
created_at: 2026-05-12T15:51:00.000+07:00
---

# ADR — Process tracing traversal strategy

## Context
We need to implement a tracer that follows `CALLS` edges in the Symbol Graph. The graph may contain cycles (recursion) and very deep paths.

## Decision
1. **Algorithm**: Use a depth-limited search (DLS) or BFS with a visited set to detect cycles.
2. **Depth Limit**: Set a default limit of **8 hops**. This is sufficient for most application-level flows while preventing performance degradation on extremely complex graphs.
3. **Entry Points**: Explicitly use the nodes emitted by `FrameworkRecognizer` (kinds: `page`, `route`, `tool`) as the seeds for tracing.
4. **Resolution**: Only follow `resolved: true` edges to maintain high precision in the resulting paths.

## Consequences
- Recursive flows will be truncated at the first cycle.
- Deep utility chains beyond 8 hops will be truncated (user can override depth via CLI/MCP).
- Unresolved dynamic calls will act as "leafs" in the trace.

## Alternatives
- Exhaustive search (too slow on large graphs).
- Abstract interpretation (too complex for v1).

## Source
- `[[FEAT--SYMBOLS-PROCESS-TRACING]]`
- `[[ALGO--SYMBOLS-CALL-GRAPH-TRAVERSAL]]`

## Connections
- [[CONCEPT--SYMBOLS-PROCESS-TRACING]]

