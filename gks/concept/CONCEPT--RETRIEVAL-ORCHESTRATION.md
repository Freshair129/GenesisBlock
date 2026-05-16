---
id: CONCEPT--RETRIEVAL-ORCHESTRATION
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Retrieval orchestration — RRF fusion across GKS vector + Obsidian + episodic + backlinks
tags:
  - msp
  - retrieval
  - rrf
  - hybrid
  - msp-recall
  - m7c
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","FEAT--MSP-OBSIDIAN-CLIENT","FEAT--CONSOLIDATOR","CONCEPT--EMBEDDING-STRATEGY","CONCEPT--MEMORY-VECTOR-BACKLINKS"]}
created_at: 2026-05-05T15:55:00.000+07:00
---

# CONCEPT — retrieval orchestration

## Problem

`msp_recall(query)` is the killer agent-facing primitive: "tell me what we know about X". A single source rarely answers well:

- **Vector-only** misses exact ID matches and recent episodic context
- **Keyword-only** misses semantic neighbours (different words, same idea)
- **Graph-only** misses anything not yet linked
- **Episodic-only** misses durable knowledge in `gks/`

Per `msp_spec.md` §7c, M7c fuses four sources via Reciprocal Rank Fusion (RRF) and returns ranked hits with provenance. Caller (agent / MCP tool) gets one merged list — no "which source is best for this query" decision.

## The four sources

| Source | What | Latency budget | Headless? |
|---|---|---|---|
| **GKS vector** | semantic neighbours via `createEmbedder`/`createNomicEmbedder` + `VectorBackend` | ~50–200ms (local nomic) | ✅ yes |
| **Obsidian text** | keyword + tag search via REST when `client.mode === 'rest'`; grep-on-disk fallback when `'filesystem'` | ~20ms REST / ~100ms grep | ✅ via fallback |
| **Episodic** | match query against `episodic_memory.json` summary + tags | ~5ms (in-memory) | ✅ yes |
| **Backlinks** | graph-hop from any atom in the candidate set (1-hop expansion) | ~5ms (from in-memory edge list) | ✅ yes |

All four run in parallel. The slowest (vector) sets the floor; orchestration adds ~5ms for RRF.

## Reciprocal Rank Fusion (RRF)

For each source `s`, every hit at rank `r` (1-based) contributes:

```
score_s(hit) = weight_s / (k + r)
```

Final score per hit ID = sum across sources where it appears. `k` is a constant (typical 60); `weight_s` defaults to 1.0 per source.

Why RRF over weighted-sum-of-similarity:
- **No score normalisation needed** — vector similarity, BM25, exact-match all live in different ranges. RRF only uses ordinal rank.
- **Robust to outliers** — one source with an extreme score doesn't dominate.
- **Tunable per source** without re-normalising — adjust `weight_s`.

## What gets returned

```ts
{
  hits: RetrievalHit[],
  semantic_available: boolean,     // GKS vector path operational
  obsidian_available: boolean,     // Obsidian REST reachable (for deep-links)
  fallback_reasons: string[],      // e.g. ['obsidian-rest-down: timeout']
  timings: {                       // per-source latency in ms (debug)
    vector?: number
    obsidian?: number
    episodic?: number
    backlinks?: number
    fusion?: number
  }
}

interface RetrievalHit {
  atomId: string                   // 'FRAME--FOO' / 'ADR--BAR' / 'episode-2026-05-04-xyz'
  source: 'gks-vector' | 'obsidian-text' | 'grep' | 'episodic' | 'backlinks'
  score: number                    // RRF-summed score (higher = better)
  rank: number                     // final 1-based rank in the merged list
  snippet?: string                 // best-effort text excerpt for the agent
  perSourceRanks?: Partial<Record<SourceName, number>>  // for debug
}
```

## Where it sits

```
msp_recall(query, opts)
  │
  ▼
┌────────────────────────────────────────────────┐
│ src/orchestrator/retrieval/index.ts            │  ← M7c (this work)
│  ├── parallel fan-out (Promise.all)            │
│  │   ├── gks-vector source                     │
│  │   ├── obsidian-client search (REST or fs)   │
│  │   ├── episodic source (read .json + match)  │
│  │   └── backlinks source (1-hop from set)     │
│  ├── per-source: try / timeout / collect       │
│  ├── RRF fusion (k, weights from opts)         │
│  ├── dedup by atomId (sum across sources)      │
│  └── top-K + provenance + timings              │
└────────────────────────────────────────────────┘
                    │
                    ▼
              caller (agent)
```

Wraps:
- **`createObsidianClient`** (M7a) for Obsidian search
- **`@evaai/gks` `recall`** OR direct `VectorBackend.search` (depending on what 3.5.6 exposes; aspirational `createNomicEmbedder` from 3.6.0 stays consistent — see `[[ADR--EMBEDDING-MODEL-PARITY]]` Status note)
- **`Episode[]`** structure from M7b consolidator (read from `episodic_memory.json`)
- **`backlinks.jsonl`** from M3c-1 indexer

Nothing new persists. M7c is pure read-side orchestration.

## Invariants

- **Total budget bounded** — `opts.timeoutMs` (default 1500ms). Slow sources are cancelled, partial results returned with `fallback_reasons` populated.
- **Source failures are non-fatal** — vector-down doesn't block episodic results. Each source try / catch-individually.
- **Idempotent** — same query + same on-disk state → same hits (modulo embedder non-determinism on borderline scores).
- **Headless-safe** — works without Obsidian (filesystem fallback) and without an LLM (vector still works; embedder is local nomic / sentence-transformers).
- **No mutation** — strictly read-only on all four sources.

## Out of scope (deferred)

- **MCP tool wrapping** (`msp_recall`) — M7f
- **Cross-namespace recall** — M9 (cross-tenant auth needed first)
- **Query rewriting / HyDE** — out of M7 scope
- **Cache layer** — RRF inputs aren't expensive to recompute; cache when retrieval > 500ms
- **Re-ranking via cross-encoder** — out of M7 scope; could be M10c
- **Agent learning from clicks** — feedback loop is application-layer concern

## Source

`msp_spec.md` §7c, `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`, M7a/M7b/M7e completion (PRs #12, #16, #19, #20).

## Connections
- [[FEAT--MSP-OBSIDIAN-CLIENT]]
- [[FEAT--CONSOLIDATOR]]
- [[CONCEPT--EMBEDDING-STRATEGY]]
- [[CONCEPT--MEMORY-VECTOR-BACKLINKS]]

