---
id: PROTO--SYMBOLS-TRACE-INVARIANTS
phase: 2
type: proto
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Structural invariants for execution traces
tags:
  - msp
  - symbol-graph
  - proto
  - invariants
crosslinks: {"implements":["ADR--SYMBOLS-PROCESS-TRACING"]}
created_at: 2026-05-12T15:30:00.000+07:00
---

# PROTO — Trace Invariants

## Rule 1: Termination Guard
Every trace result MUST have a finite length and terminate at either a leaf node, a cycle, or the maximum depth limit.
- **Severity**: Error
- **Enforcement**: Validated by the `trace-invariants` predicate.

## Rule 2: Entry Point Origin
Traces initiated via architectural tools (e.g. `symbol_trace` for a route) MUST originate from a node tagged with a framework `attrs` or a recognized framework `kind` (page, route, tool).
- **Severity**: Warning
- **Enforcement**: Check `src_id` of the first edge in the trace.

## Rule 3: No Dangling Resolved Edges
A trace path MUST NOT contain a `resolved: true` edge whose `dst_id` does not exist in the `symbols` table.
- **Severity**: Error
- **Enforcement**: Referrential integrity check.

## Source
- `ADR--SYMBOLS-PROCESS-TRACING`
- `ALGO--SYMBOLS-CALL-GRAPH-TRAVERSAL`
