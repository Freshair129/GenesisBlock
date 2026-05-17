---
id: CONCEPT--PROTO-AUTHORITY-ENFORCEMENT
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: PROTO--AUTHORITY-ENFORCEMENT — git author tier ↔ touched paths
tags:
  - msp
  - proto
  - authority
  - tier
  - governance
  - m8e
crosslinks: {"references":["CONCEPT--PROTO-PATTERN","FRAMEWORK--AUTHORITY-MATRIX","ADR--HUMAN-REVIEW-GATES"]}
created_at: 2026-05-05T16:28:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — [[PROTO--AUTHORITY-ENFORCEMENT]]

## Problem

`[[FRAMEWORK--AUTHORITY-MATRIX]]` defines tiers (T1/T2/T3) with allowed write paths:

- T1: `inbound/` only
- T2: `gks/concept/`, `gks/feat/` + inbound
- T3 (Boss): everything

Today nothing checks the git author's tier when reviewing changed paths. A T1 agent could land an ADR-- via inbound + auto-promote, defeating the matrix.

## Rule

For each PR, identify author tier (from a config map: github username → tier). For each path touched, check the tier's write-allow list per `[[FRAMEWORK--AUTHORITY-MATRIX]]`.

Promotion path: T1 atoms always go through inbound + human review. Direct commits to `gks/<protected>/` from a T1 author → CI fail.

## Trigger

CI workflow. Predicate reads:
- Git author email / login from `git log`
- Path list from `git diff --name-only base..head`
- Tier map from `.brain/msp/authority.yaml` (project-local)

## Severity

`error` for T1/T2 violating their allowed-paths. `warning` for T3 actions that bypass human review (gentle nudge, not block — Boss has authority but should know).

## Configuration shape

`.brain/msp/authority.yaml`:

```yaml
tiers:
  T1: ["agent-junior", "claude-opus-4-7-junior"]
  T2: ["alice", "bob"]
  T3: ["boss"]
allowed_paths:
  T1: [".brain/msp/projects/*/inbound/**"]
  T2: [".brain/msp/projects/*/inbound/**", "gks/concept/**", "gks/feat/**"]
  T3: ["**"]
```

Falls back to `T1` (most restrictive) when author not in the map.

## What this CONCEPT does NOT decide

- Default fallback (T1 here, but could be config-driven)
- Multi-author handling (PRs with several committers)
- How tier transitions are announced (out of MSP scope)

## Source

`[[FRAMEWORK--AUTHORITY-MATRIX]]`, `[[ADR--HUMAN-REVIEW-GATES]]`, `[[CONCEPT--MSP-ROADMAP]]` §2 M8e.

## Connections
- [[CONCEPT--PROTO-PATTERN]]

