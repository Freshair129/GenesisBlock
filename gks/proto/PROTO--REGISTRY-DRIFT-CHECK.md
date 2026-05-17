---
id: PROTO--REGISTRY-DRIFT-CHECK
phase: 2
type: proto
status: draft
tier: safety
source_type: axiomatic
vault_id: default
title: Registry Drift Check
tags:
  - msp
crosslinks: {}
created_at: 2026-05-17T04:07:44.976+07:00
aliases:
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  domain: proto
---

# PROTO — Registry Drift Check

## Rule

Every `type` field declared in any atom's frontmatter MUST be defined in `atom_registry.yaml`. The validator must check the `type` field against the parsed keys of `taxonomy.clusters.*.types`. Furthermore, the `phase` and `tier` of the atom must match the registry's definition for that type.

## Severity

error

## Enforcement

The MSP validator `packages/msp/src/validator/cli.ts` (specifically a new `registry-drift` rule) reads `atom_registry.yaml` on startup and fails validation if an unregistered type or mismatched phase/tier is encountered.

## Counter-example

An atom with `type: invalid-type` or an `ADR` atom with `phase: 1` (registry says phase 2).

## Source

- Phase 0 doc-to-code requirement.
