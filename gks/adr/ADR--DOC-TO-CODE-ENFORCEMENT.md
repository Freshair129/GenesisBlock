---
id: ADR--DOC-TO-CODE-ENFORCEMENT
phase: 2
type: adr
status: stable
created_at: 2026-05-13T12:00:00+07:00
vault_id: GKS-CORE
tier: genesis
title: Doc-to-code enforcement model (master-spec §6 → GKS primitives)
tags: &a1
  - workflow
  - enforcement
  - agent-rule
  - hotfix
  - msp-gatekeeper
crosslinks: &a2
  references:
    - ADR--EXTENDED-TAXONOMY
    - ADR--FLAT-ATOM-LAYOUT
    - ADR--REVERSE-CITATION-LOOKUP
    - CONCEPT--MASTER-PROMOTION
  partially_superseded_by:
    - ADR--TASK-TRACKING-AT-ORCHESTRATOR
linked_symbols: &a3
  - file: packages/gks/src/memory/types.ts
    fn: normaliseStatus
  - file: packages/gks/src/memory/types.ts
    fn: isApprovedStatus
  - file: packages/gks/src/memory/verify-flow.ts
    fn: verifyFlow
  - file: packages/gks/src/memory/validate-links.ts
    fn: validateLinks
  - file: packages/gks/src/scaffold/new-feature.ts
    fn: scaffoldNewFeature
  - file: packages/gks/src/hotfix/store.ts
  - file: packages/gks/src/hotfix/types.ts
  - file: packages/gks/bin/gks.ts
    fn: cmdVerifyFlow
  - file: packages/gks/bin/gks.ts
    fn: cmdValidate
  - file: packages/gks/bin/gks.ts
    fn: cmdNewFeature
  - file: packages/gks/bin/gks.ts
    fn: cmdHotfix
aliases: &a4
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--DOC-TO-CODE-ENFORCEMENT
  phase: 2
  type: adr
  status: stable
  created_at: 2026-05-13T12:00:00+07:00
  vault_id: GKS-CORE
  tier: genesis
  title: Doc-to-code enforcement model (master-spec §6 → GKS primitives)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  aliases: *a4
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--DOC-TO-CODE-ENFORCEMENT
    phase: 2
    type: adr
    status: stable
    created_at: 2026-05-13T12:00:00+07:00
    vault_id: GKS-CORE
    tier: genesis
    title: Doc-to-code enforcement model (master-spec §6 → GKS primitives)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    aliases: *a4
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      domain: adr
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — Doc-to-code enforcement model

## Context

`FRAMEWORK_MASTER_SPEC.md` §6 prescribes a six-phase doc-to-code flow
(P1 CONCEPT → P2 ADR/ENTITY/API → P3 BLUEPRINT → P4 task → P5 src →
P6 AUDIT) plus an Agent Rule, a Hotfix Escape Hatch, and a CLI surface.
GKS already had the atom **types** (ADR-012 taxonomy) and the
**inbound queue** but no chain-walking enforcement turning the spec
into a hard gate.

## Decision

Implement the six gaps as a thin enforcement layer over GKS
primitives (no new storage semantics, ADR-008 boundary preserved):

| Item | Surface |
|---|---|
| 1. ~~`TASK--` prefix~~ | **superseded by [[ADR--TASK-TRACKING-AT-ORCHESTRATOR]] (ADR-015)** — task state lives at the orchestrator, not as atoms |
| 2. `status: approved` alias | `normaliseStatus` / `isApprovedStatus` map APPROVED→stable |
| 3. Chain walker | `gks verify-flow <id>` — exits 1 on broken edge / not-stable node |
| 4. Hotfix escape hatch | `HOTFIX--<sha>` atom + 48 h timer + `gks hotfix check` pre-commit gate |
| 5. `gks new-feature` scaffolder | drops 4 candidates (CONCEPT/ADR/FEAT/BLUEPRINT) into inbound; microtasks via `--task-tracker=local\|msp\|external` |
| 6. Link checker | `gks validate --links` — read-only crosslink integrity |

## Consequences

**Positive** — master-spec §6 becomes mechanically enforceable.
Agent Rule §6.3 reduces to `gks verify-flow <FEAT> && exit-0`.
Hotfix §6.4 leaves an auditable atom with a hard 48 h deadline plus
explicit backfill provenance via `crosslinks.resolves`.

**Negative** — ~600 LOC added, plus per-pre-commit chain-walk cost on
projects that opt in (50–200 ms in the typical graph). The
`stable=APPROVED` alias is doc debt; mitigated by accepting both
spellings at the CLI/MCP boundary.

## What this does NOT change

- ADR-008 storage-engine scope — every item is a primitive over atom data.
- ADR-009 peer-subsystem boundary — orchestration timing stays in MSP.
- ADR-010 `linked_symbols` semantics — re-used by item 4's pre-commit gate.
- ADR-012 taxonomy — adds TASK-- and HOTFIX-- to the recognised set.
- Inbound queue mechanics — the new-feature scaffolder writes through it.

## References

- `docs/adr/014-doc-to-code-enforcement.md` — full text
- `FRAMEWORK_MASTER_SPEC.md` §6 (P1-P6, Agent Rule, Hotfix, CLI), §7 (MSP Gatekeeper)
- ADR-008 / ADR-009 / ADR-010 / ADR-012 — boundary + read-side primitives

## Connections
- [[ADR--EXTENDED-TAXONOMY]]
- [[ADR--FLAT-ATOM-LAYOUT]]
- [[ADR--REVERSE-CITATION-LOOKUP]]
- [[CONCEPT--MASTER-PROMOTION]]

