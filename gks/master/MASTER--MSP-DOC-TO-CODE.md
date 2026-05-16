---
id: MASTER--MSP-DOC-TO-CODE
phase: 0
type: master
status: stable
tier: master
source_type: axiomatic
promoted_from: CONCEPT--CODEGEN-MICROTASK-CONTRACT
promoted_at: 2026-05-09T08:00:00.000Z
promotion_adr: ADR--MASTER-PROMOTION-DOC-TO-CODE
vault_id: default
title: Doc-to-code — atoms before code, every milestone, no exceptions
tags:
  - msp
  - master
  - doc-to-code
  - governance
  - instinct
crosslinks: {"references":["FRAMEWORK--KNOWLEDGE-3-TIER","ADR--MASTER-PROMOTION-DOC-TO-CODE","CONCEPT--CODEGEN-MICROTASK-CONTRACT"]}
created_at: 2026-05-09T15:00:30.000+07:00
---

# MASTER — Doc-to-code (atoms before code, every milestone)

## Intent

For any non-trivial change, write the governing atoms before writing code, in this order: FRAME → CONCEPT → ADR/FEAT → BLUEPRINT → CODE → AUDIT.

## Why

The atom chain is the durable record of why a change exists. Code without atoms is undocumented intent; atoms without code are unproven claims. Writing the chain first surfaces design conflicts cheaply, keeps reviewers on the same map as authors, and makes future supersession traceable. Every shipped MSP milestone (M0–M9) has followed this order; deviations have always cost a follow-up cleanup PR.

## Directives

1. Open a new branch only after identifying which atom layer the change starts at. If unsure, write a CONCEPT first.
2. Land FRAME / CONCEPT / ADR / FEAT / BLUEPRINT atoms in `gks/<type>/` before any code under `src/`, `test/`, `scripts/`, or `web/` that implements them.
3. Validate atoms (`npx tsx src/validator/cli.ts --all`) and resolve crosslinks (`npm run msp:check-links`) before staging code changes — pre-commit hook enforces this.
4. Skip Phase 4 (TASK) only for single-developer slices; bigger handoffs require it (see `[[ADR--PROMOTION-LEVELS]]`).
5. Close every milestone with an AUDIT atom describing what shipped vs. the BLUEPRINT — including any deviations.

## Apply when

A new branch is opened, a PR is drafted, or any file under `src/`, `test/`, `scripts/`, or `web/` is created or substantially modified. Doc-only PRs (typo fixes, README) are exempt; refactors that preserve behavior still require the AUDIT step.

## Conflicts with

(none currently — flag any future Master that contradicts this directive.)

## Connections
- [[CONCEPT--CODEGEN-MICROTASK-CONTRACT]]
- [[ADR--MASTER-PROMOTION-DOC-TO-CODE]]
- [[FRAMEWORK--KNOWLEDGE-3-TIER]]

