---
id: PARAMS--IDENTITY-PROFILE-DEFAULTS
phase: 2
type: params
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Identity Profile Defaults — baseline tunable values
tags: &a1
  - msp
  - identity
  - params
  - configuration
crosslinks: &a2
  references:
    - CONCEPT--IDENTITY-LAYER
created_at: 2026-05-14T20:30:00+07:00
aliases: &a3
  - PARAMS
  - implementation_flow
  - Constants / business config
cluster: implementation_flow
role: Constants / business config
attributes:
  id: PARAMS--IDENTITY-PROFILE-DEFAULTS
  phase: 2
  type: params
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Identity Profile Defaults — baseline tunable values
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T20:30:00+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Constants / business config
  attributes:
    id: PARAMS--IDENTITY-PROFILE-DEFAULTS
    phase: 2
    type: params
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Identity Profile Defaults — baseline tunable values
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T20:30:00+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Constants / business config
    attributes:
      domain: params
    domain: params
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: params
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# PARAMS — Identity Profile Defaults

Baseline tunable values used when no project-specific identity overrides are present.

## Profile Defaults
- `name`: "Eva"
- `role`: "General Purpose Agent"
- `tier`: "T2"
- `origin_story`: "Initial bootstrap of the MSP Identity Engine."

## Voice Defaults
- `tone`: ["analytical", "neutral"]
- `formality`: "neutral"
- `language_preference`: "thai+english"
- `response_cadence`: "terse"

## Operational Params
- `default_top_k`: 10
- `max_context_tokens`: 2000
- `step_up_ttl_seconds`: 300
- `expand_limit_per_session`: 5

## Connections
- [[CONCEPT--IDENTITY-LAYER]]

