---
id: CONCEPT--MSP-HOTFIX-WRAPPER
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: MSP hotfix wrapper — npm scripts + pre-commit gate for overdue hotfixes
tags:
  - msp
  - hotfix
  - escape-hatch
  - automation
crosslinks: {"references":["ADR--HOTFIX-ESCAPE-HATCH","FEAT--MSP-PRECOMMIT-HOOK"]}
created_at: 2026-05-03T17:45:48.892+07:00
---

# CONCEPT — MSP hotfix wrapper

## Problem

`[[ADR--HOTFIX-ESCAPE-HATCH]]` defines the 48-hour backfill rule but doesn't wire it into MSP's npm-script surface or local enforcement. Today users must remember to run `npx gks hotfix open ...` and `npx gks hotfix check --file=...` manually. After 48 h the deadline elapses silently — no commit is blocked, no warning printed, contract decays.

## Hypothesis

If MSP exposes `npm run msp:hotfix:{open,list,close,check}` as namespace-consistent passthroughs to `gks hotfix ...`, AND extends the pre-commit hook to invoke `gks hotfix check` on every staged non-`gks/` non-`.brain/` path, then:

- The 48 h timer becomes mechanically enforced on this repo (per `[[ADR--HOTFIX-ESCAPE-HATCH]]`'s "locally" caveat).
- Users discover hotfix commands via `npm run` listing.
- The escape hatch stays usable without procedure-rot.

## Scope

In:
- npm scripts: `msp:hotfix:open`, `msp:hotfix:list`, `msp:hotfix:close`, `msp:hotfix:check`. All thin passthroughs.
- Extend `examples/hooks/pre-commit-validator.sh` to gather staged paths outside `gks/` + `.brain/`, then invoke `gks hotfix check --file=<path>` once aggregating all paths.
- Tests covering the new hook behaviour.

Out:
- Auto-detect HEAD SHA in `msp:hotfix:open` (user passes `$(git rev-parse HEAD)`).
- Distributed enforcement (per `[[ADR--HOTFIX-ESCAPE-HATCH]]`: orchestrator's job, not MSP's).
- New atom types — `HOTFIX--` already exists in the GKS taxonomy.

## Source

P1 item #6 from the M3 production-readiness backlog.

## Connections
- [[FEAT--MSP-PRECOMMIT-HOOK]]

