---
id: FEAT--RETRIEVAL-ORCHESTRATION
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Retrieval orchestration — recall(query) fans out across 4 sources, fuses via RRF
tags:
  - msp
  - retrieval
  - rrf
  - msp-recall
  - m7c
  - user-facing
crosslinks: {"implements":["ADR--RETRIEVAL-RRF-FUSION"],"references":["CONCEPT--RETRIEVAL-ORCHESTRATION","FEAT--MSP-OBSIDIAN-CLIENT","FEAT--CONSOLIDATOR"]}
linked_symbols:
  - {"file":"packages/msp/src/orchestrator/retrieval/index.ts"}
  - {"file":"packages/msp/src/orchestrator/retrieval/types.ts"}
  - {"file":"packages/msp/src/orchestrator/retrieval/fusion.ts"}
  - {"file":"packages/msp/src/orchestrator/retrieval/sources/vector.ts"}
  - {"file":"packages/msp/src/orchestrator/retrieval/sources/obsidian.ts"}
  - {"file":"packages/msp/src/orchestrator/retrieval/sources/episodic.ts"}
  - {"file":"packages/msp/src/orchestrator/retrieval/sources/backlinks.ts"}
created_at: 2026-05-05T15:56:00.000+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: "Feature spec"
---

# Retrieval orchestration — `recall(query)` fans out across 4 sources, fuses via RRF

## User-facing API

```ts
import { recall } from '@/orchestrator/retrieval'
import { createObsidianClient } from '@/obsidian/client'

const client = await createObsidianClient({ root: process.cwd() })

const result = await recall({
  query: 'how did we decide rate limiting?',
  root: process.cwd(),
  namespace: 'evaAI',
  obsidian: client,                 // optional; if absent, obsidian source is skipped
  topK: 10,                         // default 10
  timeoutMs: 1500,                  // default 1500
  weights: { episodic: 1.5 },       // optional per-source weight override
  rrfK: 60,                         // default 60
})

result.hits[0]                      // { atomId, source, score, rank, snippet, perSourceRanks }
result.semantic_available           // true if vector path operational
result.obsidian_available           // true if Obsidian REST reachable (for deep-links)
result.fallback_reasons             // ['obsidian-rest-down: timeout'] or []
result.timings                      // { vector: 142, episodic: 3, ... }
```

## Acceptance criteria

- [ ] `recall(opts)` returns `RetrievalResult` with `hits`, `semantic_available`, `obsidian_available`, `fallback_reasons`, `timings`
- [ ] **Parallel fan-out** — all 4 sources run concurrently via `Promise.allSettled`
- [ ] **Per-source try/catch** — one failing source doesn't break the call
- [ ] **Per-source timeouts** — each source has its own budget (defaults from ADR); on timeout, omit that source + append to `fallback_reasons`
- [ ] **Total budget enforced** — `opts.timeoutMs` overrides sum of per-source; orchestration cancels remaining sources at budget
- [ ] **RRF fusion** per `[[ADR--RETRIEVAL-RRF-FUSION]]` — `weight_s / (k + rank)`, summed across sources, sorted desc
- [ ] **Tie-breaking** — more sources > lower min rank > lexicographic atomId
- [ ] **No mutation** — read-only on all sources
- [ ] **Headless-safe** — works without Obsidian (filesystem fallback in `createObsidianClient` mode='filesystem' or `obsidian: undefined`)
- [ ] **Snippet preserved** from source (no re-extraction)
- [ ] **Provenance complete** — `source` field on every hit, `perSourceRanks` for debug
- [ ] **Idempotent** — same query + same on-disk state → same hits (modulo embedder borderline non-determinism)
- [ ] Test target 350 → ~395 (+45)

## Surfaces

| Surface | Form |
|---|---|
| TS API | `recall(opts: RecallOptions): Promise<RetrievalResult>` |
| Sub-modules | `fusion.ts` (pure RRF), `sources/{vector,obsidian,episodic,backlinks}.ts` |
| Tests | `test/orchestrator/retrieval/{fusion,vector,obsidian,episodic,backlinks,index}.test.ts` |

## Out of scope

- MCP tool wrapping (`msp_recall`) → M7f
- Cross-namespace recall → M9
- Cache layer → only if perf demands (M9+)
- Cross-encoder re-rank → M10c
- Query rewriting / HyDE → out of M7

## Connections
- [[CONCEPT--RETRIEVAL-ORCHESTRATION]]
- [[FEAT--MSP-OBSIDIAN-CLIENT]]
- [[FEAT--CONSOLIDATOR]]

