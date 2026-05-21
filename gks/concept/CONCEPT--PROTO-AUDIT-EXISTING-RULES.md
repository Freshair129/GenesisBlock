---
id: CONCEPT--PROTO-AUDIT-EXISTING-RULES
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: M8f — audit existing validator rules → promote to PROTO atoms where appropriate
tags: &a1
  - msp
  - proto
  - audit
  - refactor
  - governance
  - m8f
crosslinks: &a2
  references:
    - CONCEPT--PROTO-PATTERN
    - ADR--ANTI-HALLUCINATION-RULES
    - FEAT--MSP-VALIDATOR
created_at: 2026-05-05T16:28:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--PROTO-AUDIT-EXISTING-RULES
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: M8f — audit existing validator rules → promote to PROTO atoms where
    appropriate
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-05T16:28:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--PROTO-AUDIT-EXISTING-RULES
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: M8f — audit existing validator rules → promote to PROTO atoms where
      appropriate
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-05T16:28:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--PROTO-AUDIT-EXISTING-RULES
      phase: 1
      type: concept
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: M8f — audit existing validator rules → promote to PROTO atoms where
        appropriate
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-05T16:28:00.000+07:00
      aliases: *a3
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
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — audit existing validator rules → PROTO promotion

## Problem

`src/validator/rules/` contains ~12 hard-coded rules:

- `forbidden-fields.ts`
- `dangling-wikilinks.ts`
- `id-uniqueness.ts`
- `id-format.ts`
- `future-date.ts`
- `summary-min.ts`
- `adr-monotonic.ts`
- `evidence-for-decisions.ts`
- `id-filename-match.ts`
- … etc

Each is a TS function plus a passing test. None is documented as a PROTO atom — the rule lives in code and an ADR cross-link, but doesn't have its own atomic governance record.

M8f audits each rule and decides:

1. **Promote to PROTO** — if the rule has a clear FRAME backing + governance value
2. **Keep in core validator** — if it's a structural / parsing rule (e.g. id-format) that ANY consumer should see, not optional
3. **Deprecate** — if obsolete

## Audit table (proposed)

| Rule | Disposition | Why |
|---|---|---|
| `forbidden-fields` | core | structural; protects every atom |
| `dangling-wikilinks` | core | structural; cited in 2 ADRs |
| `id-uniqueness` | core | structural |
| `id-format` | core | structural |
| `id-filename-match` | core | structural |
| `future-date` | core | structural; trivially universal |
| `summary-min` | **PROTO** | governance per `[[ADR--FORBIDDEN-FIELDS-LIST]]` (length minimums; soft) |
| `adr-monotonic` | **PROTO** | governance per `[[ADR--ANTI-HALLUCINATION-RULES]]` (numbering rule) |
| `evidence-for-decisions` | **PROTO** | governance per `[[ADR--ANTI-HALLUCINATION-RULES]]` (ADR has Context/Decision/Consequences) |
| `cite-or-mark-inferred` | **PROTO** | governance, soft warning per ADR |

3 promotions. The promoted PROTOs link to their existing ADRs via `enforces:`.

## Trigger

This is **work** (refactor + atom write), not a runtime rule. M8f's deliverable:

- 3 new PROTO atoms (SUMMARY-MIN, ADR-MONOTONIC, EVIDENCE-FOR-DECISIONS)
- 3 wrappers in `src/validator/proto/` that delegate to the existing rule fns
- Original `src/validator/rules/<name>.ts` files become aliases or get inlined into the PROTOs

After M8f the validator runs structural rules (core, fail-fast) THEN PROTO rules (status-aware, soft when draft). Cleaner separation.

## What this CONCEPT does NOT decide

- Per-rule promotion decision finalisation — review during M8f impl
- Whether to break the existing `src/validator/rules/` directory (likely yes; clean cutover preserves test coverage)
- Whether structural rules are themselves PROTO-eligible (no — they're system-critical, not policy)

## Source

`[[CONCEPT--MSP-ROADMAP]]` §2 M8f, `[[CONCEPT--PROTO-PATTERN]]`, audit of `src/validator/rules/`.

## Connections

- [[FEAT--MSP-VALIDATOR]]
