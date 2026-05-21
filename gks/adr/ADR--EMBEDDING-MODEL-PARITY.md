---
id: ADR--EMBEDDING-MODEL-PARITY
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Canonical embedding model — nomic-embed-text-v1.5 across GKS + Smart
  Connections
tags: &a1
  - msp
  - gks
  - embedding
  - semantic-search
  - smart-connections
  - decision
crosslinks: &a2
  references:
    - CONCEPT--EMBEDDING-STRATEGY
    - ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS
    - CONCEPT--OBSIDIAN-AS-RUNTIME
created_at: 2026-05-04T09:02:48.728+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--EMBEDDING-MODEL-PARITY
  phase: 2
  type: adr
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Canonical embedding model — nomic-embed-text-v1.5 across GKS + Smart
    Connections
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-04T09:02:48.728+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--EMBEDDING-MODEL-PARITY
    phase: 2
    type: adr
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Canonical embedding model — nomic-embed-text-v1.5 across GKS + Smart
      Connections
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-04T09:02:48.728+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--EMBEDDING-MODEL-PARITY
      phase: 2
      type: adr
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Canonical embedding model — nomic-embed-text-v1.5 across GKS + Smart
        Connections
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-04T09:02:48.728+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Architecture decision record
      attributes:
        domain: adr
      domain: adr
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — embedding model parity

> **Status (updated 2026-05-07)**: ✅ GKS 3.6.0 published to npm. `npm view @freshair129/gks dist-tags` → `{ latest: '3.6.0' }`. MSP `package.json` bumped to `^3.6.0`. `createNomicEmbedder()` is now usable directly. Migration tracked in `[[AUDIT--GKS-3]].6.0-PUBLISHED`. The "Fallback while GKS 3.6.0 is unpublished" section below is now historical — kept for traceability of the workaround period (2026-05-04 to 2026-05-07).

## Context

GksV3 3.6.0 ships `createNomicEmbedder()` — a Node-side local embedder running `nomic-ai/nomic-embed-text-v1.5` via `@huggingface/transformers`. 768-dim, Thai+English, fully local, no external service.

Smart Connections (Obsidian community plugin) lets the user pick the embedding model in a GUI dropdown — could be sentence-transformers variants, BGE, or nomic.

If GKS embeds with model A and Smart Connections embeds with model B, the **same atom is embedded twice** with **incompatible vectors** (different dimension, tokenizer, normalisation). Cost: 2× compute + 2× storage + inconsistent retrieval results across surfaces.

Worse: a query embedded by GKS cannot be used to search Smart Connections's index (and vice versa), so cross-surface retrieval is impossible without re-embedding queries through both.

`[[ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS]]` (M7-prep) said "MSP delegates semantic to Smart Connections; never embeds". That's incomplete — GKS DOES embed, and we should treat its embedder as the canonical Node-side path. Smart Connections becomes the in-Obsidian browse surface.

This ADR locks the canonical model so all surfaces stay compatible.

## Decision

### Canonical embedding model

**`nomic-ai/nomic-embed-text-v1.5`** — 768-dimensional, 2048 token context, Thai + English mixed-content support.

This matches GKS 3.6.0's default in `createNomicEmbedder()`. Smart Connections users must configure their plugin to use the same model:

```
Obsidian Settings → Smart Connections → Embedding Model:
   nomic-ai/nomic-embed-text-v1.5
```

(Smart Connections supports model selection via its GUI — see `examples/setup/smart-connections-config.md` shipped in this PR.)

### Canonical writer

**GKS** is the canonical embedder. It writes vectors to `.brain/msp/projects/<ns>/vector/atomic.jsonl` (or a configured backend like pgvector / HNSW).

Smart Connections may build its own index in `.smart-connections/` — that's used **only for human browsing inside Obsidian** (Smart View pane, etc.). MSP's agent-facing retrieval (`msp_recall` in M7c) queries GKS's vector store, not Smart Connections's.

### Why two indexes if model is the same

Pragmatically:

- GKS persists vectors in JSONL/HNSW/pgvector via its `VectorBackend` interface.
- Smart Connections persists in `.smart-connections/` with its own schema (which is plugin-version-specific and not stable to read from outside).
- Reading Smart Connections's storage from MSP would couple MSP to plugin internals — fragile.

