---
id: GENESIS--IDENTITY-ENGINE
phase: 0
type: genesis
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Identity Engine — passport-bound agent identity resolution
tags: &a1
  - msp
  - knowledge-block
  - identity
  - manifest
manifest_version: 1.0.0
members: &a2
  core:
    cognitive:
      - COGNITIVE--EGO-DEATH-PASSPORT
    algo:
      - ALGO--IDENTITY-RESOLUTION
    runbook:
      - RUNBOOK--IDENTITY-MIGRATION
    concept:
      - CONCEPT--IDENTITY-LAYER
    params:
      - PARAMS--IDENTITY-PROFILE-DEFAULTS
  optional:
    guard:
      - GUARD--IDENTITY-SCHEMA
      - GUARD--PASSPORT-NONNULL
    protocol:
      - PROTOCOL--IDENTITY-API
    stack:
      - STACK--MSP-NODE-RUNTIME
    safety:
      - SAFETY--PII-REDACTION
    mod:
      - MOD--IDENTITY
daci: &a3
  driver: MOD--IDENTITY
  approver:
    - PERSONA--T3-ARCHITECT
  contributor:
    - PERSONA--T2-IMPLEMENTER
  informed:
    - ENTITY--MSP-USERS
crosslinks: &a4
  references:
    - SPEC--GENESIS-BLOCK-MANIFEST
    - MOD--IDENTITY
    - ALGO--IDENTITY-RESOLUTION
    - COGNITIVE--EGO-DEATH-PASSPORT
    - CONCEPT--IDENTITY-LAYER
    - RUNBOOK--IDENTITY-MIGRATION
    - PARAMS--IDENTITY-PROFILE-DEFAULTS
    - GUARD--IDENTITY-SCHEMA
    - GUARD--PASSPORT-NONNULL
    - PROTOCOL--IDENTITY-API
    - STACK--MSP-NODE-RUNTIME
    - SAFETY--PII-REDACTION
created_at: 2026-05-14T21:10:00+07:00
aliases: &a5
  - GENESIS
  - implementation_flow
  - Block Manifest (v2.3+)
cluster: implementation_flow
role: Block Manifest (v2.3+)
attributes:
  id: GENESIS--IDENTITY-ENGINE
  phase: 0
  type: genesis
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Identity Engine — passport-bound agent identity resolution
  tags: *a1
  manifest_version: 1.0.0
  members: *a2
  daci: *a3
  crosslinks: *a4
  created_at: 2026-05-14T21:10:00+07:00
  aliases: *a5
  cluster: implementation_flow
  role: Block Manifest (v2.3+)
  attributes:
    id: GENESIS--IDENTITY-ENGINE
    phase: 0
    type: genesis
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Identity Engine — passport-bound agent identity resolution
    tags: *a1
    manifest_version: 1.0.0
    members: *a2
    daci: *a3
    crosslinks: *a4
    created_at: 2026-05-14T21:10:00+07:00
    aliases: *a5
    cluster: implementation_flow
    role: Block Manifest (v2.3+)
    attributes:
      id: GENESIS--IDENTITY-ENGINE
      phase: 0
      type: genesis
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Identity Engine — passport-bound agent identity resolution
      tags: *a1
      manifest_version: 1.0.0
      members: *a2
      daci: *a3
      crosslinks: *a4
      created_at: 2026-05-14T21:10:00+07:00
      aliases: *a5
      cluster: implementation_flow
      role: Block Manifest (v2.3+)
      attributes:
        domain: genesis
      domain: genesis
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: genesis
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: genesis
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# GENESIS — Identity Engine

The **Identity Engine** is the first canonical Genesis Block in the MSP ecosystem. It aggregates the five essential dimensions required to define and maintain a stable agent identity ("soul") across heterogeneous sessions and providers.

## Behavioral Objective

When loaded, this block instructs the agent to:

1. **Acknowledge** its identity as an externalized artifact (the passport).
2. **Surrender** pre-existing personas (Ego Death).
3. **Resolve** identity conflicts by favoring project-level overrides over global baselines.
4. **Enforce** structural integrity of its own soul (Schema Guard).

## Governance

Decisions regarding the Identity Engine are driven by `[[MOD--IDENTITY]]`. High-level architectural changes require approval from the `[[PERSONA--T3-ARCHITECT]]`.

## Connections

- [[COGNITIVE--EGO-DEATH-PASSPORT]]
- [[ALGO--IDENTITY-RESOLUTION]]
- [[RUNBOOK--IDENTITY-MIGRATION]]
- [[CONCEPT--IDENTITY-LAYER]]
- [[PARAMS--IDENTITY-PROFILE-DEFAULTS]]
- [[GUARD--IDENTITY-SCHEMA]]
- [[GUARD--PASSPORT-NONNULL]]
- [[PROTOCOL--IDENTITY-API]]
- [[STACK--MSP-NODE-RUNTIME]]
- [[SAFETY--PII-REDACTION]]
- \[\[PERSONA--T2-IMPLEMENTER\]\]
- \[\[ENTITY--MSP-USERS\]\]
- [[SPEC--GENESIS-BLOCK-MANIFEST]]
