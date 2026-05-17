---
id: BLUEPRINT--TRACE-INVARIANTS-PREDICATE-CONTEXT
phase: 3
type: blueprint
status: draft
tier: process
title: BLUEPRINT — Extend PredicateContext with Symbol Graph Reader
crosslinks:
  implements:
    - PROTO--SYMBOLS-TRACE-INVARIANTS
    - FRAMEWORK--MSP-ARCHITECTURE-V2
  parent_blueprint:
    - BLUEPRINT--PROTO-LOADER
linked_symbols:
  - file: packages/msp/src/validator/proto/types.ts
  - file: packages/msp/src/validator/proto/loader.ts
created_at: 2026-05-16T01:00:00.000+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  domain: blueprint
---

# BLUEPRINT — Predicate Context Extension for Symbol Graph

## Geography
Files modified/created:
- `packages/msp/src/validator/proto/types.ts`: Extend `PredicateContext`.
- `packages/msp/src/validator/proto/loader.ts`: Open/close DB, inject reader.
- `packages/msp/src/validator/proto/symbol-graph-reader.ts` (NEW): Typed reader interface + implementation.
- `packages/msp/test/validator/proto/loader.test.ts`: Update tests.

## Acceptance
- `PredicateContext` has a `symbolGraph: SymbolGraphReader | null` field.
- The `SymbolGraphReader` provides a strict read-only API over the underlying `SymbolStore`.
- `loader.ts` initializes the symbol graph at the start of a validation run, warns once if absent, and closes it securely at the end.
- Existing predicates that do not use `symbolGraph` continue functioning correctly without modification.

## Dependencies
- PR #1 (antigravity/proto-trace-invariants-atom-graph) merged.
- The symbol-graph DB exists in the repository (`.brain/msp/projects/evaAI/symbols/graph.db`).

## Tasks
1. **Define Interface**: In `packages/msp/src/validator/proto/symbol-graph-reader.ts`, define the `SymbolGraphReader` interface and its implementation wrapping `SymbolStore`.
2. **Extend Types**: In `packages/msp/src/validator/proto/types.ts`, import the interface and add `symbolGraph` to `PredicateContext`.
3. **Update Loader**: In `packages/msp/src/validator/proto/loader.ts`, add the logic to instantiate the reader, handle potential missing DB, and inject it into the context before calling `runProtos`. Make sure to close the DB after execution.
4. **Update Tests**: Adjust loader test cases to mock or gracefully ignore the missing `symbolGraph`.

## Connections
- [[PROTO--SYMBOLS-TRACE-INVARIANTS]]
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[BLUEPRINT--PROTO-LOADER]]

