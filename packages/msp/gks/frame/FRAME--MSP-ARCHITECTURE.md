---
id: FRAME--MSP-ARCHITECTURE
phase: 0
type: frame
status: superseded
vault_id: default
tier: genesis
source_type: axiomatic
title: MSP architecture — gatekeeper layers + write flow
tags:
  - msp
  - architecture
  - gatekeeper
  - foundation
  - superseded
crosslinks:
  references: []
  superseded_by: [FRAME--MSP-ARCHITECTURE-V2]
created_at: 2026-05-03T15:01:47.791+07:00
---

> ⚠️ **SUPERSEDED — DO NOT USE AS REFERENCE** ⚠️
>
> This v1 FRAME is preserved for crosslink-chain integrity only. Do not read its body for current behavior.
>
> - **Active SSOT**: [`FRAME--MSP-ARCHITECTURE-V2`](FRAME--MSP-ARCHITECTURE-V2.md) — passport-orchestrator framing
> - **Inbound queue (mentioned below) was REMOVED in Phase 3** (2026-05-09); use `msp_candidate` MCP tool instead — see `BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION` and `ADR--AGENT-WRITE-BOUNDARIES`
> - **MSP is now agent-agnostic** — see `CONCEPT--AGENT-AGNOSTIC`
>
> Body preserved verbatim below for historical reference; treat every claim as out-of-date.

# FRAME — MSP architecture (v1, superseded)

MSP sits between agents (T1/T2/T3 — Claude / Gemini / SLM) and the canonical knowledge store under `gks/`. It refuses any direct write to `gks/`; the only legal path is the inbound queue.

## Write flow (one direction)

```
Agent → /submit-memory → .brain/msp/projects/<ns>/inbound/
         → MSP validator → human review → gks/<type>/
                                          + atomic_index.jsonl
```

Each arrow is a hard gate. Bypass attempts are caught at:

| Gate | Mechanism |
|---|---|
| Direct write to `gks/` | filesystem ACL + git pre-commit hook |
| Schema violation | MSP validator (forbidden fields, ID format, dangling wikilinks, …) |
| Wrong reviewer | promote workflow requires explicit human approval |
| Missing chain | `gks verify-flow FEAT--<id>` exits 1 if any link is broken |

## Layers

| # | Layer | Owner | Path | What |
|---|---|---|---|---|
| 1 | Contract | MSP maintainer | `.brain/msp/LLM_Contract/` | YAML schemas the validator reads |
| 2 | Inbound | Agents (write), MSP (read) | `.brain/msp/projects/<ns>/inbound/` | candidate atoms + `.rev-XXXX` files |
| 3 | Rejected | MSP | `.brain/msp/projects/<ns>/rejected/{date}/` | failed proposals + reason |
| 4 | Canonical | MSP (write via promote) | `gks/<type>/` | committed SSOT |
| 5 | Index | MSP (re-indexer) | `gks/00_index/atomic_index.jsonl` | derived; never hand-edit |
| 6 | Sessions | MSP | `.brain/msp/projects/<ns>/sessions/*.jsonl` | linear conversation history |
| 7 | Episodic | MSP | `.brain/msp/projects/<ns>/memory/episodic_memory.json` | rich event summaries |
| 8 | Vector / Backlinks | MSP | `.brain/msp/projects/<ns>/vector/` | hybrid retrieval edges |

## What this frame does NOT define

- *How* atoms are validated → see `CONCEPT--ATOMIC-WRITE-CONTRACT`, `ADR--ANTI-HALLUCINATION-RULES`
- *When* phases gate code writes → see `FRAME--PHASE-GOVERNANCE`
- *Who* can edit which path → see `FRAME--AUTHORITY-MATRIX`
- The exact crosslink predicates → see `FRAME--CROSSLINKS-VOCABULARY`

## Source

`msp_spec.md` §2 (Architecture Overview).
