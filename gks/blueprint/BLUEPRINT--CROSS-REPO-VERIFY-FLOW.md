---
id: BLUEPRINT--CROSS-REPO-VERIFY-FLOW
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Cross-repo verify-flow Implementation Plan
tags: [msp, governance, verification, plan, m9c]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - FEAT--CROSS-REPO-VERIFY-FLOW
    - CONCEPT--MSP-ROADMAP
linked_symbols:
  - file: packages/gks/bin/gks.ts
  - file: packages/gks/src/memory/verify-flow.ts
created_at: 2026-05-21T11:15:00+07:00
---

# BLUEPRINT — Cross-repo verify-flow

## 1. Goal

Implement the technical logic to load remote atom indices and merge them into the local verification state, enabling `gks verify-flow` to span multiple repositories.

## 2. Implementation Steps

### T1: Remote Index Fetcher (`packages/gks/src/memory/remote-loader.ts`)

- Implement a utility to load an `atomic_index.jsonl` from a remote location.
- Support local path resolution (absolute or relative to current root).
- Support simple HTTP GET for remote URLs (optional for MVP, focus on local paths first).
- Add error handling for missing or malformed remote indices.

### T2: Multi-Index Merger

- Update `gks/bin/gks.ts` in the `cmdVerifyFlow` function.
- Logic:
    1. Parse the `--remote` flag.
    2. Load local index as usual.
    3. If `--remote` is set, load the remote index using T1.
    4. Merge the two indices into the `byId` Map.
    5. **Namespace Handling:** If collisions occur, the local atom wins. In a future iteration, we will support prefixing (e.g., `remote:ADR--XYZ`).

### T3: CLI Option Expansion

- Update the `options` object in `cmdVerifyFlow` to include:
  - `remote`: { type: 'string', multiple: true } (Allow multiple remotes).
- Update the help text to document the new flag.

### T4: Protocol Refinement

- Ensure the `verifyFlow` core logic in `packages/gks/src/memory/verify-flow.ts` remains pure and unaffected. It already operates on a `Map<string, AtomicEntry>`, so the merge logic in the CLI is sufficient for Phase 1.

## 3. Verification Plan

### 3.1 Integration Test

- Create a test fixture with two directories: `repoA` and `repoB`.
- `repoA` contains a stable ADR.
- `repoB` contains a FEAT that references the ADR in `repoA`.
- Run `gks verify-flow FEAT--... --root=repoB --remote=../repoA`.
- Verify success.
- Change ADR in `repoA` to `draft`.
- Verify failure.

### 3.2 Unit Tests

- Test the merge logic with overlapping IDs.
- Test loading of malformed remote indices.
