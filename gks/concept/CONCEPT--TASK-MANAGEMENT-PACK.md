---
id: CONCEPT--TASK-MANAGEMENT-PACK
phase: 1
type: concept
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Task & Management Domain Pack — operational workflow enforcement
aliases: &a1
  - CONCEPT
tags: &a2
  - msp
  - ucf
  - ops
  - management
crosslinks: &a3
  references:
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
    - ADR--TASK-TRACKING-AT-ORCHESTRATOR
created_at: 2026-05-17T10:15:00+07:00
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--TASK-MANAGEMENT-PACK
  phase: 1
  type: concept
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Task & Management Domain Pack — operational workflow enforcement
  aliases: *a1
  tags: *a2
  crosslinks: *a3
  created_at: 2026-05-17T10:15:00+07:00
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--TASK-MANAGEMENT-PACK
    phase: 1
    type: concept
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Task & Management Domain Pack — operational workflow enforcement
    tags: *a2
    crosslinks: *a3
    created_at: 2026-05-17T10:15:00+07:00
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--TASK-MANAGEMENT-PACK
      phase: 1
      type: concept
      status: stable
      tier: process
      source_type: axiomatic
      vault_id: default
      title: Task & Management Domain Pack — operational workflow enforcement
      tags: *a2
      crosslinks: *a3
      created_at: 2026-05-17T10:15:00+07:00
      cluster: implementation_flow
      role: Strategic intent / PRD
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — Task & Management Domain Pack

## Intent

To enforce operational discipline and security over task-related resources (ISSUES, RUNBOOKS, INCIDENTS) within the cognitive_system. This pack ensures that high-priority tasks are protected and that workflow transitions follow established best practices.

## North Star

Every operational atom is automatically categorized by priority and status. The system prevents low-tier agents from making strategic triage decisions and mandates human-in-the-loop (Step-up Auth) for any modifications to urgent production issues.

## Guiding Principles

1. **Workflow Integrity:** Prevent issues from being closed without a clear resolution trail (AUDIT/ADR).
2. **Priority Protection:** Escalate security requirements (MFA/PIN) proportionally to the priority of the task.
3. **Role-Based Triage:** Reserve prioritization and assignment changes for high-tier agents (T2+) or humans.

## Connections

- `[[ADR--TASK-TRACKING-AT-ORCHESTRATOR]]` — context for where volatile task state resides.
- `[[FEAT--ISSUE-TRACKER]]` — the implementation of the core issue tracking system.
