---
id: SAFETY--PII-REDACTION
phase: 1
type: safety
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: PII Redaction Safety — protect sensitive identity data
tags:
  - msp
  - safety
  - pii
  - redaction
  - identity
crosslinks: {"references":["FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK"]}
created_at: 2026-05-14T21:00:00+07:00
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

