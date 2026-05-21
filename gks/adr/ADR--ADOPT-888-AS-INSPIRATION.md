---
id: ADR--ADOPT-888-AS-INSPIRATION
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Adopt 8-8-8 architecture with cognitive_system-native naming
tags: [msp, memory, architecture, 888, naming]
aliases: [ADR, implementation_flow, Architecture decision record]
cluster: implementation_flow
role: Architecture decision record
crosslinks:
  references:
    - CONCEPT--TIERED-MEMORY-DISTILLATION
    - FRAMEWORK--MSP-ARCHITECTURE-V2
created_at: 2026-05-19T11:00:00+07:00
---

# ADR — Adopt 8-8-8 architecture with native naming

## Context

We need a structured way to accumulate cross-session knowledge and distill it into long-term beliefs. The **8-8-8 Memory Synthesis Protocol** from EVA v9.6.2 provides a proven conceptual framework for this (Hierarchical Memory Compression). However, a literal port of EVA's terminology (Consciousness, Session, Core, Sphere) might conflict with or confuse users of the existing `cognitive_system` (GKS/MSP) vocabulary.

## Decision

We decide to adopt the **architecture and philosophy** of the 8-8-8 protocol while translating the names into `cognitive_system`-native terms to ensure consistency with the existing codebase.

### Terminology Mapping

| EVA Term | `cognitive_system` Term | Rationale |
|---|---|---|
| Consciousness | **Active Context** | Clearer technical meaning; non-persistent. |
| Session Memory | **Session** | Matches existing `memory/sessions/` usage. |
| Core Memory | **Narrative** | Describes the "arc" and "story" of multiple sessions. |
| Sphere Memory | **Identity** | Describes the durable, behavioral DNA of the namespace. |

### Canonical Aliases

Every memory-related atom will carry the EVA names as aliases in its frontmatter to preserve discoverability for users familiar with both systems.

## Consequences

### Positive

- **Consistency:** Aligns with existing GKS/MSP terminology.
- **Clarity:** Uses more descriptive, less metaphorical names for technical components.
- **Architectural Parity:** Retains the powerful 8:1 distillation ratio and 4-pillar pipeline.

### Negative

- **Translation Layer:** Requires a mental mapping for developers moving between EVA and cognitive_system.

## Alternatives considered

- **Literal Port:** Using Consciousness/Session/Core/Sphere. *Rejected* to avoid terminology fragmentation.
- **Custom Ratios:** Using 10:1 or 5:1. *Rejected* — the 8:1 ratio is a key part of the inspired protocol's identity and selectivity.

## Source

- `ULTRAPLAN--888-MEMORY-PROTOCOL`
- `SPEC--MEMORY-888`
