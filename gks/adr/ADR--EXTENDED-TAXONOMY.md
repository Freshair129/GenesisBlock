---
id: ADR--EXTENDED-TAXONOMY
phase: 2
type: adr
status: stable
created_at: 2026-05-13T12:00:00+07:00
vault_id: GKS-CORE
tier: genesis
title: Extended atomic taxonomy + ISSUE-- as self-hosted tracker
tags:
  - taxonomy
  - governance
  - issue-tracking
  - scope
crosslinks:
  references:
    - FRAMEWORK--FOUR-LAYERS
    - ADR--FLAT-ATOM-LAYOUT
    - CONCEPT--TAXONOMY-V2-3
    - ADR--REGISTRY-DRIVEN-SCAFFOLDING
  resolves: []
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — Extended atomic taxonomy

## Context

The original 17-prefix taxonomy was implementation-first
(IDEA → CONCEPT → ADR/FEAT/ALGO/ENTITY → BLUEPRINT → microtask → code →
AUDIT). Three orthogonal axes were missing:

1. **Agent governance** — skills, protocols, guardrails, policies, personas
2. **Requirements engineering** — FR vs NFR vs constraint (split from `[[CONCEPT--REQ]]`)
3. **Ops governance** — incident post-mortems, issue tracker, risks, runbooks, SLOs

Plus an explicit "we must work without Linear/Jira" requirement.

## Decision

Extend taxonomy from **17 → ~30 core prefixes** organised into four
named clusters. Elevate `ISSUE--` to first-class self-hosted tracker
with its own light-governance tier:

| Tier | Atoms | Governance |
|---|---|---|
| **Strict** | ADR / BLUEPRINT / CONCEPT / FEAT / FRAME / ENTITY / API / etc. | inbound queue → human review → promote |
| **Light** | `ISSUE--` (initially) | direct write OK; schema-validated; comments append-only |

`SOLUTION--` rejected — solutions are ADRs in disguise; use
`crosslinks.resolves: [INC--, ISSUE--]`.

## Consequences

**Positive** — every atomic concern has a canonical prefix.
Self-hosted tracking removes Linear dependency. Clean traceability
chain (ADR resolves ISSUE resolves INC).

**Negative** — larger surface (30+ prefixes) for new contributors.
Mitigated by `docs/KNOWLEDGE-TYPES.md` + `examples/atom-templates/`.

## What ships in this ADR

- `docs/KNOWLEDGE-TYPES.md` — canonical reference (Note: Superseded by `atom_registry.yaml` per `[[ADR--REGISTRY-DRIVEN-SCAFFOLDING]]`)
- `examples/atom-templates/` — 17 starter `.md` templates
- This ADR records the decision; the ISSUE-- CLI is shipped separately
  (closed in 3.5.4).

## Connections
- [[FRAMEWORK--FOUR-LAYERS]]
- [[ADR--FLAT-ATOM-LAYOUT]]
- [[CONCEPT--TAXONOMY-V2-3]]

