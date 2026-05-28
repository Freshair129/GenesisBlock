---
id: FLOW--TEMPLATE
tier: genesis
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2
type: flow
status: active
vault_id: <YOUR-PROJECT>
title: <One-line flow summary>
domain: <domain-name>
tags: [data-flow|ui-flow|sequence]
crosslinks:
  participants: []              # MOD-- / ENTITY-- / ENDPOINT-- involved (Peer/Actor Link)
  references: []                # ADR-- / FEAT-- background context (Context Link)
  preceded_by: []               # FLOW-- that happens before this one (Sequence Link)
  followed_by: []               # FLOW-- that happens after this one (Sequence Link)
---

# FLOW — <Title>

## Trigger

What initiates this flow? (user action, scheduled job, upstream event)

## Sequence

```
Actor A ──→ Actor B: <message / data>
Actor B ──→ Actor C: <message / data>
                   ←──── <response>
Actor C ──→ Storage: <write>
```

## Failure modes

- if step N fails: <recovery / rollback>
- timeout: <handling>

## See also

- (Obsidian Canvas alternative: `FLOW--<name>.canvas` for visual layout)
