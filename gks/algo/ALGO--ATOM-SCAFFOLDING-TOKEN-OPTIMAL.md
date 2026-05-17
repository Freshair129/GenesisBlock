---
id: ALGO--ATOM-SCAFFOLDING-TOKEN-OPTIMAL
phase: 2
type: algo
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: Token-Optimal Atom Scaffolding
tags:
  - msp
crosslinks: {}
created_at: 2026-05-17T04:07:43.742+07:00
aliases:
  - ALGO
  - implementation_flow
  - Algorithm definition
cluster: implementation_flow
role: Algorithm definition
attributes:
  domain: algo
---

# ALGO — Token-Optimal Atom Scaffolding

## Inputs

- `atom_registry.yaml` schema for the requested atom type.
- Target atom slug and metadata (e.g., namespace, title).
- Generated prompt template from `msp-atom prompt`.

## Algorithm

1. The user/orchestrator requests a prompt template via `msp-atom prompt --type=<type>`.
2. The CLI reads `atom_registry.yaml`, finds the requested type, and outputs a minimal prompt string containing only the required section headers.
3. The LLM generates ONLY the content for those sections (no metadata).
4. The user/orchestrator executes `msp-atom create --type=<type> --slug=<slug> --body-from=<file>`.
5. The CLI validates the input body against the type's JSON schema (sections present).
6. The CLI generates the frontmatter (phase, tier, folder, created_at) internally without LLM involvement.
7. The CLI writes the final composed atom directly to disk.

## Complexity

O(1) file reads for the LLM (down from reading 400+ line KNOWLEDGE-TYPES.md). O(N) where N is the number of registry types during script execution.

## Edge cases

- Missing sections in LLM output: Caught by the script-side validation gate before writing.
- Invalid slug: Caught by strict `/^[A-Z][A-Z0-9_-]*$/` regex.

## Source

- Phase 0 doc-to-code requirement.
