---
id: ALGO--IDENTITY-RESOLUTION
phase: 2
type: algo
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Identity Resolution Algorithm — Global vs Project Layering
tags: &a1
  - msp
  - identity
  - resolution
  - merging
  - layering
crosslinks: &a2
  references:
    - CONCEPT--IDENTITY-LAYER
    - ADR--GLOBAL-VS-WORKSPACE
created_at: 2026-05-11T10:28:00.000Z
aliases: &a3
  - ALGO
  - implementation_flow
  - Algorithm definition
cluster: implementation_flow
role: Algorithm definition
attributes:
  id: ALGO--IDENTITY-RESOLUTION
  phase: 2
  type: algo
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Identity Resolution Algorithm — Global vs Project Layering
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-11T10:28:00.000Z
  aliases: *a3
  cluster: implementation_flow
  role: Algorithm definition
  attributes:
    id: ALGO--IDENTITY-RESOLUTION
    phase: 2
    type: algo
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Identity Resolution Algorithm — Global vs Project Layering
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-11T10:28:00.000Z
    aliases: *a3
    cluster: implementation_flow
    role: Algorithm definition
    attributes:
      domain: algo
    domain: algo
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: algo
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# ALGORITHM — Identity Resolution

Describes the logic for merging global identity data with project-specific overrides.

## 1. Input Sources
- **Global File**: `~/.msp/identity.json` (The base soul).
- **Project Override**: `.brain/msp/projects/<ns>/identity.override.json` (Sparse local tweaks).
- **Defaults**: Hardcoded in `src/identity/types.ts` for each sub-field.

## 2. Resolution logic (`view: 'merged'`)
1.  **Load Global**: Load global file; if missing, use `defaultIdentity()`.
2.  **Load Override**: Load project override file; if missing, treat as empty object.
3.  **Merge Strategy (Shallow-Merge per top-level key)**:
    - `profile`: `{ ...global.profile, ...override.profile }`
    - `voice`: `{ ...global.voice, ...override.voice }`
    - `preferences`: `{ ...global.preferences, ...override.preferences }`
4.  **Schema Enforcement**: Force `schemaVersion: 1` on the final output.

## 3. Scope Semantics
- **Global**: Only Global File + Defaults.
- **Project**: Defaults + Project Override (ignores global).

## Connections
- [[CONCEPT--IDENTITY-LAYER]]
- [[ADR--GLOBAL-VS-WORKSPACE]]

