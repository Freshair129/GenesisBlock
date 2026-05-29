---
id: RUNBOOK--BRAIN-SYNC-PROCEDURE
phase: 6
type: runbook
status: active
tier: process
source_type: axiomatic
title: Runbook — Brain Synchronization Procedure (Shadow Repo)
tags:
  - msp
  - gks
  - runbook
  - ops
  - sync
  - git
crosslinks:
  references:
    - ADR--SHADOW-REPO-SHARED-BRAIN
    - PROTO--BRAIN-INDEXING-MANDATE
created_at: 2026-05-29T12:00:00+07:00
---

# RUNBOOK — Brain Synchronization Procedure

This SOP outlines the steps for agents and humans to synchronize their local knowledge index with the remote Shadow Brain repository.

## 1. Pre-requisites
- Local GKS environment configured to point to the Shadow Repo path.
- Git credentials configured for the Brain Repository.
- `packages/msp` installed and functional.

## 2. The Pull Cycle (Sync-In)
Perform these steps whenever starting a new task or after a long idle period:
1.  **Navigate** to the Shadow Repo directory.
2.  **Execute** `git pull origin main`.
3.  **Resolve Conflicts** manually if the agent cannot perform auto-merge on Markdown files.
4.  **Trigger Re-indexing:** Navigate to the project root and run `npm run msp:index`.
5.  **Verify Integrity:** Run `npm run msp:validate`.

## 3. The Push Cycle (Sync-Out)
Perform these steps after finalizing an atomic update:
1.  **Regen Index:** Run `npm run msp:index` to ensure the local `atomic_index.jsonl` is updated (for local validation only).
2.  **Stage Changes:** `git add gks/<type>/ATOM--ID.md`.
3.  **Commit:** Use the style `feat(brain): add CONCEPT--NEW-KNOWLEDGE`.
4.  **Push:** `git push origin main`.

## 4. Handling Index Mismatch
If the local Graph Backend (`genesisBackend`) behaves unexpectedly or provides stale semantic links:
1.  **Delete** the local `.bin` snapshot and `.jsonl` cache in the local brain root.
2.  **Restart** the Indexer: `npm run msp:index --force`.
3.  **Validate** using `gks recall` to verify the new knowledge is active.

## 5. Troubleshooting
- **Link Breakage:** If `linked_symbols` fail validation, check if the Code Repo is on a different branch than the Brain Repo expected.
- **Sync Latency:** If the remote has moved too far ahead, perform a `git fetch` followed by a `git rebase` to keep a clean linear history.
