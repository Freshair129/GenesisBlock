---
id: FRAMEWORK--SCALING-LEVELS
phase: 0
type: framework
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Scaling levels — L1/L2/L3 minimum artifact sets
tags: &a1
  - msp
  - scaling
  - governance
  - foundation
crosslinks: &a2
  references:
    - FRAMEWORK--PHASE-GOVERNANCE
created_at: 2026-05-03T14:01:48.787+07:00
aliases: &a3
  - FRAMEWORK
  - implementation_flow
  - Governance / architectural framework
cluster: implementation_flow
role: Governance / architectural framework
attributes:
  id: FRAMEWORK--SCALING-LEVELS
  phase: 0
  type: framework
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Scaling levels — L1/L2/L3 minimum artifact sets
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T14:01:48.787+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Governance / architectural framework
  attributes:
    id: FRAMEWORK--SCALING-LEVELS
    phase: 0
    type: framework
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Scaling levels — L1/L2/L3 minimum artifact sets
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T14:01:48.787+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Governance / architectural framework
    attributes:
      domain: framework
    domain: framework
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: framework
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# FRAME — scaling levels

Not every change deserves a 6-phase ceremony. MSP grades work into three Scaling Levels and requires only the artifacts proportional to the blast radius.

## Levels

| Scale | Use case | Required artifacts (minimum) |
|---|---|---|
| **L1** | Quick task, 1 concern, reversible (typo fix, single-file refactor) | `MSP-ACT-` per turn + `MSP-WKT-` walkthrough |
| **L2** | Feature/module of normal scope (a new endpoint, a new validator rule) | `CONCEPT--` + `ADR--` + **`API--`** + `T*` task + `MSP-WKT-` |
| **L3** | Major / core / critical (cross-module refactor, new subsystem) | `PRD` + `REQ` + `ADR--` + `FLOW--` + **`API--`** + `BLUEPRINT--` + `AUDIT--` + `MSP-WKT-` |

## Choosing a level (heuristic)

```
single file changed?         → L1
new public surface?          → L2
touches > 3 modules
  or affects auth/data layer → L3
```

When in doubt, escalate. Downgrading from L3→L2 mid-PR loses an audit trail; upgrading L1→L2 just means writing one more atom.

## What MSP enforces

- **L2/L3 PRs without the required atoms are blocked at `gks verify-flow`** — chain integrity refuses exit-0 if a required atom is missing or `draft`.
- **L1 changes still require `MSP-WKT-`** so the next agent picks up context.
- **Hotfix tag overrides level requirements** during the 48h window (see `[[ADR--HOTFIX-ESCAPE-HATCH]]`).

## Source

`msp_spec.md` §6.1 (Scaling Level → Required Artifacts).

## Connections
- [[FRAMEWORK--PHASE-GOVERNANCE]]

