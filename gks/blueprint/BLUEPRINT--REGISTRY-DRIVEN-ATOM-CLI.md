---
id: BLUEPRINT--REGISTRY-DRIVEN-ATOM-CLI
phase: 3
type: blueprint
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: Registry-Driven Atom CLI
tags:
  - msp
crosslinks: {}
linked_symbols:
  - file: scripts/msp/msp-atom.ts
  - file: scripts/msp/codegen-schemas.ts
  - file: packages/msp/src/validator/rules/registry-drift.ts
created_at: 2026-05-17T04:07:42.521+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  domain: blueprint
---

# BLUEPRINT — Registry-Driven Atom CLI

## Geography

- `scripts/msp/codegen-schemas.ts` (NEW)
- `scripts/msp/msp-atom.ts` (NEW / refactored from scaffold-atom.ts)
- `packages/msp/src/validator/cli.ts` (MODIFIED)

## Acceptance

- [ ] Codegen script produces JSON schemas and prompt templates.
- [ ] `msp-atom` CLI supports three modes: `prompt`, `create`, `scaffold`.
- [ ] Mode B (`create`) writes atom content without reading the existing files, achieving a token cost of ~700 tokens per generation.
- [ ] Mode A (`prompt`) prints templates using ~150 tokens.

## Dependencies

- `atom_registry.yaml` (SSOT).
- `yaml` and `ajv` / JSON schema libraries.

## Tasks

- **T1**: Implement `codegen-schemas.ts` to output derived artifacts into `.brain/msp/schemas/`.
- **T2**: Add `npm run msp:codegen` to root scripts.
- **T3**: Refactor `scaffold-atom.ts` into `msp-atom.ts` with `prompt` Mode A.
- **T4**: Implement `msp-atom create` Mode B with validation gate.
- **T5**: Maintain `msp-atom scaffold` Mode C for backward compatibility.
- **T6**: Integrate generated schemas into validator (`registry-drift` rule).

## Source

- Phase 0 doc-to-code requirement.