Until the M10a "msp-bridge" companion plugin lands (Smart Connections reads GKS's store directly), we accept the 2× storage cost as the price of plugin-isolation. Compute cost is borne once per atom-change × 2 surfaces, which is small at typical vault size (< 5,000 atoms).

### Migration path if model changes

- New model selection requires `npm run re-embed` (GKS-provided) + Smart Connections re-index (plugin-managed).
- Bumping this ADR's canonical model = `update_atomic` proposal with semver bump (minor for compatible model swap, major for dimension change).
- During migration, both old and new vectors may coexist briefly — recall results are valid only on the new model post-cutover.

### What if user wants a different model

Allowed but documented as a deviation. Update `MSP_EMBEDDER_MODEL` env var (or GKS config) **and** Smart Connections plugin setting. Project-local ADR records the deviation. MSP doesn't enforce the canonical model — but defaults to it.

## Consequences

**Positive**

- Single conceptual model across surfaces — no "which embedder did this?" ambiguity.
- Vectors interchangeable in principle (same model + tokenizer + normalisation) — opens future M10a optimisation where Smart Connections reads GKS's store.
- Thai content support out of the box (nomic-embed-text-v1.5 has explicit multilingual training).
- No external API keys required — fully local default (privacy + cost win).

**Negative**

- ~2× compute + storage until M10a — accepted at current scale.
- User must configure Smart Connections manually. Recoverable failure: if user picks a different model in SC, agents still get correct results from GKS (canonical), only Obsidian's Smart View shows divergent neighbours. Drift visible to humans, not destructive.
- Locks the project to a 768-dim space — moving to higher-dim models requires migration. Acceptable; nomic v1.5 is a reasonable medium-term anchor.

## Alternatives considered

1. **Each surface picks its own model.** Rejected — inconsistent retrieval, 2× embed work with no compatibility upside.
2. **MSP ships its own embedder (e.g. bundle `transformers.js`).** Rejected — duplicates GKS's bundle; ~100 MB install bloat; version drift.
3. **Use Ollama BGE-M3 as canonical (the GKS 3.5.x default).** Rejected — Ollama is an extra install requirement; nomic via Node-side `@huggingface/transformers` runs without Ollama.
4. **Use OpenAI text-embedding-3-small.** Rejected — sends content to API; not local; cost.

## Fallback while GKS 3.6.0 is unpublished

Until `@freshair129/gks@3.6.0` lands on npm:

| Path | Today (3.5.6) | After 3.6.0 publish |
|---|---|---|
| Agent-facing semantic recall | `provider: 'ollama'` (BGE-M3, requires Ollama install) **or** `provider: 'mock'` (tests / dev) | `createNomicEmbedder()` (no Ollama needed) |
| Smart Connections in Obsidian | configure to match GKS provider — if GKS uses Ollama BGE-M3, set Smart Connections to BGE-M3 | switch both to `nomic-embed-text-v1.5` |
| Vector dimension | depends on Ollama model (typical 1024 for BGE-M3) | 768 (nomic) |

When 3.6.0 publishes:

1. `npm install @freshair129/gks@^3.6.0` (already covered by `^3.5.6` semver — auto-picks)
2. Run `npm run gks re-embed` to migrate vectors from old model to nomic
3. User reconfigures Smart Connections to match
4. This ADR's "Status note" is removed via `update_atomic`

## What this ADR does NOT change

- `[[ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS]]` framing about Smart Connections being the in-Obsidian path remains valid — but **Smart Connections is no longer the canonical writer** (once 3.6.0 ships). That ADR gets clarified in this PR.
- GKS upstream patch for backlinks API / phase 6 — separate concerns.
- M7c retrieval orchestration design — primary semantic source switches to GKS vector store; Smart Connections probe becomes secondary.

## Source

GksV3 3.6.0 CHANGELOG (introduces `createNomicEmbedder`); audit during M7-prep follow-up; user direction for model parity to avoid re-embedding cost. Validation against published `@freshair129/gks@3.5.6` performed 2026-05-04 (M7-prep follow-up review).

## Connections

- [[CONCEPT--EMBEDDING-STRATEGY]]
- [[CONCEPT--OBSIDIAN-AS-RUNTIME]]
