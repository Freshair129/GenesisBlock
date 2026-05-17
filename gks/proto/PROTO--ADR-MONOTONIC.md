---
id: PROTO--ADR-MONOTONIC
phase: 2
type: proto
status: stable
severity: error
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--ADR-MONOTONIC — new ADR-NNN must equal max(existing) + 1
tags: &a1
  - msp
  - proto
  - adr
  - validator
  - m8f
crosslinks: &a2
  enforces:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
  references:
    - CONCEPT--PROTO-PATTERN
    - CONCEPT--PROTO-AUDIT-EXISTING-RULES
    - FEAT--PROTO-LOADER
    - ADR--ANTI-HALLUCINATION-RULES
linked_symbols: &a3
  - file: packages/msp/src/validator/proto/adr-monotonic.ts
created_at: 2026-05-05T20:00:00.000+07:00
aliases: &a4
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  id: PROTO--ADR-MONOTONIC
  phase: 2
  type: proto
  status: stable
  severity: error
  vault_id: default
  tier: safety
  source_type: axiomatic
  title: PROTO--ADR-MONOTONIC — new ADR-NNN must equal max(existing) + 1
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-05T20:00:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Machine-enforced invariant
  attributes:
    id: PROTO--ADR-MONOTONIC
    phase: 2
    type: proto
    status: stable
    severity: error
    vault_id: default
    tier: safety
    source_type: axiomatic
    title: PROTO--ADR-MONOTONIC — new ADR-NNN must equal max(existing) + 1
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-05T20:00:00.000+07:00
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

# PROTO — ADR-MONOTONIC

## Rule

For atoms with id matching `ADR-NNN` (numeric form): the next-allocated number must equal `max(existing-numbers) + 1` (or 1 if none).

Slug-form `ADR--<NAME>` IDs are skipped — they don't carry an integer to monotonically check.

## Schema

Per-atom frontmatter `id`; cross-referenced against the atomic index for the running max.

## Predicate

Wraps existing core rule `adrMonotonic` from `src/validator/rules/adr-monotonic.ts` via `ruleAdapter`. Filter: only invokes on atoms with `type: adr`.

## Trigger

`msp:validate --all`.

## Severity

`error` — out-of-sequence numbers create review confusion + version-history breaks.

## Status

`draft` — overlap with core rule until cutover (M8f-2).

## Source

`[[CONCEPT--PROTO-AUDIT-EXISTING-RULES]]`, `[[ADR--ANTI-HALLUCINATION-RULES]]`.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--PROTO-PATTERN]]
- [[FEAT--PROTO-LOADER]]

