---
id: FEAT--CODING-DOMAIN-PACK
phase: 2
type: feat
status: active
tier: process
source_type: axiomatic
vault_id: default
title: "FEAT — Coding Domain Pack — classifier and policy set"
tags:
  - msp
  - ucf
  - coding
  - classifier
crosslinks: {"implements":["CONCEPT--CODING-DOMAIN-PACK"],"references":["FEAT--CLASSIFIER-PLUGINS"]}
created_at: 2026-05-17T09:15:00+07:00
cluster: implementation_flow
role: "Feature spec"
aliases:
  - FEAT
  - implementation_flow
---

# FEAT — Coding Domain Pack

## Context

This feature implements the first domain-specific extension for UCF. It consists of a specialized classifier plugin for source code and a corresponding YAML policy pack.

## Requirements

1. **Coding Classifier:**
   - Detect `language` via file extension (`.ts`, `.rs`, `.py`, `.go`, `.js`).
   - Detect `is_test` via path patterns (`*.test.ts`, `**/tests/**`).
   - Detect `is_entrypoint` via specific filenames (`index.ts`, `main.ts`, `App.tsx`, `bin.ts`, `server.ts`).
   - Detect `framework` via content markers (e.g. `express`, `react`).
2. **Policy Set:**
   - Define rules that use these attributes to restrict/permit actions based on agent tier.
   - Example: Deny T1 agents from modifying entrypoints.

## API Contract (Classifier)

Implemented as a standard UCF Classifier plugin.

- **ID:** `domain/coding`
- **Outputs:** `language`, `is_test`, `is_entrypoint`, `framework`

## Policy Rules

Rules reside in `policies/60-coding-domain.yaml`.

| ID | Description | Effect |
|---|---|---|
| `restrict-entrypoint-to-t3` | Only T3 agents or humans can modify entrypoints | `deny` if `S.tier < T3` and `R.is_entrypoint == true` |
| `permit-test-for-t1` | Explicitly permit T1 agents to read/modify test files | `permit` if `S.tier == T1` and `R.is_test == true` |

## Verification Criteria

- Classifier correctly identifies a `.test.ts` file as `language: ts` and `is_test: true`.
- Policy correctly blocks a T1 agent from accessing `packages/msp/src/index.ts`.
- Performance impact remains negligible for large codebases.
