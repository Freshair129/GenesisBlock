---
id: CONCEPT--PRODUCTION-BOOTSTRAPPER
phase: 1
type: concept
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: CONCEPT — Production Bootstrapper — instantiating clean GKS/MSP projects
tags: [msp, infra, bootstrap, production, installer]
aliases: [installer, project-generator]
cluster: implementation_flow
role: Strategic intent / PRD
crosslinks:
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
created_at: 2026-05-22T03:15:00+07:00
---

# CONCEPT — Production Bootstrapper

## Intent

To provide a CLI-based tool that can instantiate a fresh, production-ready Cognitive System project in a new directory. This separates the **Framework Development** (this repo) from **Production Usage** (the new instances).

## Problem

Currently, starting a new project (like CoVibe) requires manually copying files or cloning this repo and then "cleaning it up" (deleting design docs, ultraplans, etc.). This is error-prone and mixes framework debt with project implementation.

## Hypothesis

If we provide a command `msp-bootstrap <dir>`, the system can automatically:
1. Create a clean canonical layout (`packages/`, `apps/`, `gks/`, `.brain/`).
2. Copy the stable binaries and core logic.
3. Exclude the "scaffolding" (legacy design docs, temporary specs).
4. Provide a "ready-to-vibe" environment for EVA-CLI to start working immediately.

## Success Criteria

1.  A developer can run `npx msp-bootstrap ./my-new-project`.
2.  The resulting directory has a 100% green GKS state.
3.  The directory contains only production-essential logic, no framework design history.
4.  The framework repo (this one) remains the source for reference and editing of design docs.

## Connections
- `[[BLUEPRINT--PRODUCTION-BOOTSTRAPPER]]` — the implementation plan.
