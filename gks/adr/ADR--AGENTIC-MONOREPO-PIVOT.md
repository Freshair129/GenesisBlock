---
id: ADR--AGENTIC-MONOREPO-PIVOT
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Drop standalone publish; cognitive_system monorepo IS the product (agentic, agent-pluggable)
tags: [msp, gks, monorepo, agentic, pivot, decision, foundation]
crosslinks: {"references":["ADR--MONOREPO-STRUCTURE","FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--AGENT-AGNOSTIC"],"supersedes":[]}
created_at: 2026-05-13T18:35:00+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: "Architecture decision record"
---

# [[ADR--AGENTIC-MONOREPO-PIVOT]]

## Context
The `cognitive_system` monorepo was originally designed with `packages/gks/` as a standalone publishable library (`@freshair129/gks`). This requirement forced a fragmented architecture where each package (GKS and MSP) maintained its own documentation, scripts, and atom vaults. This duplication resulted in increased maintenance overhead, inconsistent documentation, and complex cross-package workflows.

## Decision
We have decided to drop the requirement for standalone publishing of the GKS package. The `cognitive_system` monorepo itself is now the primary product—an agentic system designed to orchestrate pluggable cognitive agents (Claude Code, Gemini CLI, Qwen CLI, etc.). This pivot allows us to adopt the canonical monorepo layout specified in `FRAMEWORK_MASTER_SPEC.md §4.2`, centralizing all shared resources.

## Consequences
**Positive**
- **Unified Brain:** Single root-level `gks/` atom vault eliminates knowledge duplication.
- **Consolidated Tooling:** Scripts and documentation are unified at the root level.
- **Improved DX:** Simplified workflows for cross-cutting changes.
- **Agent Agnostic:** Clearer tier-based agent stack (Qwen, Gemini, Claude).

**Negative**
- **No Standalone GKS:** The GKS engine is no longer separately installable from npm.
- **Monorepo Dependency:** Use of any sub-system requires the full monorepo context.

## What this enables
- **PR-B (Brain Unification):** Merging `gks/` and `gks/` into a single root `gks/` vault.
- **PR-C (Tooling Centralization):** Moving docs and scripts to the root level and shrinking sub-package `CLAUDE.md` files to minimal shims.

## What's NOT affected
The separation of concerns between the GKS engine and the MSP orchestrator remains intact. GKS continues to function as the storage-engine sub-system, and its scope as defined in `ADR-008` still holds within the unified monorepo structure.

## Connections
- [[ADR--MONOREPO-STRUCTURE]]
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--AGENT-AGNOSTIC]]

