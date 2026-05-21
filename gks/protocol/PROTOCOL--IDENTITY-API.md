---
id: PROTOCOL--IDENTITY-API
phase: 2
type: protocol
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Identity API Protocol — Get/Set Soul Passport sub-fields
tags: &a1
  - msp
  - identity
  - api
  - soul
  - profile
  - voice
  - preferences
crosslinks: &a2
  references:
    - CONCEPT--IDENTITY-LAYER
    - ADR--IDENTITY-STORAGE-SHAPE
created_at: 2026-05-11T10:28:00.000Z
aliases: &a3
  - PROTOCOL
  - agent_governance
  - Interaction contract
cluster: agent_governance
role: Interaction contract
attributes:
  id: PROTOCOL--IDENTITY-API
  phase: 2
  type: protocol
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: Identity API Protocol — Get/Set Soul Passport sub-fields
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-11T10:28:00.000Z
  aliases: *a3
  cluster: agent_governance
  role: Interaction contract
  attributes:
    id: PROTOCOL--IDENTITY-API
    phase: 2
    type: protocol
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: Identity API Protocol — Get/Set Soul Passport sub-fields
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-11T10:28:00.000Z
    aliases: *a3
    cluster: agent_governance
    role: Interaction contract
    attributes:
      id: PROTOCOL--IDENTITY-API
      phase: 2
      type: protocol
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: Identity API Protocol — Get/Set Soul Passport sub-fields
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-11T10:28:00.000Z
      aliases: *a3
      cluster: agent_governance
      role: Interaction contract
      attributes:
        domain: protocol
      domain: protocol
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: protocol
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: protocol
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# PROTOCOL — Identity API

Describes the programmatic interface for interacting with the MSP Identity Layer ("Soul").

## 1. Interaction Surface

### `getIdentity(opts?)`

- **Intent**: Retrieve the complete identity record for a namespace.
- **Input**: `root` (path), `namespace` (string), `view` ('merged'|'global'|'project').
- **Output**: `Identity` object (default-filled if file missing).
- **Behaviour**: Layered read according to `[[ALGO--IDENTITY-RESOLUTION]]`.

### `setProfile(opts?, partialProfile)`

- **Intent**: Atomically update identifying facts (name, role, etc.).
- **Constraint**: `createdAt` is immutable after initial stamp.

### `setVoice(opts?, partialVoice)`

- **Intent**: Atomically update communication style (tone, formality, etc.).

### `setPreference(opts?, key, value, ttl?)`

- **Intent**: Set a scoped preference with optional expiration.

## 2. Storage Contract

- **Atomic**: All writes MUST use the temp-then-rename pattern per `[[ADR--IDENTITY-STORAGE-SHAPE]]`.
- **Namespaced**: Files isolated by namespace subdirectory.

## Connections

- [[CONCEPT--IDENTITY-LAYER]]
