---
id: RISK--TEMPLATE
tier: genesis                  # safety | master | genesis | process
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2                     # 0 | 1 | 2 | 3 | 4 | 5 | 6
type: risk
status: active                  # stub | raw | draft | active | stable | deprecated | superseded | partial
likelihood: medium              # low | medium | high
impact: medium                  # low | medium | high | critical
vault_id: <YOUR-PROJECT>
title: <One-line risk summary>
tags: [risk]
identified_at: 2026-05-13T12:00:00.000+07:00
crosslinks:
  mitigated_by: []              # ADR-- / GUARDRAIL-- / RUNBOOK-- addressing this (Inverse Link)
  related_incidents: []         # INC-- where this risk materialised (becomes one) (Backlink/Peer Link)
  references: []                # Context / background for this risk (Context Link)
---

# RISK — <Title>

## Description

What could go wrong, and under what circumstances.

## Likelihood × Impact

- **Likelihood:** <low / medium / high> — <reasoning>
- **Impact:** <low / medium / high / critical> — <reasoning>
- **Score:** <combined>

## Indicators

Early-warning signs that this risk is materialising:

- ...

## Mitigations

| Mitigation | Status | Owner |
|---|---|---|
| <approach 1> | <planned/in-progress/done> | ... |

## Acceptance

If mitigation is "accept the risk":

- **why:** ...
- **review:** <when to re-evaluate>
