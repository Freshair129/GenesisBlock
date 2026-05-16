---
id: CONCEPT--DECISION-ATROPHY-GUARDS
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Decision atrophy — valid_until enforcement + scheduled review report
tags:
  - msp
  - lifecycle
  - valid-until
  - atrophy
  - m9a
crosslinks: {"references":["CONCEPT--PROTO-PATTERN","CONCEPT--MSP-ROADMAP"]}
created_at: 2026-05-05T16:28:00.000+07:00
---

# CONCEPT — decision atrophy guards

## Problem

ADRs have `valid_until` semantics in some atoms (e.g. `[[CONCEPT--MSP-ROADMAP]]` has `valid_until: 2026-08-01`). When that date passes:

- The atom is **silently stale** — nothing alerts the team
- Agents continue citing it as authoritative
- Drift between "what we said" and "what we now believe" isn't visible until someone happens to re-read

A library of "enforced lies" forms over months.

## Two guards

### Guard 1 — `valid_until_check` (CI / weekly cron)

A scheduled job that scans `gks/` for atoms with `valid_until` past `now()`. Emits:
- A list to a designated `STALE_ATOMS` issue (or local file `stale_atoms.md`)
- Optional: PR comment when a touched atom has `valid_until` in next 30 days

### Guard 2 — `valid_until` required for certain atom types

Some atoms inherently age (project plans, embedded benchmarks, time-bound preferences). Required-fields contract gets:

```yaml
roadmap_concepts:
  required_frontmatter: [valid_until]
  default_valid_period_days: 90
```

Applied to atoms tagged `tags: [roadmap, planning, ...]` (or per-id pattern).

## Why a CONCEPT not just a PROTO

This is bigger than one rule:
- M9a-1 = the scanning job (PROTO)
- M9a-2 = the required-fields contract update
- M9a-3 = process — what to do with stale atoms (review / supersede / update)

The CONCEPT captures the lifecycle aspect; specific PROTOs follow.

## Trigger

- **valid_until_check PROTO**: weekly CI cron, plus on-demand via `npm run msp:check-stale`
- **required-fields**: existing validator rule (M5d infrastructure)

## Severity

`warning` — atrophy is a process concern, not a CI fail. Emits to issue-tracker / stdout.

## Out of scope

- Auto-supersede of stale atoms (humans review, decide)
- ML predicting which atoms will age fastest

## Source

`[[CONCEPT--MSP-ROADMAP]]` §3 M9a.

## Connections
- [[CONCEPT--PROTO-PATTERN]]

