---
id: ADR--MEMORY-WRITE-AUTHORITY
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — MSP Distiller as the exclusive write authority for memory artifacts
tags: [msp, memory, authority, distillation, governance]
aliases: [ADR, implementation_flow, Architecture decision record]
cluster: implementation_flow
role: Architecture decision record
crosslinks:
  references:
    - SPEC--MEMORY-888
    - CONCEPT--TIERED-MEMORY-DISTILLATION
created_at: 2026-05-19T11:45:00+07:00
---

# ADR — Exclusive Memory Write Authority

## Context

We need to ensure the integrity of the distilled memory hierarchy (Sessions, Narratives, Identities). If multiple processes or the LLM itself were allowed to author these artifacts directly, it could lead to inconsistent narratives, circular evidence, and the loss of epistemic traceability.

## Decision

We decide that the **MSP Distiller process** (the "subconscious") is the **exclusive write authority** for all distilled memory artifacts.

### Authority Rules

1. **No LLM Direct Write:** The LLM agents (Consciousness layer) are forbidden from writing to `.brain/msp/projects/<ns>/memory/` paths. Any attempt to do so must be blocked by MSP file-system guards.
2. **Centralized Synthesis:** Only the canonical MSP distiller process can produce `Session`, `Narrative`, and `Identity` artifacts.
3. **Atomic Output:** The distiller authors memory artifacts and then emits corresponding atoms into GKS via the standard gatekeeper path. User-facing tools like `retain()` do NOT target memory storage paths.

## Consequences

### Positive

- **Integrity:** Guarantees that all long-term memory is produced via the validated 4-pillars pipeline.
- **Security:** Prevents agents from "hallucinating" or forging their own long-term beliefs.
- **Coherence:** Ensures a single, unified narrative arc for the namespace.

### Negative

- **Latency:** Memory updates are asynchronous and occur between sessions, meaning the system doesn't "remember" across sessions in real-time.

## Source

- `SPEC--MEMORY-888`
- `ULTRAPLAN--888-MEMORY-PROTOCOL`
