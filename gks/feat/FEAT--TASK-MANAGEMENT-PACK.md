---
id: FEAT--TASK-MANAGEMENT-PACK
phase: 2
type: feat
status: active
tier: process
source_type: axiomatic
vault_id: default
title: "FEAT — Task & Management Domain Pack — classifier and workflow policies"
tags:
  - msp
  - ucf
  - ops
  - task
crosslinks: {"implements":["CONCEPT--TASK-MANAGEMENT-PACK"],"references":["FEAT--CLASSIFIER-PLUGINS","FEAT--ISSUE-TRACKER"]}
created_at: 2026-05-17T10:20:00+07:00
cluster: implementation_flow
role: "Feature spec"
aliases:
  - FEAT
  - implementation_flow
---

# FEAT — Task & Management Domain Pack

## Context

This feature implements the operational management extension for UCF. It provides automated metadata extraction for issues and enforces workflow-specific security policies.

## Requirements

1. **Task Classifier:**
   - Detect `domain: ops` for ISSUE, RUNBOOK, and INCIDENT atoms.
   - Extract `priority` (low, medium, high, urgent) from frontmatter.
   - Extract `status` (open, in_progress, closed, blocked) from frontmatter.
   - Extract `assignee` from frontmatter.
2. **Policy Set:**
   - **Triage Protection:** Deny T1 agents from modifying priority or status on `open` issues.
   - **Urgent Escalation:** Force Step-up Auth (PIN) for any modification to `urgent` issues.
   - **Closure Gate:** Deny closing an issue if no `AUDIT` or `ADR` is linked in `crosslinks.resolved_by`.

## API Contract (Classifier)

Implemented as a standard UCF Classifier plugin.

- **ID:** `domain/task`
- **Outputs:** `issue_priority`, `issue_status`, `assignee`, `is_operational`

## Policy Rules

Rules reside in `policies/70-task-management.yaml`.

| ID | Description | Effect |
|---|---|---|
| `force-pin-for-urgent` | Require Step-up Auth for urgent tasks | `deny` + `advice(pin)` if `R.issue_priority == urgent` |
| `restrict-triage-to-t2` | Only T2+ agents can triage issues | `deny` if `S.tier == T1` and `R.issue_status == open` |

## Verification Criteria

- Classifier accurately extracts priority from an `ISSUE--` atom.
- PEP correctly issues a Step-up Auth challenge when a user attempts to modify an urgent issue.
- T1 agent is unable to transition an issue from `open` to `in_progress`.
