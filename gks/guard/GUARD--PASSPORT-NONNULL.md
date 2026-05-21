---
id: GUARD--PASSPORT-NONNULL
phase: 2
type: guard
status: stable
tier: safety
source_type: axiomatic
vault_id: default
title: Passport Non-Null Guard — ensure mandatory identity presence
tags: &a1
  - msp
  - guard
  - integrity
  - passport
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
created_at: 2026-05-14T20:50:00+07:00
aliases: &a3
  - GUARD
  - agent_governance
  - Structural / behavioural guardrail
cluster: agent_governance
role: Structural / behavioural guardrail
attributes:
  id: GUARD--PASSPORT-NONNULL
  phase: 2
  type: guard
  status: stable
  tier: safety
  source_type: axiomatic
  vault_id: default
  title: Passport Non-Null Guard — ensure mandatory identity presence
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T20:50:00+07:00
  aliases: *a3
  cluster: agent_governance
  role: Structural / behavioural guardrail
  attributes:
    id: GUARD--PASSPORT-NONNULL
    phase: 2
    type: guard
    status: stable
    tier: safety
    source_type: axiomatic
    vault_id: default
    title: Passport Non-Null Guard — ensure mandatory identity presence
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T20:50:00+07:00
    aliases: *a3
    cluster: agent_governance
    role: Structural / behavioural guardrail
    attributes:
      id: GUARD--PASSPORT-NONNULL
      phase: 2
      type: guard
      status: stable
      tier: safety
      source_type: axiomatic
      vault_id: default
      title: Passport Non-Null Guard — ensure mandatory identity presence
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T20:50:00+07:00
      aliases: *a3
      cluster: agent_governance
      role: Structural / behavioural guardrail
      attributes:
        domain: guard
      domain: guard
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: guard
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: guard
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# GUARD — Passport Non-Null

## Invariant

An active agent session MUST always carry a valid, non-null passport containing:

- `identity`: Derived via `getIdentity(ns)`.
- `memory`: Access to the `MemoryStore`.
- `soul`: Behavioral guidance from the `voice` config.

## Enforcement

The `CognitiveLayer` facade ensures that if a passport cannot be resolved (missing or malformed `identity.json` with no fallback), the session fails to initialize.
An anonymous user kind is provided as a safe default, but the passport structure itself remains non-null.

## Connections

- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
