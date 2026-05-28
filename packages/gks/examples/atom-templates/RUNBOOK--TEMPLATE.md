---
id: RUNBOOK--TEMPLATE
tier: genesis                  # safety | master | genesis | process
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2                     # 0 | 1 | 2 | 3 | 4 | 5 | 6
type: runbook
status: active                  # stub | raw | draft | active | stable | deprecated | superseded | partial
vault_id: <YOUR-PROJECT>
title: <One-line scenario>
tags: [runbook, on-call]
last_drilled: 2026-05-13T12:00:00.000+07:00
crosslinks:
  triggered_by: []              # alert / SLO-- breach / INC-- pattern (Event/Trigger Link)
  related_incidents: []         # INC-- where this runbook was used (Backlink/Peer Link)
  references: []                # ADR-- / FEAT-- background context (Context Link)
---

# RUNBOOK — <Scenario>

## When to invoke

Trigger condition (alert text, SLO breach, dashboard signal). Be
specific enough that whoever's on-call knows in 5 seconds whether this
runbook applies.

## Severity

- **page:** yes/no
- **escalation:** <after N minutes if X>

## Diagnosis

1. <check command 1>
2. <check command 2>
3. interpret result: ...

## Mitigation steps

1. ⚠ **first**: <stop-the-bleeding action>
2. ...
3. verify: <how to confirm mitigation worked>

## Communication

- status page: <update template>
- internal: <Slack channel>
- customer: <when to notify>

## Post-incident

- log INC--<id> within 24h
- update this runbook if steps differed from reality

## See also

- <SLO-- this runbook responds to>
- <related RUNBOOK-->
