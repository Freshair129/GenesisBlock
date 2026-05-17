---
id: ADR--BRAIN-PATH-RESOLUTION
phase: 2
type: adr
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: ADR — Brain Path Resolution — deterministic per-type routing between ~/.brain/ and <repo>/gks/
tags:
  - msp
  - storage
  - two-brain
  - path-resolution
  - decision
crosslinks:
  references:
    - CONCEPT--TWO-BRAIN-ARCHITECTURE
    - ADR--GLOBAL-VS-WORKSPACE
    - CONCEPT--NAMESPACE-VAULT-BRAIN
    - SPEC--EPISODE-ATOM
created_at: 2026-05-14T01:55:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: "Architecture decision record"
---

# ADR — Brain Path Resolution

## Context

`[[CONCEPT--TWO-BRAIN-ARCHITECTURE]]` introduces two knowledge gravity wells (`~/.brain/` global + `<repo>/gks/` project). Without a deterministic routing function, every consumer of the memory store would re-implement "where do I look first?" — leading to divergent lookup orders and confused merge behaviour.

This ADR locks down the per-atom-type routing table. Future implementations (`resolver.ts`, MCP tools, CLI subcommands) MUST follow it.

## Decision

Adopt a deterministic, per-atom-type routing table that resolves every read/write between the two brains. Every consumer of the memory store MUST use this resolver; ad-hoc path construction is prohibited.

## Routing table (per atom type)

| Atom type | Read order | Write target |
|---|---|---|
| `ADR`, `FEAT`, `BLUEPRINT`, `AUDIT`, `CONCEPT` | project ONLY | project |
| `FRAMEWORK`, `SPEC`, `PROTOCOL` | project ONLY | project |
| `SKILL`, `ALGO`, `PROTO`, `PARAMS` | global → project (fallback) | global by default; project iff `vault_id` set |
| `EPISODE` | project ONLY | project |
| `IDENTITY`, `REGISTRY` | global ONLY | global |
| `MOD`, `MASTER` | project ONLY | project |
| `HOTFIX`, `INC`, `ISSUE` | project ONLY | project |

> **Amendment (2026-05-14).** `EPISODE` was originally classified global-only. It is now **project-only**. A dispatch episode records a task run *against a specific repo* — it is the reverse-path evidence the Meta-Learning Loop consumes (Code → AST → Symbol Graph → Execution Trace), which is project-scoped. The Phase D runtime (`result-recorder.ts`), the Phase D integration test, and the Phase F4 GC (`episode-gc.ts`, archives to `gks/episode/_archive/`) all already treated episodes as project-local; this amendment makes the routing table agree with three phases of implementation. The contradiction was tracked in `[[SPEC--EPISODE-ATOM]]` §7. Cross-machine agent run-history sync remains out of scope.

## Rationale per category

**Project-only types** = artefacts that describe *this codebase*. Putting them in `~/.brain/` breaks portability and pollutes other projects.

**Global-first, project-fallback** = reusable patterns. A `[[SKILL--CODE-REVIEW]]` written once in `~/.brain/skills/` should be usable in every project, but a project may shadow it with a `vault_id`-scoped variant.

**Global-only** = cross-project state that has no per-project meaning. `[[IDENTITY--AGENT-PROFILE]]` is the agent's persona; it doesn't change per repo. (`EPISODE` was originally placed here; the 2026-05-14 amendment moved it to project-only — see above.)

## Conflict resolution

If a global atom and a project atom have the same `id`, the project wins for **read** operations (it shadows global). For **write**, the resolver refuses and asks the caller to pick a `vault_id` — silent overwrites are a privacy hazard.

## File-system layout

```
~/.brain/                         (global, machine-local, NEVER in git)
├── identity.json
├── registry.yaml
├── skills/
│   ├── skill-creator/
│   └── code-review/
├── proto/
└── params/

<repo>/gks/                       (project, in git)
├── 00_index/                     (gitignored)
├── adr/ concept/ feat/ ...
├── episode/                      (dispatch episodes; _archive/ holds GC'd ones)
└── (NO .brain/ subdir — superseded by ~/.brain/ at home)
```

## Migration from prior idiom

`[[ADR--GLOBAL-VS-WORKSPACE]]` used `~/.msp/` and `<repo>/.brain/msp/projects/<ns>/`. The new resolution:

- `~/.msp/` → `~/.brain/`
- `<repo>/.brain/msp/projects/<ns>/` → `<repo>/gks/` (already done by PR #101 Phase B)
- Migration script `scripts/msp/init-brain.mjs` performs the `~/.msp/ → ~/.brain/` move (or copy on Windows where symlinks need admin).

## Consequences

- **Pro:** Single source of truth for "where does X live?" — no per-consumer divergence.
- **Pro:** Privacy: project secrets in atoms never leak to other repos.
- **Pro:** Cross-project learning: SKILLs and PROTOs accumulate in `~/.brain/` and benefit every future project.
- **Con:** Two physical locations to back up. Mitigated by: project = git, global = standard home backup.
- **Con:** Resolver introduces ~1 stat() call per lookup. Acceptable for a knowledge store; can cache via in-memory index.

## Out of scope

- How to sync `~/.brain/` across machines (manual / rsync / future tooling).
- Whether the global brain should be encrypted at rest (separate threat-model ADR).
- Per-tenant isolation within a single global brain (already covered by `vault_id`).

## Connections
- [[CONCEPT--NAMESPACE-VAULT-BRAIN]]

