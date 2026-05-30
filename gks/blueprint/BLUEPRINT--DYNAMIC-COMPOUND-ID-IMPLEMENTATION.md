---
id: BLUEPRINT--DYNAMIC-COMPOUND-ID-IMPLEMENTATION
phase: 3
type: blueprint
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Implementation Plan for Dynamic Compound ID Formatting and Schema Restructuring
tags:
  - msp
  - codegen
  - schema
  - architecture
crosslinks:
  references:
    - ADR--DYNAMIC-COMPOUND-ID-K-SUFFIX
    - CONCEPT--USAGE-ROLLUPS
created_at: 2026-05-17T10:07:15.100+07:00
aliases:
  - BLUEPRINT
cluster: implementation_flow
role: Implementation plan
attributes:
  knowledgeId: DYNAMIC-COMPOUND-ID-IMPLEMENTATION
  linked_symbols:
    - file: packages/gks/src/memory/atomic-id.ts
    - file: scripts/msp/msp-atom.ts
    - file: atom_schema.yaml
    - file: atom_registry.yaml
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
linked_symbols:
  - file: packages/gks/src/memory/atomic-id.ts
  - file: src/validator/utils/registry.ts
  - file: scripts/msp/msp-atom.ts
  - file: scripts/msp/codegen-schemas.ts
  - file: atom_registry.yaml
---

# BLUEPRINT — Dynamic Compound ID and Registry Restructuring

## Geography

* [atom_registry.yaml](file:///c:/Users/freshair/cognitive_system/atom_registry.yaml)
* [atomic-id.ts](file:///c:/Users/freshair/cognitive_system/packages/gks/src/memory/atomic-id.ts)
* [registry.ts](file:///c:/Users/freshair/cognitive_system/src/validator/utils/registry.ts)
* [msp-atom.ts](file:///c:/Users/freshair/cognitive_system/scripts/msp/msp-atom.ts)
* [codegen-schemas.ts](file:///c:/Users/freshair/cognitive_system/scripts/msp/codegen-schemas.ts)

---

## Acceptance

1. **Registry Architecture**:
    * `atom_registry.yaml` is split into `schema_config` and `atom_list`.
    * No duplicate/redundant sequence keys remain on individual types.
    * All standard frontmatter fields (e.g. `vault_id`, `tier`, `source_type`) are globally declared under `schema_spec.fields`.
2. **Scaffolding & Templates**:
    * `scripts/msp/msp-atom.ts` reads the template parameters dynamically from `schema_config`.
    * `npx tsx scripts/msp/msp-atom.ts scaffold` compiles new compound IDs correctly.
3. **Validation Green Line**:
    * All 378 unit tests pass successfully.
    * Vitest suites complete with zero errors.

---

## Dependencies

* [ADR-376--DYNAMIC-COMPOUND-ID-K-SUFFIX--K3](file:///c:/Users/freshair/cognitive_system/gks/adr/ADR-376--DYNAMIC-COMPOUND-ID-K-SUFFIX--K3.md)

---

## Tasks

* [x] **Registry Restructuring**:
  * [x] Restructure `atom_registry.yaml` into nested `schema_config` and `atom_list`.
  * [x] Declare standard GKS frontmatter fields dictionary in `schema_spec.fields`.
  * [x] Purge all redundant `primary_key`, `sequence_key`, `search_key`, `phase_key` attributes from individual types.
* [x] **Core Validation Adapters**:
  * [x] Relax `ATOMIC_ID_PATTERN` regex in `packages/gks/src/memory/atomic-id.ts` to support both legacy and compound formats.
  * [x] Update prefix splitter in `src/validator/utils/registry.ts` and `required-fields.ts` to split by `-` instead of `--`.
  * [x] Adapt parser in `registry.ts` to support nested taxonomy.
* [x] **Code Generation & Scaffolder**:
  * [x] Update `scripts/msp/msp-atom.ts` to resolve counter levels and templates dynamically from `registry.schema_config`.
  * [x] Update `scripts/msp/codegen-schemas.ts` to generate schemas from `registry.schema_config.taxonomy`.
* [x] **Verification & Sign-off**:
  * [x] Run `npm run msp:codegen` and verify schema output.
  * [x] Run `npm test --workspace=packages/gks` to guarantee zero regressions.

---

## Source

* [atom_registry.yaml](file:///c:/Users/freshair/cognitive_system/atom_registry.yaml)
* [AUTHORING-MANUAL.md](file:///c:/Users/freshair/cognitive_system/docs/gks/AUTHORING-MANUAL.md)
