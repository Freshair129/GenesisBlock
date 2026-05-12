---
id: CONCEPT--AGENT-AGNOSTIC
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: MSP is agent-agnostic — pluggable into EVA, Claude Code, Gemini CLI, Hermes, Antigravity
tags:
  - msp
  - agent-agnostic
  - architecture
  - foundation
  - integration
crosslinks: {"references":["FRAME--MSP-ARCHITECTURE-V2","CONCEPT--OBSIDIAN-AS-RUNTIME"]}
created_at: 2026-05-09T08:00:00.000+07:00
---

# CONCEPT — MSP is agent-agnostic

## Problem

The Freshair ecosystem has three distinct layers:

```
COGNITIVE LAYER (agents)        EVA / Hermes / openclaw / Claude Code / Gemini CLI / Antigravity
        │ uses
        ▼
MEMORY OS                       MSP (this repo) — passport, sessions, episodic, identity, retrieval
        │ uses
        ▼
KNOWLEDGE BASE                  GKS (@freshair129/gks) — atomic / vector / episodic / obsidian / graph
```

Historically MSP grew out of EVA's Memory OS needs. Some atoms still carry EVA-shaped vocabulary (`MSP-IMP-` / `MSP-TSK-` / `MSP-WKT-` process IDs, references to `FRAMEWORK_MASTER_SPEC`, etc.). That coupling hurts pluggability — agents that aren't EVA inherit EVA's cognitive-model assumptions for free, which is wrong.

## Hypothesis

MSP must be **agent-agnostic** at the contract level. EVA is one consumer; Claude Code, Gemini CLI, Antigravity, openclaw, and any future agent must be able to plug in without inheriting EVA-specific semantics.

## What this means concretely

### 1. MSP exposes one MCP surface; agents plug in identically

Each cognitive-layer client points to the same global bin:

```jsonc
// Claude Code (~/.claude/mcp.json or .claude/settings.json)
{ "mcpServers": { "msp": { "command": "msp-mcp-server" } } }

// Gemini CLI (~/.gemini/config.json)
{ "mcpServers": { "msp": { "command": "msp-mcp-server" } } }

// Antigravity / Codex / custom agent
[servers.msp]
command = "msp-mcp-server"
```

Project resolution from cwd / env / `.mspconfig` — same logic regardless of which agent invokes the bin. See `CONCEPT--NAMED-PROJECT-REGISTRY` for the resolution contract.

### 2. EVA-specific concerns live in EVA, not MSP

MSP does **not** own:

- `MSP-IMP-` / `MSP-TSK-` / `MSP-WKT-` process artifact IDs (these are EVA's `registry.yaml`, per GKS `docs/MSP_RELATIONSHIP.md`)
- The 3-tier knowledge model from `FRAMEWORK_MASTER_SPEC.md` (EVA cognitive-layer spec)
- RI levels, RMS affect, EVA-specific epistemic workflow
- "Master Block" / `msp:master compose` loader (EVA orchestration)

When MSP needs a process-ID convention, it lives in `gks/concept/` of the consuming project, not as MSP core.

### 3. Identity/voice/preferences belong to the agent's user, not the agent type

`identity.json` schema is universal: `name`, `role`, `voice`, `preferences`, `guardrails`. EVA's "tier" / "stakes-level" notions are **not** in MSP's identity schema; they live in EVA's Memory OS layer if needed.

## What MSP still owns (agent-agnostic)

- **Sessions** (per-turn JSONL log, per-project)
- **Episodic memory** (importance-scored summaries)
- **Identity / soul** (single JSON, schema-stable)
- **Retrieval orchestration** (RRF over GKS sources)
- **Context compression** (token-budget summarisation)
- **Validator** (atom shape, anti-hallucination, shift-left wikilink check)
- **Candidate → PR pipeline** (`msp_candidate` MCP tool → `.brain/.../candidates/`)

Everything above is independent of which agent is on top. EVA's biological consolidation cascade, Claude Code's slash commands, Gemini CLI's tool framework — all consume the same MSP primitives.

## Trade-offs

**Positive**
- Any agent that speaks MCP gets a passport for free.
- Cleaner mental model — easier to onboard contributors who don't know EVA.
- MSP can evolve without breaking EVA (and vice versa).

**Negative**
- Some current atoms reference EVA concepts and need cleanup over time (tracked via `AUDIT--ARCH-DOC-CLEANUP` and follow-up audits).
- "Universal" surface means we can't bake EVA-specific optimisations into MSP core; they live as plugins or in EVA itself.

## Source

Architectural clarification 2026-05-09 — established that MSP is the Memory OS sandwiched between agents (cognitive layer) and GKS (knowledge base). Cherry-picked from `SPEC--ARCHITECTURE-V2.md` §2, §4.5 (multi-client global install pattern). See `AUDIT--ARCH-DOC-CLEANUP`.
