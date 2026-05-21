---
id: ADR--DISTILLER-VS-SKILL-CREATOR-BOUNDARY
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Boundary between the 8-8-8 Distiller and the MLL Skill Creator
tags: [msp, memory, architecture, skill-creator, distiller]
aliases: [ADR, implementation_flow, Architecture decision record]
cluster: implementation_flow
role: Architecture decision record
crosslinks:
  references:
    - CONCEPT--TIERED-MEMORY-DISTILLATION
created_at: 2026-05-19T12:15:00+07:00
---

# ADR — Distiller vs. Skill Creator Boundary

## Context

The Memory Lifecycle Layer (`PRD--MLL`) defines a **Skill Creator** that extracts procedural patterns from agent behavior. The 8-8-8 protocol also extracts "behavioral patterns." We need to avoid duplicate extraction logic and define which module owns which type of behavioral memory.

## Decision

We decide that the **Skill Creator is responsible for procedural patterns**, while the **8-8-8 Distiller is responsible for narrative and factual patterns**.

### Responsibility Matrix

| Memory Class | Description | Primary Module | Output |
|---|---|---|---|
| **Procedural** | "How-to" patterns; tool-use sequences; repeatable skills. | **Skill Creator** | `SKILL--*` atoms |
| **Narrative** | Interaction arcs; recurring themes; factual distilled beliefs. | **8-8-8 Distiller** | `NARRATIVE--*` / `IDENTITY--*` |
| **Factual** | Specific distilled truths about the project/user. | **8-8-8 Distiller** | `BELIEF--*` / `FACT--*` |

### Interaction

If the 8-8-8 Distiller identifies a recurring procedural behavior (e.g., "The agent always uses grep before editing"), it notes it as a **theme** in the Narrative/Identity artifact but does NOT attempt to author a `SKILL--*` atom. It may optionally signal the Skill Creator to investigate.

## Consequences

### Positive

- **Extraction Specialization:** Allows each extractor to use optimized prompts (procedural vs. narrative).
- **Reduced Bloat:** Prevents the same pattern from being stored in two different atom types.

### Negative

- **Dependency:** The full "behavioral DNA" of a system is split across two subsystems.

## Source

- `SPEC--MEMORY-888`
- `ULTRAPLAN--888-MEMORY-PROTOCOL`
