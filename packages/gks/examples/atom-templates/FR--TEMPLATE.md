---
id: FR--TEMPLATE
phase: 1                     # 0 | 1 | 2 | 3 | 4 | 5 | 6
type: fr
status: active                  # stub | raw | draft | active | stable | deprecated | superseded | partial
vault_id: default
tier: process                  # safety | master | genesis | process
source_type: axiomatic          # axiomatic | learned
title: <One-line functional requirement>
created_at: 2026-05-13T12:00:00.000+07:00
tags: [functional]
aliases:
  - FR
  - requirements
  - Functional requirement
cluster: requirements
role: Functional requirement
crosslinks:
  references: []
linked_symbols: []
attributes:
  priority: medium
  domain: requirements
---

# FR — <Title>

## Requirement

State the functional requirement. The system **shall** <observable behaviour>. It must be specific and testable.

## Verification

Describe the verification approach (e.g. unit test, E2E test, or manual validation).

## Source

- <Link to the REQ-- or CONCEPT-- that originating this requirement>
