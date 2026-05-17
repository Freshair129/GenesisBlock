---
id: FEAT--SECURITY-SECRET-PACK
phase: 2
type: feat
status: active
tier: process
source_type: axiomatic
vault_id: default
title: FEAT — Security & Secret Domain Pack — deep scanner and zero-exposure rules
tags: &a1
  - msp
  - ucf
  - security
  - secrets
crosslinks: &a2
  implements:
    - CONCEPT--SECURITY-SECRET-PACK
  references:
    - FEAT--CLASSIFIER-PLUGINS
created_at: 2026-05-17T10:35:00+07:00
cluster: implementation_flow
role: Feature spec
aliases: &a3
  - FEAT
  - implementation_flow
attributes:
  id: FEAT--SECURITY-SECRET-PACK
  phase: 2
  type: feat
  status: active
  tier: process
  source_type: axiomatic
  vault_id: default
  title: FEAT — Security & Secret Domain Pack — deep scanner and zero-exposure rules
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-17T10:35:00+07:00
  cluster: implementation_flow
  role: Feature spec
  aliases: *a3
  attributes:
    id: FEAT--SECURITY-SECRET-PACK
    phase: 2
    type: feat
    status: active
    tier: process
    source_type: axiomatic
    vault_id: default
    title: FEAT — Security & Secret Domain Pack — deep scanner and zero-exposure rules
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-17T10:35:00+07:00
    cluster: implementation_flow
    role: Feature spec
    aliases: *a3
    domain: feat
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: feat
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# FEAT — Security & Secret Domain Pack

## Context

This feature implements the security-hardened extension for UCF. it introduces a `SecurityClassifier` capable of deep content scanning and a policy pack that enforces local-only processing for secrets.

## Requirements

1. **Security Classifier:**
   - Detect `has_secret` (boolean) via regex patterns for AWS, GitHub, Stripe, OpenAI, etc.
   - Detect `secret_type` (key, password, token, jws).
   - Detect `encryption_level` (none, vault, pgp).
   - Heuristic entropy check for random-looking high-length strings.
2. **Policy Set:**
   - **Zero Cloud Exposure:** Absolutely `deny` `expose-to-llm` if `R.has_secret == true` AND agent is not local (T2/T3).
   - **Local Processing Permit:** `permit` reading secrets only for `T1` agents (local SLM) or `human` users.
   - **Step-up for High Risk:** Require PIN for any modification to atoms with `encryption_level: vault`.

## API Contract (Classifier)

Implemented as a standard UCF Classifier plugin.

- **ID:** `domain/security`
- **Outputs:** `has_secret`, `secret_type`, `encryption_level`, `leak_risk`

## Policy Rules

Rules reside in `policies/80-security-secrets.yaml`.

| ID | Description | Effect |
|---|---|---|
| `block-secrets-from-cloud` | Block secrets from reaching Cloud LLMs | `deny` if `R.has_secret == true` and `S.tier > T1` |
| `permit-local-secrets` | Allow local agents to see secrets | `permit` if `R.has_secret == true` and `S.tier == T1` |

## Verification Criteria

- A file containing `sk-ant-03-` is correctly tagged as `has_secret: true`.
- A request from Gemini (T2) to read a secret-tagged file is denied.
- A request from local Qwen (T1) to read the same file is permitted.
