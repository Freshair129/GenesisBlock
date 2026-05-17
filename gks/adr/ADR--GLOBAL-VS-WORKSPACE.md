---
id: ADR--GLOBAL-VS-WORKSPACE
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Global vs workspace storage — ~/.msp for cross-project,
  .brain/msp/projects/<ns>/ for per-project
tags:
  - msp
  - storage
  - global
  - workspace
  - agent-agnostic
  - decision
crosslinks:
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--AGENT-AGNOSTIC
    - CONCEPT--NAMED-PROJECT-REGISTRY
    - ADR--PATH-ENCODING
created_at: 2026-05-09T07:00:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — global vs workspace storage

## Context

`[[CONCEPT--AGENT-AGNOSTIC]]` declares MSP must be pluggable into any cognitive-layer client (Claude Code, Gemini CLI, Antigravity, Hermes, openclaw, EVA). Each of those clients already follows a **global root + per-project workspace** pattern:

| Client | Global root | Per-project |
|---|---|---|
| Claude Code | `~/.claude/` | `./.claude/` |
| Gemini CLI | `~/.gemini/` | (project root) |
| EVA | `~/.eva/` | per-project workspace |
| git (analogous) | `~/.gitconfig` | `./.git/config` |

Today, MSP has **no global root**. All state lives in workspace under `.brain/msp/projects/<namespace>/` (per `[[ADR--PATH-ENCODING]]`). That layout works for a single-project agent, but breaks the moment a user runs MSP from two cwd's with the same identity:

- Identity (`name`, `role`, `voice`, `preferences`) is duplicated per-project — change in one workspace doesn't reach the other.
- No place to register multiple projects (`[[CONCEPT--NAMED-PROJECT-REGISTRY]]` needs `~/.msp/projects.yaml`).
- Cross-project audit / search has nowhere to live.

We need to decide what's global, what's workspace, and how the resolver merges them.

## Decision

### Storage split

| Concern | Location | Rationale |
|---|---|---|
| **Identity (profile, voice)** | Global `~/.msp/identity.json` | User identity is user-level, not project-level. Same human across projects. |
| **Preferences** (default model, embedder choice, retrieval defaults) | Global `~/.msp/preferences.json` | Apply across projects; per-project override allowed. |
| **Projects registry** | Global `~/.msp/projects.yaml` | Per `[[CONCEPT--NAMED-PROJECT-REGISTRY]]`. |
| **Cross-project audit** | Global `~/.msp/audit/<date>.jsonl` | Records cross-project recall, identity edits, registry changes. |
| **Sessions** (per-turn JSONL) | Workspace `./.brain/msp/projects/<ns>/sessions/*.jsonl` | Conversation is per-project by definition. |
| **Episodic memory** | Workspace `./.brain/msp/projects/<ns>/memory/episodic_memory.json` | Project-specific context. |
| **Candidates** (`msp_candidate` writes) | Workspace `./.brain/msp/projects/<ns>/candidates/` | Candidates target the project's `gks/<type>/`. |
| **Vector / backlinks** | Workspace `./.brain/msp/projects/<ns>/vector/` | Computed from project atoms. |
| **Per-project identity override** (optional) | Workspace `./.brain/msp/projects/<ns>/identity.override.json` | Project-specific voice/role for clinic-vs-EVA-vs-self contexts. |

### Resolution rules

**Identity** (read order, last write wins for the resolved view):

1. `~/.msp/identity.json` — global default
2. `./.brain/msp/projects/<ns>/identity.override.json` — per-project override (sparse; only fields user explicitly overrode)
3. Result: shallow-merged `Identity` object passed to the agent.

**Writes** target the layer that owns the field:

- `msp_identity_set { scope: 'global' | 'project', key, value }` — explicit scope; default `global` for `profile`/`voice`, `project` for `preferences` overrides. Errors if scope/key combination is invalid.

**Projects registry**:

