---
id: CONCEPT--MSP-VALIDATOR
phase: 1
type: concept
status: stable
vault_id: default
title: MSP validator pipeline
tags:
  - msp
  - validator
  - gatekeeper
  - schema-enforcement
  - anti-hallucination
crosslinks: {"references":[]}
created_at: 2026-05-03T06:24:23.832Z
---

# CONCEPT — MSP validator pipeline

## Problem

GKS provides `proposeInbound()` as the single write-path to candidate atoms, but it only checks ID format and phase range. It does **not** enforce the schema, ID-uniqueness, wikilink integrity, or anti-hallucination rules that the master spec (`msp_spec.md` §4) requires. Without a layer above, agents can:

1. Forge identity fields (`commit_hash`, `validated_by`, `promotion_level`) — SSOT poisoning
2. Invent ADR numbers that collide with existing atoms — ID forgery
3. Reference `[[FEAT--ghost]]` that doesn't exist — dangling links
4. Set `created_at` in the future, omit required fields, or include `TODO`/`TBD` placeholders — content rot

These are exactly the four risks `msp_spec.md` §1 says MSP exists to prevent.

## Hypothesis

If MSP runs a validator over every inbound candidate **before** GKS's `inbound promote` is invoked, and the validator rejects on hard-rule violations (forbidden fields, ADR-number collisions, dangling wikilinks, future dates), then the canonical store under `gks/<type>/` retains its schema discipline regardless of how many agents (Claude/Gemini/SLM) write to it.

The validator must be:

- **Deterministic** — same input → same exit code, no LLM in the loop
- **Composable** — pre-commit hook + CI step + manual `npm run msp:validate`
- **Cheap** — runs on a single inbound file or the whole queue in <100ms typical
- **Self-describing** — every rejection includes file path, rule ID, and the offending field/value

## Scope

In:
- Frontmatter schema validation (required + conditional + forbidden fields)
- ADR-number monotonicity check against `gks/00_index/atomic_index.jsonl`
- Wikilink resolution (`[[X]]` → `X` exists in atomic index)
- ID-format regex enforcement
- Phase ↔ status compatibility check
- Anti-hallucination guards (no future dates, no `TBD`/`TODO` in `summary`)

Out (handled elsewhere):
- Content-quality review of decisions (human reviewer's job)
- Code-symbol existence verification (GitNexus / `verify-flow`'s job)
- Cross-tenant authorization (orchestrator's job)
- Embedding / vector search (GKS's job)

## Why this lives in MSP, not GKS

Per ADR-008 in GksV3, GKS is a *storage engine* and deliberately stays out of workflow / governance. ADR-009 makes MSP the orchestration layer. The validator is workflow — it decides what schema discipline a project enforces. Different projects pick different contracts; baking ours into GKS would couple them.
