---
id: ADR--AGENT-TIER-COST-POLICY
phase: 2
type: adr
status: draft
tier: genesis
source_type: axiomatic
vault_id: default
title: ADR — Agent Tier Cost Policy — escalation and budget rules
tags: &a1
  - msp
  - agents
  - cost
  - governance
crosslinks: &a2
  references:
    - CONCEPT--AGENT-TIER-ROUTING
created_at: 2026-05-13T22:52:00+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--AGENT-TIER-COST-POLICY
  phase: 2
  type: adr
  status: draft
  tier: genesis
  source_type: axiomatic
  vault_id: default
  title: ADR — Agent Tier Cost Policy — escalation and budget rules
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T22:52:00+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--AGENT-TIER-COST-POLICY
    phase: 2
    type: adr
    status: draft
    tier: genesis
    source_type: axiomatic
    vault_id: default
    title: ADR — Agent Tier Cost Policy — escalation and budget rules
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-13T22:52:00+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      domain: adr
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# ADR — Agent Tier Cost Policy

## Context

Using cloud LLMs (T2/T3) incurs costs. We need a clear policy for when to escalate from local T1 to cloud tiers.

## Decision

1.  **Automatic Escalation**: If T1 fails twice on the same task (acceptance test fail), escalate to T2.
2.  **Manual Escalation**: Users can explicitly request T2/T3 for specific tasks.
3.  **Tier Cap**: 
    - T2 (Gemini Flash) is allowed for all regular feature work.
    - T3 (Claude) is restricted to `critical` severity tasks or explicitly approved sessions.
4.  **Token Budget**: Tasks exceeding 2M tokens context must use T2 (Gemini).

## Consequences

- Improved success rate for complex tasks.
- Controlled cloud spend.
- Clear path for agent self-improvement (if T1 fails, T2 fixes and T1 learns).

## Connections
- [[CONCEPT--AGENT-TIER-ROUTING]]

