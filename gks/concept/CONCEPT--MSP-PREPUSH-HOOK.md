---
id: CONCEPT--MSP-PREPUSH-HOOK
phase: 1
type: concept
status: stable
vault_id: default
title: MSP pre-push hook — verify-flow per touched FEAT before push
tags:
  - msp
  - prepush
  - hook
  - verify-flow
  - automation
crosslinks: {"references":["FEAT--MSP-PRECOMMIT-HOOK","FEAT--MSP-VALIDATOR"]}
created_at: 2026-05-03T10:39:27.384Z
---

# CONCEPT — MSP pre-push hook

## Problem

The pre-commit hook catches schema, ID, and wikilink violations on each staged atom — file-by-file. But chain integrity is a property of the *whole graph*: an ADR can be valid in isolation while its FEAT references it incorrectly, or a BLUEPRINT can be promoted before its FEAT. Pre-commit can't catch this — it only sees the file being edited.

Pre-push is the right gate for cross-atom checks because by the time we push, all candidate atoms in the range are already promoted (or about to be). One `gks verify-flow` call per touched FEAT validates the full chain reachable from that FEAT.

## Hypothesis

If a portable bash hook reads the push range from stdin (oldsha → newsha per ref), computes which FEAT atoms had any related file (FEAT/ADR/CONCEPT/BLUEPRINT/AUDIT) touched in that range, and runs `npx gks verify-flow <FEAT-ID>` on each — then broken chains are caught locally before they reach the remote, with no orchestrator dependency.

## Scope

In:
- Read git's pre-push stdin format: lines of `<local-ref> <local-sha> <remote-ref> <remote-sha>`.
- For each push range, `git diff --name-only <remote-sha>..<local-sha>` filtered to `gks/**/*.md`.
- Extract `FEAT--<slug>` candidates either from the FEAT files directly, or from upstream FEATs that reference touched ADR/CONCEPT/BLUEPRINT atoms (best-effort: just collect FEATs whose path was touched + the FEATs explicitly named in commit messages).
- Run `npx gks verify-flow` on each unique candidate; collect failures.
- Exit 1 on any non-OK; standard `--no-verify` bypass.

Out:
- Reverse-link traversal (which FEATs reference a touched ADR?). We only walk FEATs whose own file was touched. The full reverse traversal is expensive and not always wanted; orchestrator can do it in CI.
- Auto-fix.
- Push to multiple remotes — hook runs once per push.

## Source

P1 item #7 from the M3 production-readiness backlog.
