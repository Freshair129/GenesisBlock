---
id: PROTO--EVIDENCE-FOR-DECISIONS
phase: 2
type: proto
status: stable
severity: error
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--EVIDENCE-FOR-DECISIONS — ADR body requires Context/Decision/Consequences
tags:
  - msp
  - proto
  - adr
  - evidence
  - validator
  - m8f
crosslinks:
  enforces:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
  references:
    - CONCEPT--PROTO-PATTERN
    - CONCEPT--PROTO-AUDIT-EXISTING-RULES
    - FEAT--PROTO-LOADER
    - ADR--ANTI-HALLUCINATION-RULES
linked_symbols:
  - file: packages/msp/src/validator/proto/evidence-for-decisions.ts
created_at: 2026-05-05T20:00:00.000+07:00
aliases:
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  domain: proto
---

# PROTO — EVIDENCE-FOR-DECISIONS

## Rule

Atoms with `type: adr` must contain headings (case-insensitive):
- `## Context` (or `# Context`)
- `## Decision` (or `# Decision`)
- `## Consequences` (or `# Consequences`)

Catches "decision-without-evidence" hallucinations: an agent writes `## Decision` but skips `## Context`, leaving a verdict without justification.

## Schema

Per-atom body (markdown). Frontmatter `type: adr` filter.

## Predicate

Wraps existing core rule `evidenceForDecisions` from `src/validator/rules/evidence-for-decisions.ts` via `ruleAdapter`. Filter: `type === 'adr'`.

## Trigger

`msp:validate --all`.

## Severity

`error` — un-evidenced ADRs are anti-pattern.

## Status

`draft` — overlap with core rule until cutover (M8f-2).

## Source

`[[CONCEPT--PROTO-AUDIT-EXISTING-RULES]]`, `[[ADR--ANTI-HALLUCINATION-RULES]]`.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--PROTO-PATTERN]]
- [[FEAT--PROTO-LOADER]]

