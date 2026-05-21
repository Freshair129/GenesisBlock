---
id: ISSUE--TEMPLATE
tier: genesis
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2
type: issue
title: <Issue summary>
status: draft
priority: medium                # low | medium | high | urgent
assignee: <MSP-AGT-... or MSP-USR-...>     # optional initially
reporter: <MSP-USR-...>
labels: []
updated_at: 2026-05-13T12:00:00.000+07:00
linked_symbols: []
crosslinks:
  related_incidents: []         # INC-- if this issue stems from an incident (Backlink)
  resolved_by: []               # ADR-- / FEAT-- / HOTFIX-- when closing — what fixed it (Forward/Fix Link)
  duplicates_of: []             # ISSUE-- if this is a duplicate (Peer Link)
  blocks: []                    # ISSUE-- this one is blocking (Dependency Link)
  blocked_by: []                # ISSUE-- blocking this one (Dependency Link)
  references: []                # External discussions / logs / relevant background context
---

# ISSUE — <Short title>

## Description

What's the problem? Symptoms, scope, affected components.

## Reproduction

Steps to reproduce (when applicable):

1. ...
2. ...
3. observed: ...
4. expected: ...

## Impact

Who / what is affected. Severity / urgency rationale.

## Discussion

<!-- Append-only chronological — most recent at the bottom.
     Format: ### <ISO timestamp> [identity] <action>
     Status changes are logged here too. -->

### <ISO timestamp> [<MSP-USR-...>] draft

First report.

## Resolution

<!-- Filled when status: closed. Reference the ADR / commit / PR
     that resolved it. -->

_(pending)_
