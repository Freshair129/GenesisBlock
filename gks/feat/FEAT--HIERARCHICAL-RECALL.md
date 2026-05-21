---
id: FEAT--HIERARCHICAL-RECALL
phase: 2
type: feat
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: FEAT — Hierarchical Recall — multi-tier RRF fusion with Identity Preamble
tags: [msp, memory, retrieval, 888, recall, m11e]
aliases: [FEAT, implementation_flow, Feature specification]
cluster: implementation_flow
role: Feature specification
crosslinks:
  references:
    - CONCEPT--IDENTITY-EVOLUTION
    - SPEC--888-TIERED-MEMORY-DISTILLATION
    - ADR--RETRIEVAL-RRF-FUSION
created_at: 2026-05-20T11:15:00+07:00
---

# FEAT — Hierarchical Recall

## 1. Summary

The Hierarchical Recall feature upgrades the `msp_recall` pipeline to support the full 8-8-8 memory hierarchy. It integrates Narrative (Tier 2) and Identity (Tier 3) tiers into the RRF fusion process and introduces the "Identity Preamble" — a high-priority context injection mechanism for durable beliefs.

## 2. Motivation

Current retrieval treats all memory (Episodes) as having similar structural significance. As we move to a tiered model, we need the retrieval engine to recognize that a "distilled belief" (Identity) carries more factual weight than a "single observation" (Episode). Hierarchical Recall allows the agent to see the "Big Picture" (Narratives) while still having access to "Fine Detail" (Episodes) when needed.

## 3. Requirements

### 3.1 Multi-Tier RRF Fusion

- Extend `recall()` to query the `memory/narrative` and `memory/identity` vector namespaces.
- Implement tier-specific RRF weights:
  - **Identity Weight:** 1.8x (highest priority)
  - **Narrative Weight:** 1.4x (high priority)
  - **Episode Weight:** 1.0x (standard priority)
- Each hit returned must be tagged with its source `tier`.

### 3.2 Identity Preamble

- Provide a dedicated tool or option to fetch all `confirmed` Identity beliefs for the current namespace.
- These beliefs should be formatted as a "System Preamble" block that can be prepended to the LLM's system prompt.
- Avoid RRF filtering for the preamble; it should be deterministic based on the current `epistemic_state`.

### 3.3 Epistemic Filtering

- Retrieve only memory units in the `confirmed` or `hypothesis` states by default.
- **Contested** units receive a 0.2x penalty multiplier.
- **Deprecated** units are strictly excluded from results.

### 3.4 MCP Extension

- Update `msp_recall` output to include the `tier` field on each hit.
- Add `msp_identity_beliefs` tool to fetch the current preamble.

## 4. Acceptance Criteria

- [ ] `msp_recall` successfully returns a mixture of Episode, Narrative, and Identity hits.
- [ ] Identity hits consistently rank higher than equivalent semantic hits from lower tiers.
- [ ] Preamble tool returns only `confirmed` beliefs.
- [ ] RRF results correctly show the `tier` of each document.

## 5. Connections

- `[[CONCEPT--IDENTITY-EVOLUTION]]` — the high-level intent.
- `[[SPEC--888-TIERED-MEMORY-DISTILLATION]]` §9.3 — Technical specification.
- `[[ADR--RETRIEVAL-RRF-FUSION]]` — the algorithm being extended.
