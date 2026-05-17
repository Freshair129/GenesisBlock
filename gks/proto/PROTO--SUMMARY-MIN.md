---
id: PROTO--SUMMARY-MIN
phase: 2
type: proto
status: stable
severity: error
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--SUMMARY-MIN — summary length bounds + placeholder ban
tags: &a1
  - msp
  - proto
  - summary
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
  - file: packages/msp/src/validator/proto/summary-min.ts
created_at: 2026-05-05T20:00:00.000+07:00
aliases: &a4
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  id: PROTO--SUMMARY-MIN
  phase: 2
  type: proto
  status: stable
  severity: error
  vault_id: default
  tier: safety
  source_type: axiomatic
  title: PROTO--SUMMARY-MIN — summary length bounds + placeholder ban
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-05T20:00:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Machine-enforced invariant
  attributes:
    id: PROTO--SUMMARY-MIN
    phase: 2
    type: proto
    status: stable
    severity: error
    vault_id: default
    tier: safety
    source_type: axiomatic
    title: PROTO--SUMMARY-MIN — summary length bounds + placeholder ban
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

# PROTO — SUMMARY-MIN

## Rule

If `summary:` is present in atom frontmatter:
- Must be a string
- Must be 10–300 characters (after trim)
- Must NOT contain placeholder strings: `TBD`, `TODO`, `FIXME`, `lorem ipsum`

## Schema

Per-atom frontmatter only.

## Predicate

Wraps existing core rule `summaryMin` from `src/validator/rules/summary-min.ts` via `ruleAdapter`. M8f promotion — original rule still runs in core until PROTO is promoted to stable.

## Trigger

`msp:validate --all`.

## Severity

`error` — short or placeholder summaries are clear bugs.

## Status

`draft` — original rule already enforces this; PROTO version is the eventual replacement, kept off the fail-exit path during overlap. M8f-2 will cut over once observation confirms parity.

## Source

`[[CONCEPT--PROTO-AUDIT-EXISTING-RULES]]` (M8f), `[[ADR--ANTI-HALLUCINATION-RULES]]`.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--PROTO-PATTERN]]
- [[FEAT--PROTO-LOADER]]

