---
id: ADR--MSP-CANDIDATE-CLI
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: msp-candidate CLI — non-MCP agent path to MSP; no direct GKS contact; no
  SKILL.md for core features
tags:
  - msp
  - agent
  - cli
  - gemini
  - qwen
  - boundary
crosslinks:
  references:
    - CONCEPT--KNOWLEDGE-LAYERS-V2
    - ADR--AGENT-WRITE-BOUNDARIES
    - ADR--MSP-INTERFACE-LAYER
    - ADR--MSP-MCP-SERVER
    - ADR--GEMINI-AS-SLM-PROVIDER
created_at: 2026-05-17T00:00:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — msp-candidate CLI (non-MCP agent path)

## Context

`[[ADR--AGENT-WRITE-BOUNDARIES]]` establishes that agents write to `.brain/` only via MSP, never directly to `gks/`. The enforcement tool for MCP-capable agents (Claude) is the `msp_candidate` MCP tool.

However, this repo runs three agent tiers:

| Tier | Agent | MCP support |
|---|---|---|
| T3 | Claude Code | ✅ full MCP |
| T2 | Gemini CLI | ❌ no MCP |
| T1 | Qwen CLI | ❌ no MCP |

Before this ADR, the only documented non-MCP path was `gks propose-inbound` CLI — a GKS-level command that was deprecated in Phase 3 (2026-05-09). Gemini and Qwen had no compliant write path, creating a gap that violated the MSP-as-sole-gateway principle.

Additionally, the `atom-creator` skill (`atom-creator/SKILL.md` + `atom-creator/scripts/create_atom.cjs`) wrote atoms **directly to `gks/<type>/`**, bypassing both MSP and the inbound queue entirely — a hard violation of `[[ADR--AGENT-WRITE-BOUNDARIES]]`.

## Decision

### 1. `msp-candidate` CLI is the authoritative non-MCP agent path

A new CLI binary `msp-candidate` is added to `packages/msp/`:

```
packages/msp/src/memory/candidates/cli.ts
packages/msp/bin: "msp-candidate" → dist/memory/candidates/cli.js
```

It wraps `CandidateWriter` — the same class used by the `msp_candidate` MCP tool — ensuring both paths produce identical output under `.brain/msp/projects/<ns>/candidates/`.

```bash
# Gemini / Qwen usage
msp-candidate propose \
  --id=FEAT--MY-FEATURE \
  --type=feat \
  --title="My Feature" \
  --body="..." \
  --root=.
```

### 2. Agent interface matrix (extended from ADR--AGENT-WRITE-BOUNDARIES)

| Agent | Atom proposal path | Direct GKS write |
|---|---|---|
| Claude (T3) | `msp_candidate` MCP tool | ❌ forbidden |
| Gemini (T2) | `msp-candidate` CLI | ❌ forbidden |
| Qwen (T1) | `msp-candidate` CLI | ❌ forbidden |
| Human | PR to `gks/` | ✅ via PR + CI only |

### 3. `gks propose-inbound` is forbidden for agent use

`gks propose-inbound` (and `gks new-feature`) are GKS-internal CLIs. Agents must not call them. They bypass the MSP layer and were the legacy path before `msp_candidate` existed. References to these commands in `GEMINI.md` and `qwen.md` must be removed or replaced with `msp-candidate`.

### 4. Core features use MCP tools + CLI only — no SKILL.md

SKILL.md files are **not used for core system features** in this repo. The rationale:

- MCP tool schemas enforce correctness at call time (schema validation, type checking)
- SKILL.md relies on natural language instruction — higher hallucination risk, higher token cost
- MCP + CLI paths are compatible with all three agent tiers; SKILL.md is Claude Code-only
- SKILL.md is appropriate for reasoning guidance, style, and complex decision workflows — not for structured system operations

SKILL.md files may exist for **developer ergonomics** (e.g., onboarding, workflow guides) but must not be the implementation path for any operation that has an MCP tool or CLI equivalent.

### 5. `atom-creator` skill is deprecated

The following files are removed as they violate `[[ADR--AGENT-WRITE-BOUNDARIES]]`:

- `atom-creator/SKILL.md` — superseded by `msp-candidate` CLI
- `atom-creator/scripts/create_atom.cjs` — wrote directly to `gks/`, hard violation
- `atom-creator/references/taxonomy.md` — taxonomy reference is canonical in `CONCEPT--TAXONOMY-V2-3`

The `atom-creator/` directory is removed entirely.

## Enforcement

- `GEMINI.md` and `qwen.md` updated to reference `msp-candidate propose` as the atom creation path
- `atom-creator/` directory removed from repo
- Pre-commit hook already gates direct `gks/` writes (unchanged from `[[ADR--AGENT-WRITE-BOUNDARIES]]`)

## Consequences

### Positive

- **Single gateway.** All agent tiers now route through MSP for atom proposals — no exceptions.
- **No legacy confusion.** `gks propose-inbound` deprecation is fully enforced at the agent guidance level.
- **Token efficient.** MCP + CLI require no SKILL.md loading overhead.
- **Schema-validated.** Both MCP tool and CLI share `CandidateWriter` — one implementation, two interfaces.

### Negative

- **CLI maintenance.** `msp-candidate` CLI must be kept in sync with `CandidateWriter` API changes. Mitigation: they share the same class; breaking changes to the writer will surface as TypeScript compile errors in the CLI.

### Neutral

- **No change to canon promotion.** Human PR review for `gks/` remains unchanged per `[[ADR--AGENT-WRITE-BOUNDARIES]]`.

## Alternatives considered

### A. Give Gemini/Qwen MCP access via bridge

Route non-MCP agents through an MCP bridge process that translates shell calls to MCP protocol.

**Rejected because:** adds infrastructure complexity and a new failure point. The CLI wrapping `CandidateWriter` achieves identical semantics with zero extra infrastructure.

### B. Keep `gks propose-inbound` as the non-MCP path

Document `gks propose-inbound` in `GEMINI.md` as the Gemini/Qwen path.

**Rejected because:** it bypasses MSP entirely. MSP's audit log, namespace layering, and future middleware hooks would not apply. Contradicts the MSP-as-Memory-OS architectural goal.

### C. Use SKILL.md to instruct Gemini to call `msp-candidate`

Wrap the CLI call in a SKILL.md for Gemini, leveraging skill-creator framework.

**Rejected because:** circular — if the agent can decide to invoke the skill, it can call the CLI directly. SKILL.md adds token overhead for no enforcement benefit. The CLI reference in `GEMINI.md` achieves the same result without runtime token cost.

## Source

- `[[ADR--AGENT-WRITE-BOUNDARIES]]` — the bright-line boundary this ADR extends
- `[[ADR--MSP-INTERFACE-LAYER]]` — MSP as orchestration layer
- `packages/msp/src/memory/candidates/cli.ts` — implementation
- Conversation 2026-05-17 — architectural review that identified the Gemini/Qwen gap
