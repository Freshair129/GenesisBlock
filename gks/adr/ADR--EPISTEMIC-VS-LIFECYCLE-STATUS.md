---
id: ADR--EPISTEMIC-VS-LIFECYCLE-STATUS
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Separate epistemic_state from document status
tags: [msp, memory, epistemology, lifecycle, governance]
aliases: [ADR, implementation_flow, Architecture decision record]
cluster: implementation_flow
role: Architecture decision record
crosslinks:
  references:
    - CONCEPT--EPISTEMIC-STATES
    - CONCEPT--TAXONOMY-V2-3
created_at: 2026-05-19T11:30:00+07:00
---

# ADR — Separate epistemic_state from document status

## Context

We need to track both the **lifecycle** of a document (is it a draft? is it stable?) and the **confidence** in the facts it contains (is it unverified? is it contested?). Using the existing `status` field for both purposes would lead to overloaded meanings and prevent knowledge from regressing in confidence without losing its structural authority.

## Decision

We decide to introduce a new frontmatter field **`epistemic_state`** that is strictly decoupled from the existing **`status`** field.

### Field Definitions

| Field | Purpose | Authority | Property |
|---|---|---|---|
| **`status`** | Document lifecycle (`draft` → `stable`) | Human / Gatekeeper | Monotonic (No regression) |
| **`epistemic_state`** | Factual confidence (`hypothesis` → `contested`) | Distiller / Logic | Non-Monotonic (Can regress) |

### Validator Integration

The `msp:validate` suite will be updated to:

1. Enforce that `epistemic_state` is one of the allowed values: `hypothesis`, `confirmed`, `contested`, `deprecated`.
2. Allow `epistemic_state` to change in any direction in a PR (non-monotonic).
3. Continue to enforce monotonicity for the `status` field.

## Consequences

### Positive

- **Granularity:** Allows for stable documents that contain currently contested information.
- **Auto-Correction:** Enables the distiller to automatically flag beliefs for review without manually changing document statuses.

### Negative

- **Metadata Load:** Increases the number of mandatory fields in memory-related atoms.

## Source

- `CONCEPT--EPISTEMIC-STATES`
- `SPEC--MEMORY-888`
- `ULTRAPLAN--888-MEMORY-PROTOCOL`
