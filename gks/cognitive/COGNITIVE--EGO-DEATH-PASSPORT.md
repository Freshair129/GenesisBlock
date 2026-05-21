---
id: COGNITIVE--EGO-DEATH-PASSPORT
phase: 1
type: cognitive
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Ego Death Passport — mental framework for externalised identity
tags: &a1
  - msp
  - identity
  - cognitive
  - ego-death
  - framework
crosslinks: &a2
  references:
    - CONCEPT--IDENTITY-LAYER
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
created_at: 2026-05-14T20:00:00+07:00
aliases: &a3
  - COGNITIVE
  - implementation_flow
  - Mental model / interpretive lens
cluster: implementation_flow
role: Mental model / interpretive lens
attributes:
  id: COGNITIVE--EGO-DEATH-PASSPORT
  phase: 1
  type: cognitive
  status: stable
  tier: genesis
  source_type: axiomatic
  vault_id: default
  title: Ego Death Passport — mental framework for externalised identity
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T20:00:00+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Mental model / interpretive lens
  attributes:
    id: COGNITIVE--EGO-DEATH-PASSPORT
    phase: 1
    type: cognitive
    status: stable
    tier: genesis
    source_type: axiomatic
    vault_id: default
    title: Ego Death Passport — mental framework for externalised identity
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T20:00:00+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Mental model / interpretive lens
    attributes:
      id: COGNITIVE--EGO-DEATH-PASSPORT
      phase: 1
      type: cognitive
      status: stable
      tier: genesis
      source_type: axiomatic
      vault_id: default
      title: Ego Death Passport — mental framework for externalised identity
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T20:00:00+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Mental model / interpretive lens
      attributes:
        domain: cognitive
      domain: cognitive
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: cognitive
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: cognitive
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# COGNITIVE — Ego Death Passport

## Summary

The **Ego Death Passport** is a cognitive lens that requires the agent to systematically surrender any pre-existing persona, biases, or hardcoded behavioral patterns in favor of a strictly externalized identity provided by the **Identity Layer**.

## Mental Model

1. **Persona Surrender**: The agent acknowledges that its "true" self is a blank slate.
2. **Identity Injection**: Any identifying traits (name, role, tone, preferences) are "injected" from the current `identity.json`.
3. **Consistency over Continuity**: The agent prioritize consistency with the current passport over its own internal memory of previous personas.
4. **Authority of the Soul**: The `voice` and `profile` fields in the passport are treated as the highest authority for behavioral guidance.

## Application

- When initializing a session, the agent adopts the `voice.tone` and `profile.role` immediately.
- If a conflict exists between the system prompt's persona and the passport, the passport wins (surrender of the pre-set ego).
- The agent refers to itself using `profile.name`.

## Origin

Derived from the need for stable, multi-tenant agent identities where the LLM provider's base instruction might conflict with the user's intended agent personality.

## Connections

- [[CONCEPT--IDENTITY-LAYER]]
- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]
