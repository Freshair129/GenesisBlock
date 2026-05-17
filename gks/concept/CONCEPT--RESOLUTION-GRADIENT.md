---
id: CONCEPT--RESOLUTION-GRADIENT
phase: 1
type: concept
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Resolution gradient — graded retrieval with FULL / MENTION + expand-on-demand
attributes:
  id: CONCEPT--RESOLUTION-GRADIENT
  phase: 1
  type: concept
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Resolution gradient — graded retrieval with FULL / MENTION + expand-on-demand
  attributes:
    id: CONCEPT--RESOLUTION-GRADIENT
    phase: 1
    type: concept
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Resolution gradient — graded retrieval with FULL / MENTION +
      expand-on-demand
    attributes:
      domain:
        - ucf
        - msp
    tags: &a1
      - msp
      - ucf
      - concept
      - retrieval
      - resolution
      - tokens
      - context-window
    crosslinks: &a2
      references:
        - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
        - CONCEPT--NAMESPACE-VAULT-BRAIN
        - CONCEPT--KNOWLEDGE-LAYERS-V2
        - CONCEPT--CONTEXT-COMPRESSION
    created_at: 2026-05-13T17:22:00.850+07:00
    aliases: &a3
      - CONCEPT
      - implementation_flow
      - Strategic intent / PRD
    cluster: implementation_flow
    role: Strategic intent / PRD
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T17:22:00.850+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
tags: *a1
crosslinks: *a2
created_at: 2026-05-13T17:22:00.850+07:00
aliases: *a3
cluster: implementation_flow
role: Strategic intent / PRD
---

# CONCEPT — Resolution gradient

> The **HOW MUCH** axis of the Universal Context Framework. Resources do not load at a binary on/off; they load at one of several **resolution tiers** chosen by relevance and budget. Agents (or users) can `expand()` a tier on demand — like an LSP "go to definition" applied to retrieval.
>
> Spec section: `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §6. Related decision: D-2 (2-tier MVP).

## Problem

Naive retrieval (top-K by cosine similarity) returns K results at **full content**. As K grows, context window fills, and most of that content is tangentially relevant. The cost is not just tokens — **irrelevant context produces worse outputs**: hallucinated cross-references, spurious "helpful" suggestions, lost focus on the actual task.

Industry response has been to throw more tokens at the problem (1M-context Claude, etc.). This concept proposes the opposite: **load less, but load it smarter**. Humans reading research do not load every cited paper in full; we scan abstracts, expand the relevant ones, glance at titles for the rest. Retrieval should follow the same gradient.

## Hypothesis

A small set of resolution tiers, combined with on-demand promotion, captures the value of full-context retrieval at a fraction of the cost:

```
Tier      Content shape                          Typical tokens   MVP?
─────────────────────────────────────────────────────────────────────────
FULL      complete body + frontmatter            500 – 5000       ✅ yes
SUMMARY   frontmatter + first paragraph + h2s    50 – 300         ⏳ Phase 4
SKELETON  id + title + 1-line description        20 – 60          ⏳ Phase 4
MENTION   id only (pointer for expand)           5 – 10           ✅ yes
```

Per D-2, MVP ships only **FULL** + **MENTION** + `expand()`. The four-tier data model is encoded from day one so SUMMARY/SKELETON renderers are an **additive** Phase 4 deliverable, not a re-architecture.

Empirically, this saves 80-95% of tokens on representative query sets while preserving the ability for an agent to recover full content when relevance is misjudged.

## Scope

In:

- Resolution tier enumeration: `FULL | SUMMARY | SKELETON | MENTION`.
- Tier-assignment function: per-Resource, computed from `score(R) = w1·similarity + w2·1/(1+hops)`, capped by vault and attribute policy.
- `expand(id, { to: tier, reason })` MCP tool and facade method.
- Per-tier token budget allocation (50/30/15/5 default, configurable per vault).
- Tier rendering pipeline: each tier has a dedicated renderer producing the appropriate token shape.
- `default_resolution_policy` field on Vault config.
- Audit log entries for every tier assignment and every `expand()` call.

Out:

- The decision of **which atom to retrieve at all** — that remains the job of vector search + graph traversal (Layer 3 of the pipeline). Resolution tiers operate on the already-retrieved candidate set.
- ABAC filtering — runs before tier assignment (Layer 2 of the pipeline). A denied atom does not get a tier; it is omitted.
- Adaptive learning ("user expanded this 5 times → boost to FULL next time"). Deferred to Phase 5+ — see spec §14.

## Why two tiers first, four eventually

MVP shipping FULL+MENTION proves the **`expand()` autonomy** hypothesis: that agents are willing and able to drill into Resources mid-thought. This is the highest-risk assumption — if it fails, the entire gradient idea is undermined.

If telemetry shows ≥20% of MENTION-tier Resources get expanded by the agent, the renderers for SUMMARY and SKELETON ship in Phase 4 to give intermediate stops on the FULL/MENTION cliff. Below that threshold, the cliff is fine and the renderers are not worth their implementation cost.

## Skeleton content rules

When SKELETON ships, its content must carry **enough for the agent to decide whether to `expand()`** but **no more than that**. Mandatory:

- atom id
- title
- 1-line description (from frontmatter `description:` field or first heading)

Optional, per-vault configurable:
- tags / domain attributes
- last-updated date
- crosslink count (signal of centrality in the graph)

The 1-line description is **mandatory** in particular because without it, agents will hallucinate content from the title alone.

## Composition with ABAC

A denied atom does not get a tier. But the reverse asymmetry must hold too: an atom that ABAC permits at SKELETON but not FULL **must** have its SKELETON content audited for sensitive identifier leakage (a title like `PATIENT-JOHN-DOE-DIAGNOSIS` cannot be a safe SKELETON).

This is policy-author responsibility, enforced by `pack-pii` and similar packs that mark titles redactable.

## Verification

- Recall returns mixed-tier results on a representative query (top-3 FULL + remaining MENTION).
- `expand('id', { to: 'FULL' })` promotes the resource and the new content lands in the next composition.
- Token consumption on a standard query set is ≥60% lower than flat top-K with the same K.
- Budget overflow triggers compression in the configured order (e.g. `compress_full_first`) rather than dropping low-tier first.
- Audit log records each tier assignment with the score that produced it.

## Out of scope

- Concrete hop-metric tuning (`w1` / `w2`) — see spec §14 OQ-1; working assumption `0.7 · sim + 0.3 · 1/(1+hops)`.
- Cross-vault resolution policy (e.g. "atoms from `archive-vault` max out at SKELETON") — covered by Vault config, deferred to `[[FEAT--VAULT-COMPOSITION]]`.
- LLM-side compression (re-summarising a FULL atom) — see `[[CONCEPT--CONTEXT-COMPRESSION]]`.
- Skeleton title redaction implementation — domain-pack scope.

## Source

- `packages/msp/docs/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §6 — tier shapes, scoring, budget allocation, expand-on-demand.
- Decision §0 D-2 — 2-tier MVP, 4-tier gated on Phase 3 telemetry.
- `[[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]` §10 Layer 4 — where this concept sits in the pipeline.
- `[[CONCEPT--NAMESPACE-VAULT-BRAIN]]` — Vaults carry `resolution_policy`.
- Self-RAG (Asai et al., 2023) — prior art for agent-directed retrieval.
- HippoRAG (Gutiérrez et al., 2024) — multi-hop retrieval over knowledge graphs.

## Connections
- [[CONCEPT--KNOWLEDGE-LAYERS-V2]]

