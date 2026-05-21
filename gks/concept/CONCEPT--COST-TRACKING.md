---
id: CONCEPT--COST-TRACKING
phase: 1
type: concept
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: Cost Tracking — per-tier price estimates + USAGE bucket aggregation
tags: &a1
  - msp
  - agents
  - cost
  - usage
  - observability
crosslinks: &a2
  references:
    - CONCEPT--AGENT-TIER-ROUTING
    - ADR--AGENT-TIER-COST-POLICY
    - SPEC--USAGE-ATOM
    - SPEC--EPISODE-ATOM
created_at: 2026-05-14T03:44:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--COST-TRACKING
  phase: 1
  type: concept
  status: draft
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Cost Tracking — per-tier price estimates + USAGE bucket aggregation
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T03:44:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--COST-TRACKING
    phase: 1
    type: concept
    status: draft
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Cost Tracking — per-tier price estimates + USAGE bucket aggregation
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T03:44:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--COST-TRACKING
      phase: 1
      type: concept
      status: draft
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Cost Tracking — per-tier price estimates + USAGE bucket aggregation
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T03:44:00.000+07:00
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

# CONCEPT — Cost Tracking

## Problem

`DispatchResult.cost_usd` is part of the dispatcher's public shape (per `[[BLUEPRINT--AGENT-DISPATCHER]]`) but nothing populates it. Tier prices differ by **two orders of magnitude** (T1 is free, T2 is sub-cent, T3 is dollars per million tokens). Without per-call cost estimation, three things break:

1. **Budget visibility** — operators cannot answer "what did the last 24h of dispatch cost me?"
2. **Tier-policy enforcement audit** — `[[ADR--AGENT-TIER-COST-POLICY]]` caps T3 to `critical` severity. If the cap leaks, no telemetry surfaces the waste.
3. **Meta-Learning signal** — the dispatcher's reverse-path execution traces are weaker when cost is unrecorded; future routing tweaks have no quantitative baseline to beat.

## Approach

Two layers, deliberately decoupled:

### Layer 1 — pure pricing math (`cost-tracker.ts`)

A small, dependency-free module that turns `(tier, input_tokens, output_tokens)` into a dollar estimate using a hardcoded `PRICING` table. Token counts are themselves estimated from text length (~4 chars/token — the industry-standard rough heuristic) since calling actual tokenizers per dispatch is overkill for ballpark figures.

The values are **directionally accurate, not actuarial**. The goal is "T3 is roughly 40× T2", not "to four decimal places". Pricing snapshot is dated in a code comment so a future maintainer can refresh without spelunking.

### Layer 2 — USAGE-bucket atom (`usage-recorder.ts`)

After each dispatch, append the call's tier + cost to today's `[[USAGE--DAILY]]-<isoDate>` atom. Bucket atoms accumulate over a day; first call of the day creates the file with proper frontmatter, subsequent calls update the body's JSON summary block.

This mirrors the episode-recorder pattern (one atom per logical unit, written best-effort, never fails dispatch) but trades per-call granularity for daily aggregation — readers want "what did today cost?" not "list every call".

## What this is NOT

- **Not billing.** These figures are pre-tax, pre-discount estimates from public pricing pages. They will diverge from the actual invoice.
- **Not a budget enforcer.** Tier caps and escalation are the budget mechanism (see `[[ADR--AGENT-TIER-COST-POLICY]]`). Cost tracking is observation, not control.
- **Not a tokenizer.** We estimate tokens from char-count; tier-specific tokenizers (tiktoken etc.) would be more accurate but introduce native deps and per-tier divergence.

## Related

- `[[SPEC--USAGE-ATOM]]` — frontmatter contract for the daily bucket atom
- `[[BLUEPRINT--COST-TRACKING]]` — implementation plan
- `[[SPEC--EPISODE-ATOM]]` — sibling runtime-atom type; USAGE is its daily-aggregate complement
- `[[ADR--AGENT-TIER-COST-POLICY]]` — the policy this concept makes observable

## Connections

- [[CONCEPT--AGENT-TIER-ROUTING]]
