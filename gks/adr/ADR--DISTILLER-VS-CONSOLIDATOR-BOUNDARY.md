---
id: ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Boundary between the 8-8-8 Distiller and the M7b Consolidator
tags: [msp, memory, architecture, consolidator, distiller]
aliases: [ADR, implementation_flow, Architecture decision record]
cluster: implementation_flow
role: Architecture decision record
crosslinks:
  references:
    - CONCEPT--TIERED-MEMORY-DISTILLATION
created_at: 2026-05-19T12:00:00+07:00
---

# ADR — Distiller vs. Consolidator Boundary

## Context

We have two distinct memory-shaping modules: the **M7b Consolidator** and the **M11 8-8-8 Distiller**. We need to clearly define their boundaries to avoid responsibility overlap and ensure they work together effectively.

## Decision

We decide to sit the **Distiller alongside the Consolidator**, with a clear separation of concerns based on their input, output, and cadence.

### Responsibility Matrix

| Feature | M7b Consolidator | M11 8-8-8 Distiller |
|---|---|---|
| **Primary Goal** | Single-session density and importance scoring. | Cross-session synthesis and narrative accumulation. |
| **Input** | Raw session turns. | Consolidated Session artifacts (output of consolidator). |
| **Output** | `Episode` atoms in GKS. | `Narrative` and `Identity` artifacts/atoms. |
| **Cadence** | Continuous / End-of-session. | Every 8 Sessions (or configurable ratio). |
| **Scope** | Within-session recall. | Cross-session wisdom/identity. |

### Integration Flow

The Distiller is a **consumer** of the Consolidator. It waits for the Consolidator to finish processing a session, reads the resulting `episodic_memory.json`, and adds it to the distillation queue.

## Consequences

### Positive

- **Modular Design:** Each module has a narrow, well-defined scope.
- **Collaborative Strength:** Consolidator handles the "what happened now," while Distiller handles "what does it mean in the long run."

### Negative

- **Pipeline Latency:** The end-to-end memory pipeline becomes longer (Turn → Episode → Narrative → Identity).

## Source

- `SPEC--MEMORY-888`
- `ULTRAPLAN--888-MEMORY-PROTOCOL`
