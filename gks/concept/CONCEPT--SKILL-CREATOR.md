---
id: CONCEPT--SKILL-CREATOR
phase: 1
type: concept
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: "Skill Creator — automated distillation of execution history into reusable skills"
tags:
  - msp
  - mll
  - skill
  - learning
  - evolution
crosslinks:
  references:
    - SPEC--META-LEARNING-LOOP
    - CONCEPT--ATOM-CONTRADICTION-DETECTION
created_at: 2026-05-18T11:15:00+07:00
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — Skill Creator

## Intent

To enable the cognitive_system to autonomously evolve its capabilities by distilling successful execution patterns into reusable `SKILL--` atoms. This "Meta-Learning" process converts transient experience (EPISODES) into permanent, searchable knowledge.

## North Star

The system identifies recurring successful workflows and automatically proposes them as structured skills. Agents can recall these skills in future tasks to reduce reasoning overhead, minimize token usage, and ensure consistent high-quality outcomes.

## Guiding Principles

1. **Evidence-Based Learning:** Skills must be derived from actual successful execution traces (EPISODES).
2. **Stability via Consensus:** Proposed skills undergo a multi-model stability check to ensure the definition is robust and accurate.
3. **Continuous Refinement:** Skills are not static; they are updated or superseded as new, more efficient patterns emerge.

## Connections

- `[[SPEC--META-LEARNING-LOOP]]` — the technical specification for the Meta-Learning Loop.
- `[[CONCEPT--SKILL-CANDIDATE]]` — the schema for the resulting atoms.
