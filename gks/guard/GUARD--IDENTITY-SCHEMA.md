---
id: GUARD--IDENTITY-SCHEMA
phase: 2
type: guard
status: stable
tier: safety
source_type: axiomatic
vault_id: default
title: Identity Schema Guard — structural invariants for identity.json
tags:
  - msp
  - guard
  - schema
  - validation
  - identity
crosslinks: {"references":["CONCEPT--IDENTITY-LAYER"]}
created_at: 2026-05-14T20:40:00+07:00
aliases:
  - GUARD
  - agent_governance
  - Structural / behavioural guardrail
cluster: agent_governance
role: "Structural / behavioural guardrail"
---

# GUARD — Identity Schema

## Invariants
Every `identity.json` file MUST satisfy these structural invariants:

1. **Schema Version**: `schemaVersion` must be exactly `1`.
2. **Profile Core**: `profile.name` and `profile.role` must be non-empty strings.
3. **Voice Enums**: 
   - `voice.formality` must be one of `[casual, neutral, formal]`.
   - `voice.responseCadence` must be one of `[terse, standard, verbose]`.
4. **Preference Validity**: Every key in `preferences` must follow the `{ value: any, expiresAt: timestamp | null }` pattern.

## Enforcement
Mechanically enforced by Zod in `src/identity/types.ts` at write-time.
Any violation prevents the file from being committed to disk.

## Connections
- [[CONCEPT--IDENTITY-LAYER]]

