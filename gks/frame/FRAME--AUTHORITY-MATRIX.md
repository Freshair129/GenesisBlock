---
id: FRAME--AUTHORITY-MATRIX
phase: 0
type: frame
status: stable
vault_id: default
title: Authority matrix — who writes which path, by which channel
tags:
  - msp
  - authority
  - governance
  - foundation
crosslinks: {"references":["FRAME--MSP-ARCHITECTURE-V2"]}
created_at: 2026-05-03T07:01:49.281Z
---

# FRAME — authority matrix

Every path in the repo has exactly one legal write channel. Anything else is an authority violation and is blocked by the validator + pre-commit hook.

## Matrix

| Path | Direct write | Channel |
|---|---|---|
| `gks/adrs/`, `gks/algorithms/`, `gks/entities/`, `gks/features/`, `gks/flows/`, `gks/frameworks/`, `gks/modules/`, `gks/parameters/`, `gks/concepts/`, `gks/ideas/` | ❌ | `/submit-memory` → inbound queue → human review → promote |
| `gks/blueprints/` | ✅ T3 only (Claude/Opus) | direct edit, human review still required |
| `gks/microtasks/` | ✅ T2/T3 | acceptance tests gate execution |
| `gks/14_devlog/` | ✅ free-write | log per session |
| `src/` (auto-generated from microtasks) | ❌ | edit task YAML + rerun codegen |
| `src/` (hand-written, rare) | ✅ T3 only with ADR | direct edit + AUDIT-- afterwards |
| `CLAUDE.md`, `GEMINI.md`, `registry.yaml` | ❌ Boss-only | ask first |
| `.brain/msp/projects/<ns>/inbound/` | ✅ agents | drop proposal via `/submit-memory` |
| `.brain/msp/LLM_Contract/` | ❌ MSP maintainer only | code review |
| `gks/00_index/atomic_index.jsonl` | ❌ derived | `npm run msp:index` only |

## Tier definitions

| Tier | Who | Capability |
|---|---|---|
| **T1** | SLM (Qwen, Llama local) | execute microtasks under codegen contract |
| **T2** | Gemini | implementer; can write code + tasks but not ADRs |
| **T3** | Claude / Opus | architect; can write ADRs + Blueprints |
| **Boss** | Human owner | absolute authority over `CLAUDE.md`, `GEMINI.md`, `registry.yaml` |

## Enforcement points

| Where | What blocks |
|---|---|
| Pre-commit hook | direct write to `gks/<strict-tier>/` |
| `gks verify-flow` | tier writing above its rank (e.g. T1 trying to write ADR) |
| `gks inbound promote` | reviewer ≠ Boss for ADR/FEAT atoms |
| Manual code review | bypass attempts on `CLAUDE.md` etc. |

## Source

`msp_spec.md` §13 (Identity / Authority Matrix).
