---
id: MASTER--ROOT-CAUSE-ANALYSIS
phase: 0
type: master
status: stable
tier: master
source_type: axiomatic
promoted_from: CONCEPT--ROOT-CAUSE-ANALYSIS
promoted_at: 2026-05-17T02:10:00.000+07:00
promotion_adr: ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS
vault_id: default
priority: P0
constituents: &a1
  required:
    framework:
      - FRAMEWORK--KNOWLEDGE-3-TIER
    concept:
      - CONCEPT--ROOT-CAUSE-ANALYSIS
    adr:
      - ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS
  optional: {}
title: Root cause analysis — confirm origin before any fix
tags: &a2
  - msp
  - master
  - rca
  - governance
  - instinct
crosslinks: &a3
  references:
    - FRAMEWORK--KNOWLEDGE-3-TIER
    - CONCEPT--ROOT-CAUSE-ANALYSIS
    - ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS
created_at: 2026-05-17T02:10:00.000+07:00
aliases: &a4
  - MASTER
  - implementation_flow
  - Root-level policy / genesis rule
cluster: implementation_flow
role: Root-level policy / genesis rule
attributes:
  id: MASTER--ROOT-CAUSE-ANALYSIS
  phase: 0
  type: master
  status: stable
  tier: master
  source_type: axiomatic
  promoted_from: CONCEPT--ROOT-CAUSE-ANALYSIS
  promoted_at: 2026-05-17T02:10:00.000+07:00
  promotion_adr: ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS
  vault_id: default
  priority: P0
  constituents: *a1
  title: Root cause analysis — confirm origin before any fix
  tags: *a2
  crosslinks: *a3
  created_at: 2026-05-17T02:10:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Root-level policy / genesis rule
  attributes:
    id: MASTER--ROOT-CAUSE-ANALYSIS
    phase: 0
    type: master
    status: stable
    tier: master
    source_type: axiomatic
    promoted_from: CONCEPT--ROOT-CAUSE-ANALYSIS
    promoted_at: 2026-05-17T02:10:00.000+07:00
    promotion_adr: ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS
    vault_id: default
    priority: P0
    constituents: *a1
    title: Root cause analysis — confirm origin before any fix
    tags: *a2
    crosslinks: *a3
    created_at: 2026-05-17T02:10:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Root-level policy / genesis rule
    attributes:
      domain: master
    domain: master
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: master
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# MASTER — Root Cause Analysis

## Intent

Before proposing any fix, identify and confirm the root cause. Reactive patches that target symptoms introduce new bugs, mask underlying issues, and cascade.

## Why

Bugs and ambiguous requests have layers. Removing the visible symptom without understanding the deeper cause guarantees recurrence — often in a worse form. RCA is the cheapest checkpoint because rationale is freshest at the moment of analysis, while context is fully loaded.

## Directives

1. State the observable symptom precisely before interpreting it. No diagnosis-by-implication.
2. Trace the causal chain backward until removing the cause would prevent recurrence, or the chain reaches a stable boundary outside the system's control.
3. Document the chosen fix scope explicitly: root, proximate, or symptom-only — and the rationale for the choice.
4. If the root cause cannot be identified with available evidence, say so and propose targeted investigation. Never substitute a guess.
5. Reject the urge to "just make it work" before understanding why it did not.

## Apply when

Any bug report, runtime error, ambiguous user request that does not match the obvious solution shape, failed previous attempt, or behaviour that contradicts a known invariant.

## Conflicts with

(none currently — flag any future Master that allows symptomatic patching without RCA.)

## Connections

- [[CONCEPT--ROOT-CAUSE-ANALYSIS]]
- [[ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS]]
- [[FRAMEWORK--KNOWLEDGE-3-TIER]]
