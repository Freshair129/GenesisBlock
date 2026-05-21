---
id: ADR--DISTILLATION-RATIO-CONFIGURABLE
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Distillation ratios are configurable defaults
tags: [msp, memory, config, 888, distillation]
aliases: [ADR, implementation_flow, Architecture decision record]
cluster: implementation_flow
role: Architecture decision record
crosslinks:
  references:
    - CONCEPT--TIERED-MEMORY-DISTILLATION
created_at: 2026-05-19T11:15:00+07:00
---

# ADR — Distillation ratios are configurable defaults

## Context

The 8-8-8 protocol suggests a fixed 8:1 ratio for memory distillation (8 Sessions → 1 Core, 8 Cores → 1 Sphere). While this provides a strong conceptual anchor, the optimal ratio for different projects or agent types might vary based on session frequency and content density.

## Decision

We decide to make the distillation ratios **configurable defaults** rather than hardcoded constants.

### Default Values

- `distillation.episodes_per_narrative`: **8**
- `distillation.narratives_per_identity`: **8**

### Validator Rules

The validator will allow a range of `{2..32}` for these ratios. Setting a ratio outside this range requires an explicit `phase_override`.

## Consequences

### Positive

- **Flexibility:** Allows different namespaces to optimize their memory accumulation based on real usage patterns.
- **Traceability:** Changes to these ratios are recorded in the central configuration.

### Negative

- **Drift Risk:** If ratios are set too low, memory might become volatile; if too high, identity promotion might take too long.

## Source

- `SPEC--MEMORY-888`
- `ULTRAPLAN--888-MEMORY-PROTOCOL`
