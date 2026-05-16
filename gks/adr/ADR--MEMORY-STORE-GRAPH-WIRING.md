---
id: ADR--MEMORY-STORE-GRAPH-WIRING
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: GraphBackend joins MemoryStoreOptions — MemoryStore exposes store.graph after init()
tags:
  - gks
  - memory
  - graph
  - decision
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--MEMORY-STORE"]}
created_at: 2026-05-12T22:48:00.000+07:00
---

# ADR — MemoryStore graphBackend wiring

## Context

`MemoryStoreOptions` lets callers swap the vector backend via `vectorBackend: VectorBackendFactory`. There is no equivalent knob for the graph layer, even though three `GraphBackend` implementations now ship (`GraphStore`, `PgGraphBackend`, `GenesisGraphBackend`) and the `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` plan calls for a one-line swap.

Today the graph layer is constructed ad-hoc by callers — MSP's `getStore()` doesn't expose a graph at all, and consumers wanting graph traversal must instantiate `GraphStore` independently. This contradicts `[[FRAMEWORK--FOUR-LAYERS]]` (the GKS frame atom) which lists graph alongside atomic / vector / episodic / obsidian as a first-class layer.

## Decision

Add `graphBackend?: GraphBackend | ((layout) => Promise<GraphBackend> | GraphBackend)` to `MemoryStoreOptions`. Add `store.graph: GraphBackend` field initialised in `init()`. Default backend = JSONL-backed `GraphStore` at `<brain>/graph/graph.jsonl`. The new layout dir is documented in `gksLayout()`.

The change is **additive** — existing callers that don't pass `graphBackend` get the default GraphStore. No public API breaks.

## Consequences

### Positive
- `PgGraphBackend` and `GenesisGraphBackend` are now wired into `MemoryStore` symmetrically with `pgvector` / `HnswBackend`.
- The cognitive facade (`createCognitiveLayer`) can expose `layer.graph` without a separate factory call.
- `[[FRAMEWORK--FOUR-LAYERS]]` is now reflected in the API surface.

### Negative
- One extra dir (`<brain>/graph/`) on disk by default. Gitignored.
- Callers that subclass MemoryStore (none in tree) must update if they override `init()`.

### Neutral
- The graph backend is initialised lazily — `init()` is the trigger, so unit tests that never call init won't pay the cost.

## Status

Draft. Promotion to `stable` requires green CI with the new `test/memory/memory-store.test.ts` graphBackend cases passing.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--MEMORY-STORE]]

