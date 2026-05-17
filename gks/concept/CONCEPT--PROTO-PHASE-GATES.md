---
id: CONCEPT--PROTO-PHASE-GATES
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: PROTO--PHASE-GATES — enforce P0 → P6 ordering at PR-time
tags: &a1
  - msp
  - proto
  - phase-gates
  - governance
  - m8b
crosslinks: &a2
  references:
    - CONCEPT--PROTO-PATTERN
    - FRAMEWORK--PHASE-GOVERNANCE
    - FEAT--PROTO-LOADER
created_at: 2026-05-05T16:28:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--PROTO-PHASE-GATES
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: PROTO--PHASE-GATES — enforce P0 → P6 ordering at PR-time
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-05T16:28:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--PROTO-PHASE-GATES
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: PROTO--PHASE-GATES — enforce P0 → P6 ordering at PR-time
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
---

# CONCEPT — [[PROTO--PHASE-GATES]]

## Problem

`[[FRAMEWORK--PHASE-GOVERNANCE]]` declares a phase order: P0 FRAME → P1 CONCEPT → P2 ADR/FEAT → P3 BLUEPRINT → P5 CODE → P6 AUDIT. Today the validator does not enforce this — a PR landing P5 code without a P3 BLUEPRINT slips through.

## Rule

For every `linked_symbols` source file `X` referenced by a phase-5 atom (FEAT/AUDIT writing code), there MUST exist either:

- A phase-3 BLUEPRINT atom whose `linked_symbols` includes `X`, OR
- An explicit override on the atom: `phase_override: { skip_blueprint: true, reason: "..." }` (out-of-scope code under examples/, hooks/, etc.)

Equivalent rules for ADR-before-FEAT and CONCEPT-before-ADR (soft warnings; stricter than P5).

## Trigger

CI workflow + `npm run msp:validate --all`. As a PROTO atom (per M8a), it lives at `gks/proto/[[PROTO--PHASE-GATES]].md` with predicate at `src/validator/proto/phase-gates.ts`.

## Severity

`error` for missing BLUEPRINT before P5 CODE. `warning` for missing CONCEPT before ADR (often legitimate for tiny clarifications).

## What this CONCEPT does NOT decide

- Predicate impl details (lives in [[BLUEPRINT--PHASE-GATES]], future work)
- The exact FRAME atom this PROTO `enforces:` (likely `[[FRAMEWORK--PHASE-GOVERNANCE]]`)
- Override field shape — to be designed when impl PR opens

## Source

`[[FRAMEWORK--PHASE-GOVERNANCE]]`, `[[CONCEPT--MSP-ROADMAP]]` §2 M8b, `[[CONCEPT--PROTO-PATTERN]]` (M8a foundation).

## Connections
- [[FEAT--PROTO-LOADER]]

