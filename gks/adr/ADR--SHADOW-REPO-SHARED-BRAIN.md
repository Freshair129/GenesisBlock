---
id: ADR--SHADOW-REPO-SHARED-BRAIN
phase: 2
type: adr
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Shadow Repo Architecture for Shared AI Brain
tags:
  - msp
  - gks
  - architecture
  - multi-agent
  - shadow-repo
  - gitops
crosslinks:
  references:
    - COGNITIVE--SHARED-BRAIN-PHILOSOPHY
    - RUNBOOK--BRAIN-SYNC-PROCEDURE
    - PROTO--BRAIN-INDEXING-MANDATE
created_at: 2026-05-29T12:00:00+07:00
---

# ADR — Shadow Repo Architecture for Shared AI Brain

## Context
As the `cognitive_system` evolves into a multi-agent and multi-user environment, keeping the knowledge base (`gks/` folder) inside the main code repository (Monorepo approach) creates friction:
- **History Noise:** Rapid knowledge updates clutter the code commit history.
- **Access Control:** All developers have full access to all knowledge, preventing sensitive "brain" sharding.
- **Portability:** Moving the AI's "brain" to a different project requires manual file copying.

## Decision
We will adopt a **"Shadow Repo"** architecture (GitOps for Knowledge). The knowledge store will be moved to a dedicated repository (e.g., `cognitive_system_brain`) that acts as a Cloud Database via GitHub.

### Key Mechanisms:
- **Brain Repo:** A standalone repository containing only the `gks/` directory and governance atoms.
- **Code Repo:** Remains focused on source code, tests, and infrastructure.
- **Local Ephemeral State:** Files like `genesis-graph.jsonl`, `.bin` snapshots, and `backlinks.jsonl` remain local to each agent's environment and are **NEVER** committed to the Brain Repo.
- **Cross-Repo Validation:** A new `PROTO--` will be established to validate `linked_symbols` across the repository boundary.

## Consequences
### Positive:
- **Sovereign Collaboration:** Agents can pull shared knowledge, work offline, and push updates back to a central authority (GitHub).
- **Infinite Sharding:** One Code Repo can connect to multiple Brain Repos (e.g., `personal_brain`, `team_brain`, `project_brain`).
- **Clean Audit:** Knowledge evolution is tracked separately from code evolution.

### Negative/Risks:
- **Synchronization Overhead:** Requires disciplined `git pull/push` cycles.
- **Link Drift:** Renaming code symbols without updating the Brain Repo will break traceability.

## Implementation Strategy
1. **Refactor `gksLayout`:** Change `packages/gks` to support dynamic paths for the knowledge store.
2. **Setup Shadow Repo:** Create the external repository and move `gks/` content.
3. **Automate Indexing:** Implement git hooks to trigger `msp:index` on every pull.
