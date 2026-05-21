---
id: CONCEPT--NAMED-PROJECT-REGISTRY
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: learned
title: Project = first-class — named registry resolved from CLI / env / .mspconfig
tags: &a1
  - msp
  - project
  - registry
  - cherry-pick
  - agent-agnostic
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--AGENT-AGNOSTIC
    - ADR--PATH-ENCODING
    - ADR--GLOBAL-VS-WORKSPACE
created_at: 2026-05-09T07:00:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--NAMED-PROJECT-REGISTRY
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: learned
  title: Project = first-class — named registry resolved from CLI / env / .mspconfig
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-09T07:00:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--NAMED-PROJECT-REGISTRY
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: learned
    title: Project = first-class — named registry resolved from CLI / env / .mspconfig
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-09T07:00:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--NAMED-PROJECT-REGISTRY
      phase: 1
      type: concept
      status: stable
      vault_id: default
      tier: genesis
      source_type: learned
      title: Project = first-class — named registry resolved from CLI / env /
        .mspconfig
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-09T07:00:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Strategic intent / PRD
      attributes:
        domain: concept
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — Named-project registry

> **Status: stable.** Core registry + 4-step resolver shipped in Phase B (`src/projects/{registry,resolve,types}.ts` + `msp_project_{list,register,resolve}` MCP tools — see `[[AUDIT--PHASE-B-IMPL-COMPLETE]]`). Cross-project recall (the `cross_project: true` flag described in §"Cross-project search" below) is the one remaining future extension; tracked as a small follow-up. Cherry-picked from `[[SPEC--ARCHITECTURE-V2]].md` §4.4 during the 2026-05-09 architecture-doc cleanup.

## Problem

Today, project namespace is effectively hardcoded — most code paths assume `evaAI` (per `[[ADR--PATH-ENCODING]]`). Agents that want to operate across multiple projects (clinic, MSP-self, EVA, custom) have no first-class way to switch. Every cognitive-layer client (Claude Code, Gemini CLI, Antigravity) needs its own ad-hoc cwd-based heuristic, and cross-project recall requires manual path juggling.

## Hypothesis

Treat **project** as a first-class concept resolved from a small, predictable chain. One YAML registry maps short names to filesystem paths + per-project settings (embedder, retrieval defaults). Agents pass a name, MSP resolves it.

## Registry shape

```yaml
# ~/.msp/projects.yaml  (global)
projects:
  default:
    path: ~/.msp/projects/default
    embedder: nomic-embed-text-v1.5
  eva:
    path: ~/eva
    embedder: nomic-embed-text-v1.5
  msp:
    path: ~/msp
    embedder: nomic-embed-text-v1.5
  clinic:
    path: ~/clinic
    embedder: nomic-embed-text-v1.5
```

## Resolution priority

1. Explicit: `--project=<name>` CLI flag or MCP tool argument
2. Env: `MSP_PROJECT=<name>`
3. File: `.mspconfig` in cwd or any ancestor (single line: `project: eva` or full TOML/YAML)
4. Fallback: `default`

The resolved name is looked up in `~/.msp/projects.yaml` to obtain the path + settings. If the name is unknown, MSP errors loudly rather than silently falling back — projects must be registered.

## Cross-project search (future extension)

`msp_recall { query, cross_project: true }` walks all registered projects' vector stores in parallel and merges results via RRF. Audit log records which project each hit came from. **Not yet implemented** — `msp_recall` currently scopes to the resolved project. Tracked as a follow-up to Phase B.

## Migration path

- Existing `evaAI` namespace → registered as `default` project (or `eva`, depending on naming preference).
- Old hardcoded paths read from a compatibility shim that delegates to the registry.
- `[[ADR--PATH-ENCODING]]` (bare-name decision) stays valid — the registry uses bare names too.

## Open questions

- Schema for `.mspconfig` — TOML, YAML, JSON, or single-line shorthand? Lean toward single-line shorthand for the common case + full file for the rare one.
- Per-project secrets (e.g. Anthropic key for cross-project consolidation) — registry value or external? External (per `[[ADR--GLOBAL-VS-WORKSPACE]]`).
- Identity/soul global vs per-project — **resolved** in `[[ADR--GLOBAL-VS-WORKSPACE]]`: global at `~/.msp/identity.json` with optional sparse per-project override at `.brain/msp/projects/<ns>/identity.override.json`.

## Why this fits the agent-agnostic vision

Per `[[CONCEPT--AGENT-AGNOSTIC]]`, every cognitive-layer client points at the same `msp-mcp-server` bin. The registry means **"which project"** is the only thing that varies between invocations from Claude Code, Gemini CLI, Antigravity, and custom agents. Switching projects becomes an env var, not a code change.

## Trade-offs

**Positive**

- Multi-project workflows become viable — clinic and EVA can share the same MSP install without confusion.
- `.mspconfig` lets a project declare itself without per-client config.
- Cross-project recall is a single flag, not a custom script.

**Negative**

- Adds a registration step ("register before use") — friction for one-off projects.
- Resolution chain is one more thing to debug when "the wrong project loaded".

## Source

`[[SPEC--ARCHITECTURE-V2]].md` §4.4 (drafted 2026-05-07, cherry-picked here on 2026-05-09 before the original was deleted). Coupled with the global/workspace separation work planned for Phase B. See `[[AUDIT--ARCH-DOC-CLEANUP]]`.

## Connections

- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