- Resolved per `[[CONCEPT--NAMED-PROJECT-REGISTRY]]` (CLI flag → env → `.mspconfig` → fallback `default`).
- Registry path is fixed at `~/.msp/projects.yaml` (or `$MSP_HOME/projects.yaml` if `MSP_HOME` env is set, for testing / sandbox).

### `MSP_HOME` env

`MSP_HOME` defaults to `~/.msp/`. Tests, CI, and multi-account setups can override (e.g. `MSP_HOME=/tmp/msp-test/`). Behaviour mirrors `GIT_CONFIG_GLOBAL` and similar.

### Why not XDG (`~/.config/msp/`)?

XDG-compliance is the technically-correct answer on Linux, but:

1. The cognitive-layer clients we target (Claude Code, Gemini CLI, Antigravity) all use `~/.<name>/`, not XDG. Matching them reduces user surprise.
2. Cross-platform: macOS doesn't follow XDG conventions, Windows doesn't have XDG at all.
3. Users can symlink `~/.msp` → `~/.config/msp` if they prefer XDG; we don't enforce.

If the user objects, the migration to XDG is one config flag (`MSP_HOME`); no code change.

## Migration

Existing workspaces have `./.brain/msp/projects/<ns>/identity.json`. After this ADR lands and the implementation ships:

1. **First read** of identity:
   - If `~/.msp/identity.json` exists → use as global.
   - Else if `./.brain/msp/projects/<ns>/identity.json` exists → migrate it to `~/.msp/identity.json` (one-time copy + log warning on stderr) and treat as global from now on.
   - Else → write defaults to `~/.msp/identity.json`.
2. **Old workspace `identity.json`** is preserved (not deleted) but marked deprecated; a future major version may remove the migration shim.
3. **Existing `evaAI` namespace** — registered as `default` project in `~/.msp/projects.yaml` automatically on first run if registry is empty.

Migration code lives in `src/identity/migrate.ts` (to be written per `[[BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION]]`); runs idempotently on every `readIdentity` until both sides agree.

## Consequences

**Positive**
- Identity travels with the user, not the project — matches mental model of every other CLI agent.
- Multi-project usage works without config rewrite.
- Cross-project audit log enables features like "what have I asked across all projects this week".
- `MSP_HOME` env makes test isolation trivial (no need to fake `$HOME`).

**Negative**
- Two-tier read path is one more thing to debug ("which layer won?"). Mitigated by `msp_identity_get { explain: true }` showing the resolution chain.
- Migration window — old workspaces with `identity.json` get auto-migrated, which changes user-visible state without explicit consent. Mitigated by stderr warning + not deleting the old file.
- Cross-platform `~` resolution differs (Windows: `%USERPROFILE%\.msp\`). Use `os.homedir()` from Node, not string concat.

## Alternatives considered

1. **Identity per-project always** — what we have today. Rejected: doesn't scale to multi-project users; defeats `[[CONCEPT--AGENT-AGNOSTIC]]`.
2. **Identity global only, no override** — simpler. Rejected: a user might want different voice ("formal Thai") for clinic-project vs casual for self-project. Override is opt-in (only created if user calls `msp_identity_set --scope=project`).
3. **XDG-only paths** — Linux-correct. Rejected: cross-platform inconsistency; doesn't match peer clients (`~/.claude`, `~/.gemini`).
4. **Sessions in global too** (cross-project session log) — Rejected: conversations are project-scoped; merging them creates noise. Cross-project audit (which does live globally) is the right primitive for "what did I do across projects".

## Source

Phase B of architecture-doc cleanup (2026-05-09). Driven by `[[CONCEPT--AGENT-AGNOSTIC]]` requirement that MSP plug into any cognitive-layer client. `~/.claude` / `~/.gemini` / `~/.eva` patterns are direct references. Path resolution mirrors `git`'s global-vs-local config model.

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

