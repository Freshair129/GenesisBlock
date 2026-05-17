---
id: CONCEPT--PROTO-ALGO-PARAM-COUPLING
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: PROTO--ALGO-PARAM-COUPLING — bi-directional tunes ↔ tunable_by reciprocal
  validator
tags:
  - msp
  - proto
  - algo
  - param
  - coupling
  - governance
  - m8d
crosslinks:
  references:
    - CONCEPT--PROTO-PATTERN
    - ADR--GRAPH-IS-GKS-DOMAIN
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

# CONCEPT — [[PROTO--ALGO-PARAM-COUPLING]]

## Problem

Algorithms (`ALGO--*`) have tunable parameters (`PARAM--*`). The atom taxonomy currently lacks a way to ensure these stay paired:

- An ALGO atom claiming `tunable_by: [[[PARAM--FOO]]]` should have a matching `tunes: [[ALGO--BAR]]` on `[[PARAM--FOO]]`
- Drift creates orphan ALGOs (claim params that don't exist) or orphan PARAMs (tune nothing)

Per `[[ADR--GRAPH-IS-GKS-DOMAIN]]`, **existence checks** belong in `gks validate --links`. The MSP-side rule is **type-pairing**: `tunable_by` must reference PARAM--, `tunes` must reference ALGO--, and the reciprocal link must exist.

## Rule

For every [[ALGO--X]] atom with `tunable_by: [[[PARAM--Y]], ...]`:
- Each [[PARAM--Y]] must have `tunes: [..., [[ALGO--X]], ...]`

And vice versa for every [[PARAM--Y]] with `tunes: [[[ALGO--X]], ...]`.

## Trigger

`msp:validate` and pre-commit hook. PROTO predicate runs against the atomic index.

## Severity

`error` — broken coupling means a documented tunable doesn't exist or vice versa.

## Scope after audit (M7-prep follow-up)

This PROTO is **smaller than originally scoped**: existence checks delegated to `gks validate --links` per `[[ADR--GRAPH-IS-GKS-DOMAIN]]`. MSP only enforces:

1. `tunable_by` values are PARAM-- IDs (not other types)
2. `tunes` values are ALGO-- IDs (not other types)
3. Reciprocal link exists in the partner atom

## What this CONCEPT does NOT decide

- Whether `tunes` allows multiple ALGO refs (likely yes; one PARAM can tune multiple algos)
- Predicate impl (M8d impl PR)
- Migration of existing un-paired atoms (if any)

## Source

`[[CONCEPT--MSP-ROADMAP]]` §2 M8d (post-audit scope), `[[CONCEPT--PROTO-PATTERN]]`, `[[ADR--GRAPH-IS-GKS-DOMAIN]]`.
