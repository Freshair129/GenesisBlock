---
id: ADR--REGISTRY-DRIVEN-SCAFFOLDING
phase: 2
type: adr
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: Registry-Driven Scaffolding
tags:
  - msp
crosslinks: {"references":["CONCEPT--ATOM-REGISTRY-AS-SSOT"]}
created_at: 2026-05-17T04:07:40.095+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: "Architecture decision record"
---

# ADR — Registry-Driven Scaffolding

## Context

The taxonomy was previously defined in `docs/gks/KNOWLEDGE-TYPES.md` but enforced via hardcoded string lists in multiple files. This caused logic drift and made atom scaffolding an expensive operation in terms of LLM tokens (reading the full markdown to understand the schema).

## Decision

`atom_registry.yaml` is the Single Source of Truth (SSOT) for the type taxonomy. 
- Validator, scaffolder, and future tools MUST read from this registry.
- **Versioning Policy**: The registry uses semver for structure changes.
- **Authority Gate**: Adding or removing types requires PR + human review only. Agents cannot autonomously add new types to the registry.

## Consequences

- The canonical reference for atom types shifts from `docs/gks/KNOWLEDGE-TYPES.md` to `atom_registry.yaml`.
- The scaffolding tools are completely data-driven.
- Token-optimal codegen is possible by loading only the schema slice for a requested type.

## Alternatives considered

- Keeping hardcoded lists: Rejected due to maintenance burden and drift.
- LLM parsing the markdown file: Rejected due to high token costs and latency.

## Source

- Phase 0 doc-to-code requirement.
