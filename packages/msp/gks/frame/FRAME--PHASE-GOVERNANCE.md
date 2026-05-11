---
id: FRAME--PHASE-GOVERNANCE
phase: 0
type: frame
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Phase governance — P0..P6 doc-to-code flow
tags:
  - msp
  - phases
  - governance
  - doc-to-code
  - foundation
crosslinks: {"references":["FRAME--MSP-ARCHITECTURE-V2"]}
created_at: 2026-05-03T07:01:48.299Z
---

# FRAME — phase governance

MSP enforces a six-phase flow that every feature must traverse before code merges. The phases are mechanical — `gks verify-flow FEAT--<id>` walks the chain and refuses to exit-0 unless every required atom for the chosen scaling level is `stable`.

## Phases at a glance

| Phase | Activity | Primary atom | Devlog ID |
|---|---|---|---|
| **P0** | Frame the universe | `FRAME--`, `IDEA--` | — |
| **P1** | Define business need + technical draft | `CONCEPT--` | `MSP-CON-` |
| **P2** | Design structure + API spec | `ADR--`, `ENTITY--`, `API--`, `FEAT--` | `MSP-DES-` |
| **P3** | Plan deep code edits | `BLUEPRINT--` | `MSP-IMP-` |
| **P4** | Task decomposition | `T*.task.yaml` (microtask) | `MSP-TSK-` |
| **P5** | Real implementation | `src/` | `MSP-ACT-` (per turn) |
| **P6** | Acceptance test + walkthrough | `AUDIT--` | `MSP-WKT-` |

> **Note:** GKS 3.5.6 caps `phase` at 5 in the inbound queue. P6 atoms get filed at P5 until upstream alignment; tracked as M3c.

## P1 enforcement: technical feasibility

P1 `CONCEPT--` must include a **High-level API Draft** (the list of endpoints the concept implies). This prevents the failure mode where business approves a concept and P2 discovers it can't be built. See `CONCEPT--P1-TECH-FEASIBILITY` (TBD) for details.

## P2 enforcement: mandatory OpenAPI

P2 splits the high-level API draft into three atom types:
- `API--` — master OpenAPI hub
- `ENDPOINT--` — one path/method per file
- `ENTRYPOINT--` — auth/middleware logic

See `ADR--P2-MANDATORY-OPENAPI` (TBD).

## Devlog tracking

Every phase emits a devlog entry under `gks/14_devlog/` with `sessionId` for traceability. `MSP-WKT-` (P6 walkthrough) is the handover artifact between sessions; without it, the next agent has no audit trail.

## Hotfix exception

P1–P3 can be skipped only with a `HOTFIX` commit tag and a 48-hour backfill window. See `ADR--HOTFIX-ESCAPE-HATCH`.

## Source

`msp_spec.md` §6.4 (Devlog Tracking) + §6.2/§6.3 (P1/P2 enforcement) + §10.1 (Hotfix).
