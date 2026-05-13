---
id: CONCEPT--TWO-BRAIN-ARCHITECTURE
phase: 1
type: concept
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: Two-Brain Architecture — global ~/.brain/ + project <repo>/gks/
tags:
  - msp
  - architecture
  - storage
  - two-brain
  - foundation
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--AGENT-AGNOSTIC","ADR--GLOBAL-VS-WORKSPACE","CONCEPT--NAMESPACE-VAULT-BRAIN"]}
created_at: 2026-05-14T01:50:00.000+07:00
---

# CONCEPT — Two-Brain Architecture

An agentic system has two distinct **knowledge gravity wells**, and conflating them causes either privacy leaks (project secrets in global memory) or amnesia (lessons learned in one project never transfer to the next).

## The two brains

| Brain | Path | What lives there | Lifetime |
|---|---|---|---|
| **Global** | `~/.brain/` | Cross-project skills, agent identity, learned routing preferences, generic patterns | Across all projects, machine-local |
| **Project** | `<repo>/gks/` | Project-specific atoms (ADRs, FEATs, BLUEPRINTs, AUDITs), code references, episodes tied to this codebase | Committed to git, scoped to one repo |

This evolves the prior pattern in `ADR--GLOBAL-VS-WORKSPACE` (`~/.msp/` + `.brain/msp/projects/<ns>/`) — the new naming aligns with `FRAMEWORK_MASTER_SPEC §4.2` and removes the redundant `msp/projects/<ns>` nesting.

## What goes where

**Global (`~/.brain/`)** — survives `rm -rf <project>`:

- `~/.brain/identity.json` — agent persona, capabilities, T-tier preferences
- `~/.brain/skills/` — reusable skill packs (e.g. `skill-creator`)
- `~/.brain/episodic/cross-project.jsonl` — patterns observed across many projects
- `~/.brain/registry.yaml` — known project paths + their last-touched timestamps

**Project (`<repo>/gks/`)** — committed; lives or dies with the repo:

- `gks/{adr,concept,feat,blueprint,...}/` — atom vault
- `gks/00_index/` — derived index files (gitignored)
- (No `.brain/` inside the project — that was the old idiom; superseded by this concept)

## Why two, not one?

A single global brain (every project's atoms in `~/.brain/`) breaks `vault_id` isolation — `FEAT--LOGIN` in project A would collide with `FEAT--LOGIN` in project B.

A single project brain (everything in `<repo>/gks/`, no global) means each new project starts from zero — the agent forgets every routing lesson, every retry policy, every cost preference learned elsewhere.

Two brains, with a deterministic resolver, gives both: shared learning + scoped artefacts.

## Resolver contract

When `dispatch(query)` is called, the resolver picks per atom type:

```
type ∈ {ADR, FEAT, BLUEPRINT, AUDIT, CONCEPT}  → project brain ONLY
type ∈ {SKILL, ALGO, PROTO}                    → global first, project fallback
type ∈ {EPISODE, IDENTITY, REGISTRY}           → global ONLY
```

See `ADR--BRAIN-PATH-RESOLUTION` for the precise rules.

## What this concept is NOT

- It's not a sync mechanism — the two brains are independent. Cross-pollination is explicit (write a SKILL to global; write an ADR to project).
- It's not a cache layer — both brains are sources of truth in their own scope.
- It's not a backup strategy — each brain has its own persistence (global = local FS, project = git).

## Related

- `ADR--BRAIN-PATH-RESOLUTION` — concrete path rules + lookup order
- `BLUEPRINT--BRAIN-MERGE-STRATEGY` — implementation plan
- `ADR--GLOBAL-VS-WORKSPACE` — prior version of this idea (kept for history)
- `CONCEPT--NAMESPACE-VAULT-BRAIN` — sibling concept on how vaults map to brains
