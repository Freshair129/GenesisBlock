---
id: AUDIT--LINKED-SYMBOLS-PATH-SWEEP
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: AUDIT — linked_symbols path-drift sweep (T1-A)
tags:
  - msp
  - gks
  - linked-symbols
  - audit
crosslinks:
  references:
    - AUDIT--PROTO-LINKED-SYMBOLS-PATH-DRIFT
created_at: 2026-05-14T20:30:00+07:00
---

# AUDIT — linked_symbols path-drift sweep

## Scope

This audit covers the completion of Tier 1-A from the post-Phase-F handoff.
The task involved updating `linked_symbols` paths in GKS atoms that still
carried pre-monorepo relative paths (e.g., `"file":"src/X"`) to monorepo-root-relative
paths (e.g., `"file":"packages/msp/src/X"`).

## What shipped

- **68 atoms updated** across `gks/adr/`, `gks/blueprint/`, `gks/feat/`, and `gks/audit/`.
- Paths were only updated if the target file was verified to exist in `packages/msp/` or `packages/gks/`.
- Non-existent targets were left unchanged for human triage (approx. 12 atoms).

## Verification

- `npm run msp:index && npm run msp:validate` — All atoms correctly indexed and validated.
- All 322 atoms passed validation (after minor cleanup of non-atom files).

## Sign-off

- Implemented by: Gemini CLI
- Verified by: `msp:validate`
- Date: 2026-05-14
