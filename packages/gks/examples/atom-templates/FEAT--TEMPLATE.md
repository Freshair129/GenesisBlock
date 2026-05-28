---
id: FEAT--TEMPLATE
tier: genesis                  # safety | master | genesis | process
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2                     # 0 | 1 | 2 | 3 | 4 | 5 | 6
type: feat
status: active                  # stub | raw | draft | active | stable | deprecated | superseded | partial
vault_id: <YOUR-PROJECT>
title: <One-line feature summary>
tags: [user-facing]
domain: <area>
crosslinks:
  implements: []                # FR-- / NFR-- this feature satisfies (Requirement Link)
  satisfies: []                 # Explicitly link to functional/non-functional requirements
  references: []                # CONCEPT-- / ADR-- background context (Context Link)
  governed_by: []               # ADR-- that dictates how this feature is built (Governance Link)
  blueprint: BLUEPRINT--<feature-id> # The implementation plan for this feature (Plan Link)
linked_symbols: []              # files / functions implementing this (Code Citation)
---

# FEAT — <Title>

## User-facing behaviour

When user does X, system Y. Describe in plain language; no implementation
details. Reviewer should be able to write acceptance criteria from this.

## Acceptance criteria

- [ ] <observable behaviour 1>
- [ ] <observable behaviour 2>
- [ ] error case: <what should happen when X fails>

## Out of scope

- <related-but-deferred concerns>

## Open questions

- <any unresolved spec questions>
