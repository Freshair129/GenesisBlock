---
name: shadow-brain-orchestrator
description: Manage external "Shadow Repositories" for GKS knowledge sharing. Use when switching AI brains, synchronizing knowledge via Git across multiple environments, or ensuring cross-repo symbol integrity without crashing IDE agents like Antigravity.
---

# Shadow Brain Orchestrator

## Overview

This skill enables Gemini CLI to manage the "Portable Brain" architecture of the Genesis Knowledge System (GKS). It allows agents to decouple knowledge (`gks/`) from code, allowing for decentralized, auditable, and shared intelligence across multiple repositories and users.

## Core Capabilities

### 1. Brain Path Resolution
Dynamically resolve and switch between different knowledge bases using the `MSP_BRAIN_PATH` environment variable.
- **Trigger:** "Switch to team brain", "Connect to personal repository".
- **Action:** Verify path existence and update session context.

### 2. Automated Sync-in (Pull & Index)
Synchronize the local environment with the remote Shadow Brain.
- **Procedure:**
    1. Navigate to the Shadow Repo path.
    2. Execute `git pull origin main`.
    3. Return to the code root and run `npm run msp:index`.
- **Enforcement:** Mandate a re-index after every pull to maintain semantic link integrity.

### 3. Atomic Sync-out (Selective Push)
Deploy new knowledge atoms to the Shadow Repo without cluttering the Code Repo.
- **Procedure:**
    1. Validate atoms using `npm run msp:validate`.
    2. Stage ONLY `.md` files in the Shadow Repo's `gks/` folder.
    3. Commit with `feat(brain): add <ATOM-ID>` style.
    4. Push to remote.

### 4. Cross-Repo Integrity Guard
Verify that `linked_symbols` in the external brain remain valid against the local code AST.
- **Action:** Search for broken symbol links across the repository boundary.

## Safety SOPs (Critical for Antigravity Compatibility)

To prevent crashes in IDE-embedded agents like Antigravity, follow these rules:

### Sibling & Ignore Principle
- **Rule:** Never place a Shadow Repo (`.git` folder) inside the main project directory unless it is explicitly listed in the main repo's `.gitignore`.
- **Safe Setup:** Place the Shadow Repo as a **Sibling** directory (e.g., `../cognitive_system_brain`).

### Local State Shield
- **Rule:** Never commit local indexing artifacts to the Shadow Repo.
- **Shielded Files:**
    - `*.bin` (Graph snapshots)
    - `*.jsonl` (Event logs, except `atomic_index.jsonl` if required)
    - `*.lock` (File locks)
    - `.brain/` (Ephemeral session state)

## Workflow Examples

### Synchronizing Knowledge
> "Update my knowledge from the cloud brain."
1. Determine `MSP_BRAIN_PATH`.
2. Perform Git Pull.
3. Run Re-indexer.
4. Notify user of new atoms found.

### Proposing an Atom to Shadow Repo
> "I've finished the concept for the new API, push it to the brain repo."
1. Copy the atom from local candidates to Shadow Repo `gks/`.
2. Run validation.
3. Commit and Push.

## Resources

### scripts/
- `sync_and_index.ps1`: Automated PowerShell script to handle the pull-index cycle.

### references/
- `ADR--SHADOW-REPO-SHARED-BRAIN.md`: The foundational architecture decision.
- `RUNBOOK--BRAIN-SYNC-PROCEDURE.md`: The standard operational procedure.
