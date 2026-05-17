---
id: PROTO--SYMBOLS-FRAMEWORK-INVARIANTS
phase: 2
type: proto
status: active
tier: safety
source_type: axiomatic
vault_id: default
title: Structural invariants for framework-aware symbols
tags: &a1
  - msp
  - symbol-graph
  - protocol
  - invariants
  - nextjs
  - prisma
crosslinks: &a2
  implements:
    - ADR--SYMBOLS-FRAMEWORK-AWARENESS
  references:
    - CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS
    - ALGO--SYMBOLS-FRAMEWORK-RECOGNITION
linked_symbols: &a3
  - file: packages/msp/src/validator/proto/framework-invariants.ts
created_at: 2026-05-12T12:38:00.000+07:00
aliases: &a4
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  id: PROTO--SYMBOLS-FRAMEWORK-INVARIANTS
  phase: 2
  type: proto
  status: active
  tier: safety
  source_type: axiomatic
  vault_id: default
  title: Structural invariants for framework-aware symbols
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-12T12:38:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Machine-enforced invariant
  attributes:
    id: PROTO--SYMBOLS-FRAMEWORK-INVARIANTS
    phase: 2
    type: proto
    status: active
    tier: safety
    source_type: axiomatic
    vault_id: default
    title: Structural invariants for framework-aware symbols
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-12T12:38:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Machine-enforced invariant
    attributes:
      domain: proto
    domain: proto
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: proto
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# PROTO — Framework-aware symbol invariants

Structural rules that must hold true for symbols and edges emitted by framework recognizers.

## Rule 1: Next.js Page URL Requirement
Every symbol of type `Page` MUST have at least one outgoing `RENDERS_AT` edge pointing to a `URL` attribute or node.
- **Severity**: Error
- **Enforcement**: Static analysis of the symbol graph.

## Rule 2: Next.js Route Handler Linkage
Every symbol of type `Route` (API) MUST have at least one `HANDLES` edge pointing to a function symbol.
- **Severity**: Warning (some routes might be misconfigured or empty)
- **Enforcement**: Static analysis.

## Rule 3: Server/Client Runtime Exclusivity
A symbol cannot be tagged as both `runtime: 'client'` and `runtime: 'server'`.
- **Severity**: Error
- **Enforcement**: Check `attrs.runtime` on symbols.

## Rule 4: ORM Entity Mapping
Every `Entity` node (Prisma/Drizzle) MUST have an `orm` attribute identifying the source framework.
- **Severity**: Error
- **Enforcement**: Check `attrs.orm`.

## Rule 5: MCP Tool Identity
Every `Tool` node MUST have a `name` attribute matching the string identifier used in `registerTool`.
- **Severity**: Error
- **Enforcement**: Check `attrs.name`.

## Counter-example
A `Page` node in `app/dashboard/page.tsx` that lacks a `RENDERS_AT` edge to `/dashboard` violates Rule 1.

## Source
- `[[FEAT--SYMBOLS-FRAMEWORK-AWARENESS]]`
- `[[ALGO--SYMBOLS-FRAMEWORK-RECOGNITION]]`
- Next.js routing documentation

## Connections
- [[ADR--SYMBOLS-FRAMEWORK-AWARENESS]]
- [[CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS]]

