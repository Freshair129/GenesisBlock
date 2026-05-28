---
id: MASTER--TEMPLATE
phase: 0                     # 0 | 1 | 2 | 3 | 4 | 5 | 6
type: master
status: active                  # stub | raw | draft | active | stable | deprecated | superseded | partial
vault_id: default
tier: master                  # safety | master | genesis | process
source_type: axiomatic          # axiomatic | learned
title: <Short policy title>
tags: [governance, policy]
aliases:
  - MASTER
  - implementation_flow
  - Root-level policy / genesis rule
cluster: implementation_flow
role: Root-level policy / genesis rule
crosslinks:
  references: []
  supersedes: []
linked_symbols: []
attributes:
  domain: general
promoted_from: <Genesis-atom-id-optional>
promoted_at: <ISO-timestamp-optional>
promotion_adr: <ADR-link-optional>
---

# MASTER — <Title>

## Policy

Describe the core directive or rule that must be strictly enforced.

## Scope

Define where, when, and to which components or actors this policy applies.

## Enforcement

Describe how this policy is enforced (e.g. static analysis, compiler rule, manual review, or validation script).

## Source

- <Link to the genesis of this rule or discussion>
