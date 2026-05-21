---
id: CONCEPT--BELIEF-REVISION
phase: 1
type: concept
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Belief Revision — protocol for handling contradictions and state regressions
aliases: [knowledge correction, belief downgrade]
cluster: memory
role: Strategic intent / PRD
tags: [msp, memory, logic, correction, reliability]
crosslinks:
  references:
    - CONCEPT--EPISTEMIC-STATES
    - CONCEPT--TIERED-MEMORY-DISTILLATION
created_at: 2026-05-19T10:45:00+07:00
---

# CONCEPT — Belief Revision

## Intent

To provide a robust protocol for handling **contradictory evidence** and revising established beliefs. This ensures that the system can correct its own mistakes and adapt its "worldview" when faced with new, conflicting data.

## Philosophy

A healthy memory system must be able to change its mind. If a long-term belief (Tier 3 Identity) is consistently challenged by new session data (Tier 1), the system must proactively downgrade that belief, move it to a `contested` state, and eventually `deprecated` or reformulate it.

## Revision Workflow

1. **Conflict Detection:** The distiller identifies a contradiction between a high-tier belief and a new session artifact.
2. **Contestation:** The belief's epistemic state is regressed from `confirmed` to `contested`. A `challenge_counter` is initialized.
3. **Recovery Window:** The system observes subsequent sessions.
    - If supporting evidence returns → state moves back to `confirmed`.
    - If contradictions continue (threshold hit) → belief is **downgraded**.
4. **Downgrade:** The belief is removed from the Identity tier and converted back into a lower-tier Narrative for re-distillation or audit.

## Guiding Principles

1. **Stability over Volatility:** High-tier beliefs require *multiple* contradictions to trigger a downgrade, preventing knee-jerk reactions to single outliers.
2. **Auditability:** Every belief revision must produce an audit artifact (AUDIT--BELIEF-REVISION) explaining the "why" behind the change.
3. **Identity Preservation:** Fundamental identity beliefs (especially in the `safety` domain) have the highest resistance to revision.

## Connections

- `[[CONCEPT--EPISTEMIC-STATES]]` — defines the states that this protocol manipulates.
- `[[SPEC--MEMORY-888]]` — implements the state machine for revision logic.
