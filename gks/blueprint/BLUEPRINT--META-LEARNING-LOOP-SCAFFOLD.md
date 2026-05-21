---
id: BLUEPRINT--META-LEARNING-LOOP-SCAFFOLD
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Meta Learning Loop (MLL) Implementation Scaffold
tags: [msp, mll, learning, automation, plan]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - FEAT--META-LEARNING-LOOP
    - SPEC--META-LEARNING-LOOP
linked_symbols:
  - file: packages/msp/src/orchestrator/mll/orchestrator.ts
  - file: packages/msp/src/orchestrator/mll/stability.ts
  - file: packages/msp/src/orchestrator/mll/skill-creator.ts
created_at: 2026-05-21T10:00:00+07:00
---

# BLUEPRINT — Meta Learning Loop (MLL) Scaffold

## 1. Goal

Implement the core machinery for the Meta Learning Loop (MLL), starting with the automated distillation of reusable skills from successful interaction traces.

## 2. Implementation Steps

### T1: Distiller Integration (`packages/msp/src/orchestrator/mll/orchestrator.ts`)

- Implement the top-level `runMll(sessionId)` function.
- This function should be called at the end of every successful session.
- Logic:
    1. Detect task success (Acceptance tests green).
    2. Analyze complexity (Number of turns/tool calls).
    3. If threshold met, trigger `distillSkillFromEpisodes`.

### T2: Multi-Model Stability Check (`packages/msp/src/orchestrator/mll/stability.ts`)

- Implement `calculateStabilityScore(content: string)`.
- Logic:
    1. Parallel call to 3 SLM providers (mocking if not available).
    2. Ask each: "Summarize this skill in 3 words."
    3. Compare results using simple string similarity or LLM-judge.
    4. Return score (0.0 to 1.0).

### T3: Skill Candidate Refinement

- Update `skill-creator.ts` to include the stability score in the frontmatter.
- Ensure the produced atom uses the Universal Phase-Tailed Standard ID.
- Automatically add relevant `crosslinks.references` to the source episodes.

### T4: Tension Detection (MVP)

- Implement a basic scanner that checks if a `SKILL--*` atom's workflow matches a recent `BLUEPRINT--*`.
- Emit a `TENSION--*` log if significant drift is detected.

## 3. Verification Plan

### 3.1 Synthetic Learning Run

- Create a synthetic session log with a complex successful workflow.
- Run `runMll(syntheticSessionId)`.
- Verify:
  - A new `SKILL--*` atom appears in the `candidates/` folder.
  - Frontmatter contains a `stability_score`.
  - `linked_symbols` point back to the trace files.

### 3.2 Stability Check Test

- Feed the stability check a well-defined skill vs. a vague/conflicting one.
- Verify the well-defined one receives a higher score.
