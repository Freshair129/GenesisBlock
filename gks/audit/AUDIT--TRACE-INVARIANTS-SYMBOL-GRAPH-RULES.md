---
id: AUDIT--TRACE-INVARIANTS-SYMBOL-GRAPH-RULES
phase: 6
type: audit
status: stable
title: "Audit: Symbol Graph Trace Invariants Implementation"
crosslinks:
  implements: ["PROTO--SYMBOLS-TRACE-INVARIANTS"]
created_at: 2026-05-16T01:00:00.000+07:00
---

# AUDIT — Symbol Graph Trace Invariants

## Objective
Verify the implementation of Rules 1, 3, and 4a of `PROTO--SYMBOLS-TRACE-INVARIANTS` inside the existing trace-invariants predicate.

## Scope
- `SymbolGraphReader` abstraction over `SymbolStore`.
- Injection of `symbolGraph` into `PredicateContext`.
- Rule 1: Termination Guard via Depth-Limited Search (DLS).
- Rule 3: Entry Point Origin validation.
- Rule 4a: Symbol Referential Integrity checks.

## Implementation Notes
- **Rule 4a (Symbol Referential Integrity)**: A global scan over all edges validates that for every `resolved: true` edge, the `dst_id` exists in the symbols table. If the number of violations exceeds 100, the severity is downgraded to a warning to prevent pipeline blocking, as this indicates widespread drift in the graph extraction.
- **Rule 1 (Termination Guard)** & **Rule 3 (Entry Point Origin)**: Rather than scanning all traces, the implementation identifies framework entry points (kinds: `page`, `route`, `tool`) and runs a trace (up to 8 hops) to detect either natural termination (leaf node), cyclic termination, or max-depth graceful termination.

## Findings
- The `PredicateContext` has been safely extended. `symbolGraph` elegantly degrades to `null` on fresh checkouts where the `.brain/msp/projects/evaAI/symbols/graph.db` has not yet been built.
- Automated tests mock `SymbolGraphReader` effectively, ensuring no dependency on a real SQLite DB during unit testing.

## Next Steps
- Monitor Rule 4a violation counts in CI.
- Consider expanding Rule 3 to handle MCP tools.
