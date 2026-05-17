---
id: AUDIT--TRACE-INVARIANTS-ATOM-GRAPH-RULES
type: audit
status: stable
phase: 6
tier: process
title: AUDIT — Trace Invariants Atom-Graph Rules implementation
created_at: 2026-05-14T11:00:00+07:00
tags: [validator, proto, trace-invariants]
crosslinks:
  references: [PROTO--SYMBOLS-TRACE-INVARIANTS, BLUEPRINT--PROTO-LOADER]
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# AUDIT — Trace Invariants Atom-Graph Rules

## Summary
Mechanised Rule 2 (Acyclic Constraint) and Rule 4b (Atom Referential Integrity) within the `trace-invariants.ts` predicate.

## Implementation Details
- **Rule 2**: Implemented via DFS with White/Gray/Black coloring for cycle detection in a directed multigraph. Edges tracked: `supersedes`, `implements`, `parent_blueprint`.
- **Rule 4b**: Implemented via comprehensive scan of all `crosslinks` keys. Targets are checked against the `atomicIndex`.
- **Throttling**: Integrity check halts after 50 violations to prevent log flooding during initial vault migration.

## Verification
- [x] Unit tests created in `trace-invariants.test.ts` covering cycles, self-loops, and missing targets.
- [ ] Integration test with real vault (deferred due to environment constraints).

## Decision Log
- **External Markers**: Since the current indexer strips `external: true` objects, any missing reference is flagged. This ensures strictness until the indexer is upgraded.
- **Cycle Granularity**: DFS returns the first cycle found in each component. This satisfies the "emit ONE violation listing the cycle nodes" requirement.

## Connections
- [[PROTO--SYMBOLS-TRACE-INVARIANTS]]
- [[BLUEPRINT--PROTO-LOADER]]

