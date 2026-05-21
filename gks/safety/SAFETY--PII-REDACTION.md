---
id: SAFETY--PII-REDACTION
phase: 0
type: safety
status: stable
tier: safety
source_type: axiomatic
vault_id: default
title: PII Redaction Safety — protect sensitive identity data
tags: &a1
  - msp
  - safety
  - pii
  - redaction
  - identity
crosslinks: &a2
  references:
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
created_at: 2026-05-14T21:00:00+07:00
aliases: &a3
  - SAFETY
  - implementation_flow
  - Ethical safety / AI alignment
cluster: implementation_flow
role: Ethical safety / AI alignment
attributes:
  id: SAFETY--PII-REDACTION
  phase: 0
  type: safety
  status: stable
  tier: safety
  source_type: axiomatic
  vault_id: default
  title: PII Redaction Safety — protect sensitive identity data
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T21:00:00+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Ethical safety / AI alignment
  attributes:
    id: SAFETY--PII-REDACTION
    phase: 0
    type: safety
    status: stable
    tier: safety
    source_type: axiomatic
    vault_id: default
    title: PII Redaction Safety — protect sensitive identity data
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T21:00:00+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Ethical safety / AI alignment
    attributes:
      id: SAFETY--PII-REDACTION
      phase: 0
      type: safety
      status: stable
      tier: safety
      source_type: axiomatic
      vault_id: default
      title: PII Redaction Safety — protect sensitive identity data
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T21:00:00+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Ethical safety / AI alignment
      attributes:
        domain: safety
      domain: safety
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: safety
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: safety
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# SAFETY — PII Redaction

## Rule

Identity data and memory retrieved for a subagent MUST be redacted if it contains Personally Identifiable Information (PII) not within the subagent's scope.

## Constraints

- **Subject Redaction**: The `Subject.id` should be used only for audit purposes; if exposed to a subagent, it may be pseudonymised.
- **Metadata Protection**: Atom attributes containing sensitive data (e.g., `real_name`, `email`) must be filtered out before the atom is composed into the context.

## Enforcement

The Policy Enforcement Point (PEP) in Phase 2+ handles the filtration of sensitive attributes based on the `SubagentScope`.
Specific regex-based or LLM-based PII detection is a future enhancement (M11+).

## Connections

- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]
