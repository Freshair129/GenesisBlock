---
id: CONCEPT--ROOT-CAUSE-ANALYSIS
phase: 1
type: concept
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Root cause analysis — confirm origin before any fix
tags:
  - msp
  - governance
  - rca
  - debug-discipline
  - foundation
crosslinks:
  references:
    - FRAMEWORK--KNOWLEDGE-3-TIER
created_at: 2026-05-17T02:00:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  domain: concept
---

# CONCEPT — Root Cause Analysis

## Why this concept exists

The cheapest patches are often the most expensive. Fixing a symptom without understanding the underlying cause leads to recurrence, brittle workarounds, and cascading failures across unrelated subsystems.

RCA discipline forces a pause: before any code, decision, or directive change, identify the deepest layer at which the problem originates. This concept exists to codify that discipline as a Genesis atom suitable for Master promotion.

## Definition

Root cause analysis is a structured pass with four named outputs:

| Output | Definition |
|---|---|
| **Symptom** | The observable behaviour, error message, or failure mode — stated without interpretation. |
| **Proximate cause** | The mechanism immediately producing the symptom. |
| **Root cause** | The deepest contributing factor that, if removed, prevents recurrence. |
| **Fix scope** | The chosen target — root, proximate, or symptom — selected explicitly with rationale. |

A fix that targets a lower scope than the root is valid only when paired with a deliberate documented reason (cost, blast radius, time pressure).

## When to apply

- Any bug report or runtime error
- An ambiguous user request that does not match the obvious solution shape
- A previous attempt that failed (recursive RCA — apply to the failure itself)
- Observed behaviour that contradicts a known invariant

## How

1. State the symptom precisely. No interpretation. No diagnosis-by-implication.
2. Walk the causal chain backward. What triggered this? And what triggered that?
3. Stop when one of two conditions holds:
   - Removing the cause would prevent recurrence (root)
   - The chain reaches a stable boundary outside the system's control
4. Choose fix scope explicitly and record the choice.
5. If the root cannot be identified with the available evidence, say so and propose targeted investigation — never substitute a guess.

## What this concept does NOT prescribe

- Specific RCA techniques (5 whys, fishbone, fault tree). Authors choose the technique appropriate to the domain.
- Mandatory documentation for trivial fixes. The discipline is mental; the artifact is optional.
- A required time budget. RCA depth scales with blast radius — a one-line bug fix needs seconds; a recurring incident needs hours.

## Source

- CLAUDE.md § "MASTER BLOCK: ROOT CAUSE ANALYSIS MANDATE" — the original narrative form
- `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` — the promotion target tier

## Connections

- [[FRAMEWORK--KNOWLEDGE-3-TIER]]
