---
id: ENTITY--TEMPLATE
tier: genesis                  # safety | master | genesis | process
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2                     # 0 | 1 | 2 | 3 | 4 | 5 | 6
type: entity
status: active                  # stub | raw | draft | active | stable | deprecated | superseded | partial
vault_id: <YOUR-PROJECT>
title: <Entity name + brief role>
tags: [data-model]
domain: <domain-name>
crosslinks:
  used_by: []                   # FEAT-- / ENDPOINT-- that consume this (Manual Backlink)
  related_entities: []          # ENTITY-- with FK / association (Peer Link)
  part_of: []                   # MOD-- this entity belongs to (Hierarchical Link)
  references: []                # General references or external schemas
---

# ENTITY — <Name>

## Schema

```yaml
fields:
  id:
    type: string                # uuid | string | int | …
    required: true
    description: <semantics>
  created_at:
    type: timestamp
    required: true
  # …
```

## Invariants

- <constraint that must always hold>

## Lifecycle

- created when: ...
- archived when: ...
- never deleted (or: hard-deleted on policy X)

## Indexes

- by `<field>` — used by FEAT-- / ENDPOINT-- ...

## See also

- DB migration: <migration ID>
