---
id: BLUEPRINT--IDENTITY-EVOLUTION-ROLLOUT
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Identity Evolution and Hierarchical Recall Implementation Plan
tags: [msp, memory, identity, recall, rollout, plan, m11e]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - FEAT--HIERARCHICAL-RECALL
    - SPEC--888-TIERED-MEMORY-DISTILLATION
linked_symbols:
  - file: packages/msp/src/orchestrator/retrieval/index.ts
created_at: 2026-05-20T11:30:00+07:00
---

# BLUEPRINT — Identity Evolution Rollout

## 1. Goal

Implement the final stage of the 8-8-8 memory protocol: hierarchical retrieval across all memory tiers and the injection of durable beliefs into the agent's identity preamble.

## 2. Implementation Steps

### T1: Memory Tier Identification (`packages/msp/src/orchestrator/retrieval/types.ts`)

- Update `RetrievalHit` type to include an optional `tier` field.
- Enum: `'trace' | 'episode' | 'narrative' | 'identity' | 'atom'`.

### T2: RRF Source Expansion (`packages/msp/src/orchestrator/retrieval/index.ts`)

- Update the `recall()` orchestrator to include two new internal sources: `narrativeSource` and `identitySource`.
- Use the `gksLayout` to resolve paths to `.brain/.../memory/narrative/` and `~/.msp/memory/identity/`.
- Ensure these sources participate in the `rrfFuse` step with their respective weights (1.4x and 1.8x).

### T3: Identity Belief Loader (`packages/msp/src/memory/identity.ts`)

- Implement `loadIdentityBeliefs(ns: string): Promise<IdentityBelief[]>`.
- Logic:
    1. Read all JSON files in `~/.msp/memory/identity/`.
    2. Filter for beliefs marked as `confirmed`.
    3. Return as a flat list.

### T4: Preamble Generator (`packages/msp/src/orchestrator/persona/preamble.ts`)

- Create a generator that converts `IdentityBelief[]` into a Markdown string.
- Format:

    ```markdown
    ### Long-Term Identity Beliefs
    - [Belief Statement 1] (Confidence: 0.92)
    - [Belief Statement 2] (Confidence: 0.85)
    ```

- This string will be used by the main agent loop to hydrate its system prompt.

### T5: MCP Tool Finalization

- Register the new MCP tools specified in `SPEC--888-TIERED-MEMORY-DISTILLATION §12`.
- `msp_distill`: Calls the functional orchestrator from Phase B/C.
- `msp_identity_beliefs`: Calls the new preamble generator (T4).

## 3. Verification Plan

### 3.1 Tiered Retrieval Test

- Seed a namespace with 1 Episode, 1 Narrative, and 1 Identity belief.
- Perform a query.
- Verify the output hits show the correct `tier` label and are sorted according to hierarchical weights.

### 3.2 Preamble Verification

- Call `msp_identity_beliefs` for a namespace.
- Verify only `confirmed` beliefs appear and the formatting is correct for LLM ingestion.

### 3.3 End-to-End Soul Evolution

- Complete a full 8-session cycle manually.
- Trigger `msp_distill`.
- Verify a new `BELIEF--*` atom is created and subsequently appears in the agent's preamble.
