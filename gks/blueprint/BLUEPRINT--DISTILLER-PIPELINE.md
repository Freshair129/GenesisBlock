---
id: BLUEPRINT--DISTILLER-PIPELINE
phase: 3
type: blueprint
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — The 4-Pillars memory distillation pipeline
tags: [msp, memory, distillation, 888, pipeline, plan]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - SPEC--MEMORY-888
    - CONCEPT--TIERED-MEMORY-DISTILLATION
linked_symbols:
  - file: packages/msp/src/orchestrator/distiller/index.ts
created_at: 2026-05-19T13:15:00+07:00
---

# BLUEPRINT — Distiller Pipeline

## 1. Goal

Implement the asynchronous background worker responsible for distilling memory across the 8-8-8 cycle tiers using the Four Pillars methodology.

## 2. Geography

- `packages/msp/src/orchestrator/distiller/`
  - `index.ts` (orchestrator entry point)
  - `pillar-clean.ts` (P1)
  - `pillar-summary.ts` (P2)
  - `pillar-index.ts` (P3)
  - `pillar-relation.ts` (P4)

## 3. Tasks

### T1: Pillar 1 — CLEAN

- Implement logic to deduplicate turns and remove low-impact metadata.
- Inputs: List of turn objects.
- Criterion: Strip greetings, boilerplate, and turns where `atom_impact` (from consolidator) is 0.

### T2: Pillar 2 — SUMMARY (LLM Integration)

- Implement prompts for Core and Sphere synthesis.
- Core Prompt: Synthesize narrative arc and identify concept clusters.
- Sphere Prompt: Synthesize identity-level beliefs and behavioral patterns.
- Integration: Use `createSlmClient` (Claude Sonnet for Core, Claude Opus for Sphere).

### T3: Pillar 3 — INDEX (Vector Store)

- Compute embeddings for the synthesized summary.
- Insert into the project's vector store under `memory/{core,sphere}` namespace.
- Ensure bitemporal timestamps are correctly set.

### T4: Pillar 4 — RELATION (GKS Output)

- For each distilled cluster/belief, produce a `NARRATIVE--` or `BELIEF--` atom.
- Link them back to the source evidence atoms via `crosslinks.references`.
- Call the MSP gatekeeper's internal `retain()` equivalent to write atoms to GKS.

### T5: Error and Pending Handling

- Implement the `.tmp` → `rename` logic from `BLUEPRINT--MEMORY-STORAGE-LAYOUT`.
- Ensure three-failure escalation (writing `FAILURE--DISTILLER-*` atom).

## 4. Verification

- [ ] `npm test` for each pillar independently.
- [ ] Synthetic end-to-end run: 8 Sessions $\rightarrow$ Distiller $\rightarrow$ Verify Core atom exists in `gks/narrative/`.
- [ ] Verify `linked_symbols` in the produced Narrative atom correctly point back to the source Session ULIDs.
