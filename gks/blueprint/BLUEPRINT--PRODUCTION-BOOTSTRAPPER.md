---
id: BLUEPRINT--PRODUCTION-BOOTSTRAPPER
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Production Bootstrapper Implementation
tags: [msp, infra, bootstrap, plan]
aliases: [installer-plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - CONCEPT--PRODUCTION-BOOTSTRAPPER
created_at: 2026-05-22T03:30:00+07:00
---

# BLUEPRINT — Production Bootstrapper

## 1. Goal

Implement the `msp-bootstrap` CLI tool inside `packages/msp` that creates a new project directory with a clean production-ready structure.

## 2. Implementation Steps

### T1: CLI Entrypoint (`packages/msp/src/usage/cli/bootstrap.ts`)
- Implement a commander command `bootstrap <targetDir>`.
- Use `node:fs` to handle recursive directory creation.

### T2: Skeleton Mapping
- Define a list of "Essential Files" to copy:
    - `packages/msp/dist/**` (compiled core)
    - `packages/gks/dist/**` (compiled engine)
    - `apps/cli/dist/**` (proxy)
    - `apps/mcp/dist/**` (proxy)
    - `.brain/msp/LLM_Contract/**` (essential contracts)
    - `atom_schema.yaml`, `atom_registry.yaml`
- Define "Initial Atoms" for the new `gks/` folder:
    - `FRAMEWORK--MSP-ARCHITECTURE-V2.md`
    - `CONCEPT--KNOWLEDGE-LAYERS-V2.md`
    - (A small set of foundational axioms)

### T3: Brain Initialization
- Automatically create the `.brain/msp/projects/<new-project>/` subdirectories:
    - `memory/`, `session/`, `vector/`, `audit/`, `candidates/`.

### T4: Post-Bootstrap Verification
- Run `npm run msp:index` and `npm run msp:validate` in the target directory to ensure it is healthy.

## 3. Usage Example

```bash
# In the framework repo
npx msp-bootstrap G:\covibe
```

## 4. Verification Plan

- Run the bootstrap command on a temporary folder `C:\tmp\test-project`.
- Verify that `docs/plans/` and `docs/gks/ULTRAPLAN.md` are **NOT** present in the target.
- Verify that `npx gks list` works in the target directory.
