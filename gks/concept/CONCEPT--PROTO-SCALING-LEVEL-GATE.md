---
id: CONCEPT--PROTO-SCALING-LEVEL-GATE
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: PROTO--SCALING-LEVEL-GATE — auto-detect L1/L2/L3 from PR diff + check
  required-atom set
tags:
  - msp
  - proto
  - scaling-levels
  - governance
  - m8c
crosslinks:
  references:
    - CONCEPT--PROTO-PATTERN
    - FRAMEWORK--SCALING-LEVELS
created_at: 2026-05-05T16:28:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  domain: concept
---

# CONCEPT — [[PROTO--SCALING-LEVEL-GATE]]

## Problem

`[[FRAMEWORK--SCALING-LEVELS]]` defines L1/L2/L3 — escalating impact tiers requiring more documentation:

- **L1** (typo, comment, single-file refactor): no atom required
- **L2** (single-feature change ≤ N files): CONCEPT + ADR + FEAT
- **L3** (multi-module / breaking): full chain incl. BLUEPRINT + AUDIT

Today nothing classifies a PR's level. A PR touching 8 modules can ship with just an ADR, defeating the framework.

## Rule

At PR time, compute level from diff:

| Heuristic | Level |
|---|---|
| 1 file changed, ≤ 10 lines | L1 |
| 2–4 files changed OR 11–100 lines OR `src/` not touched | L2 |
| 5+ `src/` files OR > 100 lines OR breaking export change | L3 |

For each level, require the atom set per FRAME (or explicit `level_override` in PR description).

## Trigger

PR diff inspection. Runs in CI workflow (not local validate, since needs git diff against base). Could also run via `gks pr-check` CLI hook.

## Severity

`error` for L3 PRs missing required atoms. `warning` for L2 missing AUDIT.

## What this CONCEPT does NOT decide

- Heuristic threshold tuning — likely starts conservative (L1 cap = 10 lines), tunes via `[[PARAM--SCALING-LEVEL-THRESHOLDS]]`
- Predicate impl
- Override syntax in PR description

## Source

`[[FRAMEWORK--SCALING-LEVELS]]`, `[[CONCEPT--MSP-ROADMAP]]` §2 M8c.

## Connections
- [[CONCEPT--PROTO-PATTERN]]

