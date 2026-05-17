---
id: CONCEPT--MEMORY-STORE
phase: 1
type: concept
status: stable
created_at: 2026-05-13T12:00:00+07:00
vault_id: GKS-CORE
tier: genesis
title: MemoryStore — the unified four-layer façade
tags: &a1
  - architecture
  - core
  - api
crosslinks: &a2
  references:
    - FRAMEWORK--FOUR-LAYERS
    - FEAT--LOOKUP-BY-SYMBOL
linked_symbols: &a3
  - file: packages/gks/src/memory/index.ts
  - file: packages/gks/src/memory/api.ts
    fn: retain
  - file: packages/gks/src/memory/api.ts
    fn: recall
  - file: packages/gks/src/memory/api.ts
    fn: reflect
aliases: &a4
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--MEMORY-STORE
  phase: 1
  type: concept
  status: stable
  created_at: 2026-05-13T12:00:00+07:00
  vault_id: GKS-CORE
  tier: genesis
  title: MemoryStore — the unified four-layer façade
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  aliases: *a4
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--MEMORY-STORE
    phase: 1
    type: concept
    status: stable
    created_at: 2026-05-13T12:00:00+07:00
    vault_id: GKS-CORE
    tier: genesis
    title: MemoryStore — the unified four-layer façade
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    aliases: *a4
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      domain: concept
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — MemoryStore

`MemoryStore` is GKS's single TS façade. Behind it: four cooperating
storage layers (Atomic, Vector, Episodic, Obsidian) plus cross-cutting
infrastructure (audit, cost, telemetry, retry, circuit-breaker, schema
versioning).

## Three verbs

`retain(content, metadata)` — write a fact with bi-temporal versioning
+ namespace-scoped conflict detection.

`recall(query, opts)` — multi-source parallel retrieval with dedup +
optional rerank + max-total cap.

`reflect(input)` — run the deterministic Three-Gate Consolidator
(LLM-pluggable) over a session trace; produce episodic summary +
candidate atoms for the inbound queue.

## Helpers

- `lookup(id)` — exact-id atomic lookup, never approximates
- `lookupBySymbol(path)` — reverse citation (ADR-010)
- `proposeInbound(artifact)` — only authorised path to candidate atoms
- `appendTrace(sessionId, step)` — session lifecycle

## What it doesn't do

- No Memory OS / consolidation timing (orchestrator's job per ADR-008)
- No code AST / call graph (peer subsystem's job per ADR-009)
- No workflow gates (CI / process layer)

`MemoryStore` is **paradigm-agnostic** — works equally for single-tenant
CLI agents, multi-tenant SaaS, MSP-shaped Memory OS layered above, or
research projects with custom consolidation cascades.

## Connections
- [[FRAMEWORK--FOUR-LAYERS]]
- [[FEAT--LOOKUP-BY-SYMBOL]]

