---
id: ADR--DELEGATION-POLICY
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Delegation policy — L2 atoms auto-approved by 2 senior, L3 Boss-only
tags:
  - msp
  - delegation
  - authority
  - human-review
  - decision
  - m9b
crosslinks:
  references:
    - FRAMEWORK--AUTHORITY-MATRIX
    - ADR--HUMAN-REVIEW-GATES
    - CONCEPT--PROTO-AUTHORITY-ENFORCEMENT
created_at: 2026-05-05T16:28:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — delegation policy

## Context

`[[ADR--HUMAN-REVIEW-GATES]]` (M5d) requires human approval for promotions. As teams grow past 5 contributors, the Boss becomes a bottleneck — every ADR / FEAT promotion stalls waiting on a single approver. Either:

(a) Boss approves everything (rate-limit on team velocity)
(b) Bypass review (defeats the framework)

A delegation policy is needed.

## Decision

### Tier-aware approval matrix

| Atom level | Required approvers |
|---|---|
| **L1** (typo / cosmetic / single-file refactor) | 1 senior (T2 or T3) |
| **L2** (single-feature CONCEPT/FEAT/ADR) | **2 senior** (T2 or T3); does NOT need Boss |
| **L3** (multi-module / breaking / new framework) | **Boss only** (T3) |
| **HOTFIX** | 1 senior; backfill within 48h |

"Senior" = T2 or T3, per `[[FRAMEWORK--AUTHORITY-MATRIX]]`.

### Auto-promote heuristic

PRs labelled `level:L2` AND with 2 senior approvals AND CI green can auto-merge via existing GitHub auto-merge. PRs labelled `level:L3` always require Boss explicit merge action — no auto-merge.

PR level is computed by `[[PROTO--SCALING-LEVEL-GATE]]` (M8c) from diff. Authors can override via `level_override:` in PR description; senior reviewers can over-rule the override.

### Boss escalation triggers

Even at L2, certain content escalates to Boss-required:

- Any change to `gks/frame/` (FRAME atoms)
- Any change to `src/validator/contract.ts` (contract loader)
- Any change to `examples/hooks/` (pre-commit / pre-push)
- Any change touching `package.json` or `package-lock.json` (dep changes)

These match `[[FRAMEWORK--AUTHORITY-MATRIX]]` "Boss-only paths".

### Quorum override

If only 1 senior is available (e.g. small team starting out, vacation), Boss can issue a `quorum: 1` override per-PR via comment. Documented as a HOTFIX-style escape hatch with a 48-hour backfill review.

## Consequences

**Positive**
- Boss not the rate limiter on routine work
- 2-senior gate preserves the "enough eyes" principle (most bugs caught by code review have ≥ 2 reviewers)
- L3 / framework / contract / hooks remain Boss-controlled
- Quorum override gives small-team flexibility without abandoning the model

**Negative**
- Requires team to actually have 2+ seniors (T2 or T3 pool of ≥ 3)
- Reviewers must self-identify their tier — needs `.brain/msp/authority.yaml` populated
- "Quorum override" can be abused; mitigated by 48-hour backfill review

## Alternatives considered

1. **Boss approves everything always.** Rejected — bottleneck.
2. **Anyone can approve anything.** Rejected — defeats authority matrix.
3. **Approval count proportional to LOC.** Rejected — gameable; level-based heuristic better.
4. **Cryptographic signing for approvals.** Rejected — overhead vs benefit poor; GitHub's existing approval is enough.

## What this ADR does NOT decide

- **Who decides who's T2 vs T3** — out-of-band onboarding decision; recorded in `.brain/msp/authority.yaml`
- **Approval timeout** — when does an L2 PR with only 1 approver auto-escalate to Boss? Out of scope; could be follow-up
- **Branch-protection rule encoding** — implementation detail of GitHub config, not policy
- **Cross-org delegation** — out of M9 scope

## Source

`[[FRAMEWORK--AUTHORITY-MATRIX]]`, `[[ADR--HUMAN-REVIEW-GATES]]`, `[[CONCEPT--MSP-ROADMAP]]` §3 M9b, user direction (all-M planning) — needed before team scales past 5 contributors.

## Connections
- [[CONCEPT--PROTO-AUTHORITY-ENFORCEMENT]]

