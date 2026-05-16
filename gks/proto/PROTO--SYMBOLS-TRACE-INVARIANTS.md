---
id: PROTO--SYMBOLS-TRACE-INVARIANTS
phase: 2
type: proto
status: stable
severity: error
tier: process
source_type: axiomatic
vault_id: default
title: Structural invariants for execution traces and atom graphs
tags:
  - msp
  - symbol-graph
  - proto
  - invariants
  - atom-graph
crosslinks:
  enforces: ["ADR--SYMBOLS-PROCESS-TRACING"]
  implements: ["ADR--SYMBOLS-PROCESS-TRACING"]
  supersedes: ["PROTO--TRACE-INVARIANTS"]
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/trace-invariants.ts"}
created_at: 2026-05-14T20:00:00.000+07:00
---

# PROTO — Trace Invariants

This protocol defines structural invariants for both code execution traces (Symbol Graph) and atom relationship graphs to ensure finite analysis and referential integrity.

Three rules are **statically enforced** at validation time (Rule 2, 4a, 4b). Two rules are **runtime invariants** that the symbol-trace tooling must uphold; they cannot be checked against a static snapshot of the graph (Rule 1, 3).

## Rule 1: Termination Guard (Execution Traces) — runtime invariant
Every symbol trace result MUST have a finite length and terminate at either a leaf node, a cycle, or the maximum depth limit (default: 8).
- **Severity**: Error
- **Enforcement**: The `symbol_trace` MCP tool maintains a visited-set plus depth cap so traces cannot diverge. There is no static violation case to detect; the validator predicate documents this rule as deferred to the tool's implementation.

## Rule 2: Acyclic Constraint (Atom Graph)
Relationship chains for directed edges in the atom graph MUST NOT form cycles.
- **Edges**: `supersedes`, `implements`, `parent_blueprint`.
- **Severity**: Error
- **Enforcement**: `trace-invariants` predicate (rule tag `acyclic-constraint`). Iterative DFS with three-colour marking; reports the first cycle found per connected component.

## Rule 3: Entry Point Origin — runtime invariant
Traces initiated via architectural tools (e.g. `symbol_trace` for a route) MUST originate from a node tagged with a framework `attrs` or a recognised framework `kind` (page, route, tool).
- **Severity**: Warning
- **Enforcement**: Checked at the call site of the trace tool. No static violation case exists in the stored graph (which has no `trace` entity), so the validator predicate does not enforce this rule.

## Rule 4a: Symbol Referential Integrity (Symbol Graph)
A `resolved: true` edge MUST NOT have a `dst_id` that is absent from the `symbols` table.
- **Severity**: Error
- **Enforcement**: `trace-invariants` predicate (rule tag `symbol-ref-integrity`). Skipped gracefully when the symbol-graph DB is not present in the working tree. Severities are downgraded to warning if the violation count exceeds 100 (signals extractor lag rather than authoring drift).

## Rule 4b: Atom Referential Integrity (Atom Graph)
An atom MUST NOT reference a target atom ID via `crosslinks.*` that does not exist in the atomic index.
- **Severity**: Error
- **Enforcement**: `trace-invariants` predicate (rule tag `atom-ref-integrity`). Halts after 50 violations to avoid log floods; fix the first batch and re-run. The current indexer strips `external: true` markers, so all missing targets are flagged unconditionally — fix this by upgrading the indexer if external references become a real use case.

## Source
- `[[ADR--SYMBOLS-PROCESS-TRACING]]`
- `[[ALGO--SYMBOLS-CALL-GRAPH-TRAVERSAL]]`
- `[[PROTO--TRACE-INVARIANTS]]` (Merged)
