---
id: CONCEPT--AGENT-TIER-ROUTING
phase: 1
type: concept
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: Agent Tier Routing — pick T1/T2/T3 per task based on capability + cost
tags: &a1
  - msp
  - agents
  - dispatch
  - tier-routing
  - cost
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--AGENT-AGNOSTIC
created_at: 2026-05-14T01:30:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--AGENT-TIER-ROUTING
  phase: 1
  type: concept
  status: draft
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Agent Tier Routing — pick T1/T2/T3 per task based on capability + cost
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T01:30:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--AGENT-TIER-ROUTING
    phase: 1
    type: concept
    status: draft
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Agent Tier Routing — pick T1/T2/T3 per task based on capability + cost
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T01:30:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--AGENT-TIER-ROUTING
      phase: 1
      type: concept
      status: draft
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Agent Tier Routing — pick T1/T2/T3 per task based on capability + cost
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T01:30:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Strategic intent / PRD
      attributes:
        domain: concept
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

# CONCEPT — Agent Tier Routing

## Problem

MSP must dispatch a task to **one of three agent tiers** without the caller knowing which CLI exists, which is available, or what each costs:

- **T1 — Qwen CLI** (local SLM, free, weakest)
- **T2 — Gemini CLI** (cloud, ~free under quota, mid-tier capability, very large context)
- **T3 — Claude Code** (cloud, paid, strongest reasoning, smallest context)

A naïve "always use the strongest" routing burns budget on tasks T1 can handle. A naïve "always use T1" produces low-quality output and triggers retry storms. We need a deterministic, auditable routing function.

## Routing inputs

The dispatcher reads four fields from the task:

| Input | Why it matters |
|---|---|
| `task.type` | e.g. `summarize` / `codegen` / `review` — different tiers excel at different work |
| `task.severity` | `critical` (gate to T3) / `regular` / `low` |
| `task.context_size_tokens` | T3's context is smallest; oversized prompts must route to T2 |
| `task.budget_hint` | optional explicit override from the caller |

## Routing function (deterministic)

```
if budget_hint == "T3"        → T3 (caller-forced)
if context_size > 2_000_000   → T2  (only Gemini has the window)
if severity == "critical"     → T3
if task.type ∈ {summarize, classify, format} → T1
else                          → T2 (default cloud tier)
```

## Escalation

A task that fails T1 escalates **once** to T2 (and once more to T3 if `severity≥regular`). After escalation, the result is persisted as a "lesson" atom so the next equivalent task starts at the higher tier — see `[[ADR--AGENT-TIER-COST-POLICY]]`.

## What this concept is NOT

- It's not a load balancer — there is only one instance per tier.
- It's not a fallback chain — escalation is conditional, not automatic-on-failure.
- It's not a model picker — within a tier, the CLI itself picks the underlying model.

## Related

- `[[ADR--AGENT-TIER-COST-POLICY]]` — concrete escalation + budget rules
- `[[BLUEPRINT--AGENT-DISPATCHER]]` — implementation plan
- `[[CONCEPT--AGENT-AGNOSTIC]]` — sibling principle: MSP must work with any agent CLI

## Connections

- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
