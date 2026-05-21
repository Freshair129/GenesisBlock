---
id: ADR--AFFECT-POLICY-DEFAULT
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Default affect policy for memory distillation
tags: [msp, memory, affect, 888, policy]
aliases: [ADR, implementation_flow, Architecture decision record]
cluster: implementation_flow
role: Architecture decision record
crosslinks:
  references:
    - CONCEPT--TIERED-MEMORY-DISTILLATION
created_at: 2026-05-19T12:30:00+07:00
---

# ADR — Default Affect Policy

## Context

The 8-8-8 protocol from EVA includes a rich "affect" model (tracking stress, warmth, qualia, etc.). However, EVA's shipped implementation uses a lean schema focused on narrative and concept clusters. We need to decide how much of this affect model to adopt in `cognitive_system`.

## Decision

We decide to adopt a **"Spartan Default"** for the affect policy, prioritizing factual and narrative coherence over emotional modeling.

### Policy Tiers

| Tier | Name | Behavior |
|---|---|---|
| **a** | **Spartan (Default)** | No affect fields. Stays purely narrative/factual. |
| **b** | LLM-Scored | Optional opt-in. Distiller asks LLM to infer `engagement`, `urgency`, and `confidence`. |
| **c** | Sensor-Backed | Forbidden. Sourced from hypothetical bio-telemetry. |

### Rationale

We prioritize accuracy and reliability. Emotional inference via LLM (Tier b) can be subjective and may introduce noise into the distilled wisdom. Tier (a) represents the most stable and verifiable configuration.

## Consequences

### Positive

- **Reduced Token Cost:** Summarization prompts are shorter and more focused.
- **Reliability:** Prevents the system from "hallucinating" emotional states that aren't grounded in interaction data.

### Negative

- **Lower Empathy:** The system loses the ability to track the "feeling" of interaction cycles (e.g., "The boss seemed frustrated this week").

## Source

- `SPEC--MEMORY-888`
- `ULTRAPLAN--888-MEMORY-PROTOCOL`
