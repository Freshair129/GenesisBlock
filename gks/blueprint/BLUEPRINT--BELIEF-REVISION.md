---
id: BLUEPRINT--BELIEF-REVISION
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Belief Revision and Contradiction Logic Implementation Plan
tags: [msp, memory, 888, revision, logic, plan]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - CONCEPT--BELIEF-REVISION
    - CONCEPT--EPISTEMIC-STATES
    - FEAT--SEMANTIC-CONTRADICTION-JUDGE
linked_symbols:
  - file: packages/msp/src/validator/contradiction-judge.ts
created_at: 2026-05-20T10:00:00+07:00
---

# BLUEPRINT — Belief Revision

## 1. Goal

Implement the technical logic to detect contradictions between new sessions and established identity beliefs, manage the regression of epistemic states, and ensure the system's long-term worldview remains consistent.

## 2. Implementation Steps

### T1: Revision Trigger (`packages/msp/src/orchestrator/distiller/revision.ts`)

- Implement a hook in the distiller pipeline that runs after Pillar 2 (SUMMARY).
- For each new potential belief or narrative summary, use the `judgeContradiction` engine (from `contradiction-judge.ts`) to check it against existing `BELIEF--*` atoms in the namespace.

### T2: Epistemic State Machine

- Implement the logic to transition `epistemic_state`:
  - `confirmed` + Contradiction → `contested`
  - `contested` + Supporting Evidence → `confirmed`
  - `contested` + Threshold hit (e.g., 3 consecutive contradictions) → `deprecated`
- Store the `challenge_counter` and `history` of evidence in the atom's frontmatter (or a separate revision log).

### T3: Automated Audit Authoring

- When a belief is contested or deprecated, automatically draft an `AUDIT--BELIEF-REVISION` atom.
- Content must include:
  - The original belief.
  - The contradictory evidence (Session/Narrative IDs).
  - The rationale provided by the Machine Judge.
  - Suggested resolution (Keep, Modify, or Delete).

### T4: RRF Integration (Decay/Penalty)

- Update the retrieval orchestrator to apply a score penalty (multiplier of 0.2x) to any atom in the `contested` state.
- Atoms in `deprecated` state are completely excluded from retrieval.

## 3. Verification Plan

### 3.1 Contradiction Simulation

- Create a stable belief: "The agent always uses TypeScript."
- Feed a session where the agent is forced to use Python.
- Verify:
  - `judgeContradiction` flags the conflict.
  - The belief atom is moved to `contested`.
  - An `AUDIT--BELIEF-REVISION` atom is proposed.

### 3.2 Recovery Simulation

- Take a `contested` belief.
- Feed 3 sessions that confirm the original belief.
- Verify the state returns to `confirmed`.
