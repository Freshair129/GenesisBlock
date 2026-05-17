---
id: CONCEPT--ATOM-REGISTRY-AS-SSOT
phase: 1
type: concept
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: Atom Registry as Single Source of Truth
tags:
  - msp
crosslinks: {}
created_at: 2026-05-17T04:07:38.843+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  domain: concept
---

# CONCEPT — Atom Registry as Single Source of Truth

## Problem

There is persistent taxonomy drift between `docs/gks/KNOWLEDGE-TYPES.md` and hardcoded strings in `scripts/types.ts`. Furthermore, asking LLMs to read the full markdown taxonomy for every scaffolding operation incurs high token costs (around 4700 tokens per generation).

## Hypothesis

A central `atom_registry.yaml` can serve as the Single Source of Truth (SSOT). This allows programmatic access for scaffolding and validation tools without manual parsing of markdown files.

## Scope

- Establishing `atom_registry.yaml` as the canonical taxonomy registry.
- Removing hardcoded type strings in validation and scaffolding logic.
- Reducing LLM atom creation cost via schema-driven, token-optimal generation.

## Out of scope

- Redefining the semantics of existing atoms.
- Changing the folder structure of `gks/` (already handled in flat-layout ADRs).

## Verification

- The `msp:validate` checks pass.
- Scaffolder can dynamically read the registry.
- Token cost measurements drop significantly (target ~700 tokens).

## Source

- Phase 0 doc-to-code requirement.
