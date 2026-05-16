---
id: ADR--RETRIEVAL-RRF-FUSION
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Retrieval orchestration uses Reciprocal Rank Fusion (RRF) with per-source weights
tags:
  - msp
  - retrieval
  - rrf
  - fusion
  - decision
  - m7c
crosslinks: {"references":["CONCEPT--RETRIEVAL-ORCHESTRATION","CONCEPT--EMBEDDING-STRATEGY","ADR--EMBEDDING-MODEL-PARITY"]}
created_at: 2026-05-05T15:55:30.000+07:00
---

# ADR — retrieval RRF fusion

## Context

`msp_recall(query)` (M7c) merges hits from four heterogeneous sources (GKS vector, Obsidian text, episodic, backlinks). Each source produces hits with scores in **incompatible units**:

- Vector: cosine similarity ∈ [0, 1]
- Obsidian text (REST): plugin-version-specific score, often unscaled BM25-like
- Grep fallback: occurrence count, integer
- Episodic: deterministic match score from M7b summary+tags
- Backlinks: graph-hop confidence, ad-hoc

A single fusion algorithm is needed that doesn't depend on score normalisation.

## Decision

Use **Reciprocal Rank Fusion (RRF)** with per-source weights:

```
score_s(hit) = weight_s / (k + rank_s)
final_score(hit) = sum across sources where hit appears
```

- `k = 60` (default per literature; tunable via `opts.rrfK`)
- `weight_s = 1.0` for all sources by default
- `rank_s` is 1-based ordinal rank from source `s`

### Per-source weights (defaults)

| Source | Weight | Rationale |
|---|---|---|
| `gks-vector` | 1.0 | semantic recall is the primary signal |
| `obsidian-text` | 0.8 | exact-match keyword search; ranks well but plugin schema-private |
| `grep` (fs fallback) | 0.6 | naive substring; lower confidence than indexed |
| `episodic` | 1.2 | recent context is high-signal for "what were we just doing" |
| `backlinks` | 0.5 | graph-hop expansion; suggestive, not authoritative |

Weights are tunable per-call via `opts.weights: Partial<Record<SourceName, number>>`. Beyond M7c, default weights themselves should land in a `PARAM--` atom for traceable tuning (deferred).

### Why RRF over alternatives

| Algorithm | Why not |
|---|---|
| Weighted sum of normalised scores | Each source needs separate normaliser; cosine ∈ [0,1] vs BM25 unbounded vs occurrence integer; brittle |
| Borda count | Equivalent to RRF with `k=0`; pathological for long lists |
| Voting (intersect across all sources) | Too restrictive; single-source-only hits drop |
| Re-rank via cross-encoder | Latency budget too tight for M7c (~50ms); could be M10c |
| Learned-to-rank | No labeled data yet; M9+ work |

RRF is the standard 2009-era IR result for hybrid fusion: simple, robust, no normalisation, well-studied.

### Tie-breaking

When two atoms have identical RRF scores (rare with `k=60`):
1. Hit with **more contributing sources** wins (more source-overlap = stronger signal).
2. Then **lower minimum source rank** (best single-source rank).
3. Then **lexicographic atomId** for determinism.

### Top-K

Default `topK = 10`. Caller can request more; orchestration always returns full result with `rank` field, callers can slice. RRF is computed across all candidates from all sources before slicing.

### Per-source timeouts

Each source has its own `timeoutMs` slice of `opts.timeoutMs`:

| Source | Default timeout |
|---|---|
| `gks-vector` | 800ms |
| `obsidian-text` | 400ms |
| `grep` fallback | 600ms |
| `episodic` | 100ms (in-memory) |
| `backlinks` | 100ms (in-memory) |

If a source times out: omit from fusion, append `'<source>: timeout'` to `fallback_reasons`. Other sources keep going.

If `opts.timeoutMs` is set, it overrides the sum and per-source budgets are scaled proportionally.

## Consequences

**Positive**
- No score-normalisation code per source — RRF only uses ranks
- Adding a new source (e.g. M10a Smart Connections companion) is a single source adapter + a default weight
- Per-call weight overrides let agents say "give me more episodic context"
- Standard algorithm — easily explained in audits

**Negative**
- RRF discards score magnitude — a vector hit at cosine 0.99 vs 0.71 contributes the same if both rank #1 in vector. Acceptable trade-off given heterogeneous sources.
- Default weights are gut-feel. Tuning needs real query data (M9 work via `[[PARAM--RETRIEVAL-WEIGHTS]]`).
- `k=60` is also gut-feel from literature. Same caveat.

## Alternatives considered

1. **Weighted sum of cosine + BM25** — rejected; needs normalisation per-source, brittle when adding sources.
2. **Cross-encoder re-rank top-50** — considered for future. Re-rank latency dominates RRF latency by 10x at typical sizes; not for M7c. Tracked as M10c.
3. **CombSUM / CombMNZ** — older fusion algorithms; require score normalisation. RRF strictly better here.
4. **Pairwise voting** — high latency, no clear win over RRF.

## What this ADR does NOT decide

- **Source list / source names** — see `[[CONCEPT--RETRIEVAL-ORCHESTRATION]]`
- **Cancel-on-budget vs run-to-completion** — see BLUEPRINT (decision: cancel-on-budget per source, total budget is the floor)
- **Default `topK`** — 10 unless `opts.topK` overrides; caller's choice
- **Snippet selection** — see BLUEPRINT (decision: source-provided snippet preserved; no re-extraction)
- **Caching** — out of M7c

## Source

`msp_spec.md` §7c, `[[CONCEPT--RETRIEVAL-ORCHESTRATION]]`, classic Cormack et al. 2009 (Reciprocal Rank Fusion) + practical IR experience.

## Connections
- [[CONCEPT--EMBEDDING-STRATEGY]]
- [[ADR--EMBEDDING-MODEL-PARITY]]

