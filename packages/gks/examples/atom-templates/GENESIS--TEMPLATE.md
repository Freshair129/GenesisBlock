---
id: GENESIS--TEMPLATE
phase: 0                     # 0 | 1 | 2 | 3 | 4 | 5 | 6
type: genesis
status: active                  # stub | raw | draft | active | stable | deprecated | superseded | partial
vault_id: default
tier: master                  # safety | master | genesis | process
source_type: axiomatic          # axiomatic | learned
title: <Genesis Block name — Engine — short purpose>
created_at: 2026-05-13T12:00:00.000+07:00
tags: [manifest, knowledge-block]
aliases:
  - GENESIS
  - implementation_flow
  - Block Manifest (v2.3+)
cluster: implementation_flow
role: Block Manifest (v2.3+)
crosslinks:
  references: []
linked_symbols: []
attributes:
  manifest_version: 0.1.0
  domain: general
  members:
    core:
      cognitive: [COGNITIVE--<...>]
      algo:      [ALGO--<...>]
      runbook:   [RUNBOOK--<...>]
      concept:   [CONCEPT--<...>]
      params:    [PARAMS--<...>]
    optional:
      guard:     []
      safety:    []
      stack:     []
      protocol:  []
      mod:       []
      spec:      []
  daci:
    driver:       MOD--<...>
    approver:     []
    contributor:  []
    informed:     []
---

# GENESIS — <Name>

## Manifest Members

Describe the composite knowledge units included in this block and their roles.

- **Cognitive**: COGNITIVE--...
- **Algo**: ALGO--...
- **Runbook**: RUNBOOK--...
- **Concept**: CONCEPT--...
- **Params**: PARAMS--...

## DACI

Define roles and responsibilities for this block:
- **Driver**: MOD--... (The primary driving component)
- **Approver**: Component or entity that signs off.
- **Contributor**: Contributors to this block.
- **Informed**: Entities notified of changes.

## Source

- <Link to the requirements, discussion, or PR that established this block>
