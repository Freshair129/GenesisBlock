---
id: AUDIT--ARCH-DOC-CLEANUP
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Architecture-doc cleanup — declared MSP agent-agnostic; removed 3 overlapping root specs
tags:
  - msp
  - audit
  - cleanup
  - agent-agnostic
  - ssot
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--AGENT-AGNOSTIC","CONCEPT--MSP-OBSERVE-HOT-PATH","CONCEPT--NAMED-PROJECT-REGISTRY"]}
linked_symbols: []
created_at: 2026-05-09T07:00:00.000+07:00
---

# AUDIT — Architecture-doc cleanup (2026-05-09)

## Scope

The MSP repo had accumulated **seven** overlapping architecture documents framing MSP four different ways (passport / gatekeeper / cognitive-concerns / centralized-retrieval-service). The codebase reality matched none of them exactly. After auditing each doc against `src/`, `package.json`, the GKS repo (`Freshair129/GksV3`), and the running MCP server, we cut the surface to **two** SSOT documents and cherry-picked salvageable ideas into atoms.

## Mental model that drove the cleanup

```
COGNITIVE LAYER (agents)        EVA  │  Hermes  │  openclaw  │  Claude Code  │  Gemini CLI  │  Antigravity
        │ uses
        ▼
MEMORY OS                       MSP (this repo)
        │ uses
        ▼
KNOWLEDGE BASE                  GKS (@freshair129/gks)
```

Per GKS `SCOPE.md` line 135, `FRAMEWORK_MASTER_SPEC.md` is "from the EVA project" — i.e. EVA's cognitive-layer spec. Keeping it in the MSP repo coupled MSP to EVA semantics, defeating the agent-agnostic intent. See `[[CONCEPT--AGENT-AGNOSTIC]]`.

## Actions taken

### Removed

| File | Why |
|---|---|
| `CORE_FRAMEWORK_MASTER_SPEC.md` (1264 lines) | EVA's framework spec; not MSP's. Self-declared "boilerplate, fork and replace YourProject". Contradicted `msp_spec.md` on path encoding (`D--ProA` vs bare-name per `[[ADR--PATH-ENCODING]]`), tool count (claimed 11; codebase has 16), bin count (claimed 5; package.json has 6). |
| `msp_infra_startup_architecture.md` (161 lines) | Described a Fastify + Postgres + Redis + pgvector + BullMQ service that does not exist in the codebase. Mutually exclusive with the Obsidian-runtime architecture in `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`. Zero crosslinks pointed back to it. |
| `[[SPEC--ARCHITECTURE-V2]].md` (660 lines) | Proposal for cognitive-concerns reframe + GKS-as-library, pending review since 2026-05-07. Incompatible with the 3-layer mental model (EVA / MSP / GKS) confirmed during this cleanup. Salvageable ideas extracted into 3 CONCEPT atoms (see "Cherry-picked" below). |

### Cherry-picked into new atoms

| Source | New atom | Status |
|---|---|---|
| `[[SPEC--ARCHITECTURE-V2]].md` §4.2 (mem0-style hot-path extraction) | `[[CONCEPT--MSP-OBSERVE-HOT-PATH]]` | draft (aspirational) |
| `[[SPEC--ARCHITECTURE-V2]].md` §4.4 (project as first-class) | `[[CONCEPT--NAMED-PROJECT-REGISTRY]]` | draft (aspirational) |
| `[[SPEC--ARCHITECTURE-V2]].md` §4.5 (multi-client global install) | folded into `[[CONCEPT--AGENT-AGNOSTIC]]` | stable |

The cognitive-concerns reframe (§2 of SPEC v2), MSP-only-MCP-layer claim (§4.1), and satellite-package split (§4.6) were rejected — they conflict with the agent-agnostic 3-layer model.

### Patched

- `msp_spec.md` — version bumped 2.0.1 → 2.0.2; references to `FRAMEWORK_MASTER_SPEC.md` (4 occurrences) replaced with the actual SSOT atoms (`[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`, `[[FRAMEWORK--PHASE-GOVERNANCE]]`, `[[CONCEPT--RETRIEVAL-ORCHESTRATION]]`); references table now points at the new SSOT.
- `gks/frame/[[FRAMEWORK--MSP-ARCHITECTURE]].md` — v1 superseded; banner strengthened with explicit "DO NOT USE AS REFERENCE" warning + note that the inbound queue described in its body was removed in Phase 3 (see `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]`).

### Not changed (deferred)

- `gks/frame/[[FRAMEWORK--MSP-ARCHITECTURE]].md` left in `gks/frame/` (not moved to `gks/archive/`) — moving would require validator + indexer work and risks breaking the supersede chain. Banner makes intent clear.
- Global / workspace separation (`~/.msp/` vs project) — Phase B follow-up. Tracked via `[[CONCEPT--NAMED-PROJECT-REGISTRY]]` open questions.
- GKS upstream `docs/MSP_RELATIONSHIP.md` is outdated (still describes inbound queue). Phase D follow-up — file an upstream issue.

## Final SSOT shape

| Doc | Role |
|---|---|
| `gks/frame/[[FRAMEWORK--MSP-ARCHITECTURE-V2]].md` | Architecture SSOT (passport-orchestrator, 3-layer model) |
| `msp_spec.md` v2.0.2 | Technical full spec |
| `gks/concept/[[CONCEPT--AGENT-AGNOSTIC]].md` | MSP/agent boundary contract |

All other architecture docs are atoms under `gks/<type>/` reachable via crosslinks from the above three.

## Verification

- `npm run msp:index` — 196 atoms indexed, 0 duplicates
- `npx tsx src/validator/cli.ts --all` — 196 passed, 0 failed (PROTO failures pre-existing, unrelated)
- `npm run msp:check-links` — OK

## Counts

| Before | After | Delta |
|---|---|---|
| 7 root architecture docs | 2 (`msp_spec.md`, `ROADMAP.md`) | −5 |
| 1 active FRAME atom + 1 v1 superseded | same (banner strengthened) | 0 |
| 0 agent-agnostic concept atoms | 3 (AGENT-AGNOSTIC + 2 cherry-picks) | +3 |

## Source

Architecture audit conducted via subagent on 2026-05-09 against the codebase, `Freshair129/GksV3` (`/tmp/gks` clone), `@freshair129/gks@3.6.0` published `dist/`, and the 7 candidate docs. Decision driven by GKS `SCOPE.md` line 135 ("FRAMEWORK_MASTER_SPEC.md from the EVA project") and the user's clarification that EVA is the cognitive layer, MSP is the memory OS, GKS is the knowledge base — pluggable independently.
