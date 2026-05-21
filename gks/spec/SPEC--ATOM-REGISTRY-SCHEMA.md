---
id: SPEC--ATOM-REGISTRY-SCHEMA
phase: 2
type: spec
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Atom Registry Schema
tags: &a1
  - msp
crosslinks: &a2 {}
created_at: 2026-05-17T04:07:41.308+07:00
aliases: &a3
  - SPEC
  - implementation_flow
  - Technical specification
cluster: implementation_flow
role: Technical specification
attributes:
  id: SPEC--ATOM-REGISTRY-SCHEMA
  phase: 2
  type: spec
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Atom Registry Schema
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-17T04:07:41.308+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Technical specification
  attributes:
    id: SPEC--ATOM-REGISTRY-SCHEMA
    phase: 2
    type: spec
    status: draft
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Atom Registry Schema
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-17T04:07:41.308+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Technical specification
    attributes:
      id: SPEC--ATOM-REGISTRY-SCHEMA
      phase: 2
      type: spec
      status: draft
      tier: process
      source_type: axiomatic
      vault_id: default
      title: Atom Registry Schema
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-17T04:07:41.308+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Technical specification
      attributes:
        domain: spec
      domain: spec
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: spec
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: spec
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# SPEC — Atom Registry Schema

## Data Shape

The YAML structure contract for the registry is defined as follows:

- **Version**: A string representing the taxonomy version (e.g., "2.4").
- **Last Updated**: ISO-8601 timestamp with offset.
- **Taxonomy**:
  - **Clusters**: Broad groupings (e.g., `implementation_flow`, `agent_governance`).
    - **Types**: Key-value pairs where the key is the UPPERCASE atom prefix.
      - `phase`: The numeric phase (0-6).
      - `role`: Short descriptive role.
      - `tier`: Governance tier (`process`, `master`, `safety`).
      - `folder`: The lowercase folder name under `gks/`.
      - `sections`: An array of section headers required for this atom type.
      - `db_id` (optional): The database identifier field name used for Supabase integration (e.g. `atomId`, `reqId`).
      - `description` (optional): Extended semantics.
      - `lifecycle` (optional): Intended longevity.

## Wire Format

The registry acts as a JSON Schema provider for downstream tools. The fields listed in the registry for each type define the `required` elements of the markdown body and the `frontmatter` requirements (when compiled by the schema codegen script).

## Source

- Phase 0 doc-to-code requirement.
