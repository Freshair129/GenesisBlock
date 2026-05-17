---
id: BLUEPRINT--RETRIEVAL-ORCHESTRATION
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — retrieval orchestration implementation plan
tags:
  - msp
  - retrieval
  - blueprint
  - implementation
  - m7c
crosslinks:
  implements:
    - FEAT--RETRIEVAL-ORCHESTRATION
  references:
    - ADR--RETRIEVAL-RRF-FUSION
    - CONCEPT--RETRIEVAL-ORCHESTRATION
linked_symbols:
  - file: packages/msp/src/orchestrator/retrieval/index.ts
  - file: packages/msp/src/orchestrator/retrieval/types.ts
  - file: packages/msp/src/orchestrator/retrieval/fusion.ts
  - file: packages/msp/src/orchestrator/retrieval/sources/vector.ts
  - file: packages/msp/src/orchestrator/retrieval/sources/obsidian.ts
  - file: packages/msp/src/orchestrator/retrieval/sources/episodic.ts
  - file: packages/msp/src/orchestrator/retrieval/sources/backlinks.ts
created_at: 2026-05-05T15:56:30.000+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  domain: blueprint
---

# BLUEPRINT — retrieval orchestration implementation plan

```yaml
metadata:
  title: "MSP retrieval orchestration — RRF fusion across 4 sources"
  parent_feat: FEAT--RETRIEVAL-ORCHESTRATION

architectural_pattern: |
  One factory + four source adapters + one fusion module + one orchestrator.
  All plain functions over plain data. No class hierarchies.

    types.ts                 RecallOptions, RetrievalHit, RetrievalResult,
                             SourceName, SourceResult, RrfWeights, defaults
    fusion.ts                pure RRF: rankPerSource → atomScores → sortedHits
    sources/vector.ts        wraps GKS recall / VectorBackend.search via embedder
    sources/obsidian.ts      wraps M7a ObsidianClient.search (REST or fs grep)
    sources/episodic.ts      reads episodic_memory.json, scores by summary+tags overlap
    sources/backlinks.ts     reads backlinks.jsonl, 1-hop expansion from candidate set
    index.ts                 recall() orchestrator: parallel fan-out, timeouts, RRF, top-K

  Re-uses:
    - createObsidianClient (M7a)        — for sources/obsidian.ts
    - episodic_memory.json shape (M7b)  — for sources/episodic.ts
    - backlinks.jsonl (M3c-1)           — for sources/backlinks.ts
    - GKS embedder + vector backend      — for sources/vector.ts
  Imports nothing new (no deps added).

data_logic: |
  src/orchestrator/retrieval/fusion.ts (pure, no I/O)
    rrfFuse(perSource: SourceResult[], opts: { k, weights, topK }):
      atomScores: Map<atomId, {
        score: number, contributions: Array<{ source, rank }>, snippet?, sourceCount: 0
      }>
      for each sourceResult:
        for each hit at rank r (1-based):
          contribution = (weights[source] ?? 1.0) / (opts.k + r)
          if !atomScores.has(hit.atomId):
            atomScores.set(hit.atomId, { score: 0, contributions: [], snippet: hit.snippet, sourceCount: 0 })
          entry = atomScores.get(hit.atomId)
          entry.score += contribution
          entry.contributions.push({ source: hit.source, rank: r })
          entry.sourceCount += 1
          // Prefer first-seen snippet; could prefer highest-weighted source's
          // snippet — kept simple for M7c.
      sort atomScores entries by:
        1. score DESC
        2. sourceCount DESC (tie-break: more sources)
        3. min rank ASC (best single-source rank)
        4. atomId lexicographic
      slice topK
      assign final rank 1..topK
      return RetrievalHit[]

  src/orchestrator/retrieval/sources/vector.ts
    vectorSource({ query, root, namespace, topK, timeoutMs }):
      try:
        get GKS MemoryStore for root + namespace
        get embedder via createEmbedder() / createNomicEmbedder() per ADR-EMBEDDING-MODEL-PARITY
        embed(query) with race-vs-timeout
        search vector backend, top topK
        map to SourceHit[] with source: 'gks-vector', snippet from atom body
        return { source: 'gks-vector', hits, latencyMs }
      catch any: return { source: 'gks-vector', hits: [], error: msg, latencyMs }

  src/orchestrator/retrieval/sources/obsidian.ts
    obsidianSource({ obsidian, query, topK, timeoutMs }):
      if !obsidian: return { source: 'obsidian-text', hits: [], skipped: 'no-client' }
      raceTimeout(obsidian.search(query, { limit: topK }), timeoutMs):
        on success: map to SourceHit[]; source = 'obsidian-text' if mode='rest' else 'grep'
        on timeout: { hits: [], error: 'timeout' }

  src/orchestrator/retrieval/sources/episodic.ts
    episodicSource({ root, namespace, query, topK }):
      load .brain/msp/projects/<ns>/memory/episodic_memory.json (in-memory cache)
      tokenise query → bag
      score each Episode by:
        summary token-overlap (Jaccard with bag) +
        tag exact-match bonus (each match = +0.2)
      sort desc, slice topK
      map to SourceHit { atomId: episode.atomId ?? `episode:<sessionId>:<turnRange>`, source: 'episodic' }
      handle ENOENT → empty hits, no error

  src/orchestrator/retrieval/sources/backlinks.ts
    backlinksSource({ root, namespace, candidateAtomIds, topK }):
      load .brain/msp/projects/<ns>/vector/backlinks.jsonl
      build edge index: from → [to], to → [from]
      for each candidate: collect 1-hop neighbours (both directions)
      score by neighbour-count (more candidates pointing to it = more relevant)
      sort desc, slice topK
      map to SourceHit { atomId: neighbour, source: 'backlinks' }
      handle ENOENT → empty, no error

  src/orchestrator/retrieval/index.ts
    recall(opts):
      timing.start = perf.now()
      const totalBudget = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS
      const perSourceBudgets = scaleBudgets(totalBudget, opts.perSourceTimeouts ?? DEFAULT_PER_SOURCE_TIMEOUTS)

      // Phase A: 3 sources fan out from query directly (vector, obsidian, episodic)
      const [vectorRes, obsidianRes, episodicRes] = await Promise.allSettled([
        vectorSource({ query, root, namespace, topK, timeoutMs: perSourceBudgets.vector }),
        obsidianSource({ obsidian, query, topK, timeoutMs: perSourceBudgets.obsidian }),
        episodicSource({ root, namespace, query, topK, timeoutMs: perSourceBudgets.episodic }),
      ])

      // Collect candidates from phase A successes
      const phaseA = collectSettled([vectorRes, obsidianRes, episodicRes])

      // Phase B: backlinks expansion from candidates (small + fast — fits remaining budget)
      const candidates = uniqueAtomIds(phaseA)
      const backlinksRes = await raceTimeout(
        backlinksSource({ root, namespace, candidateAtomIds: candidates, topK }),
        perSourceBudgets.backlinks,
      )

      // Phase C: fuse
      const fusedHits = rrfFuse([...phaseA, backlinksRes], { k: opts.rrfK ?? 60, weights, topK })

      return {
        hits: fusedHits,
        semantic_available: vectorRes.status === 'fulfilled' && (vectorRes.value.hits.length > 0 || !vectorRes.value.error),
        obsidian_available: obsidian?.mode === 'rest',
        fallback_reasons: collectFallbackReasons([vectorRes, obsidianRes, episodicRes, backlinksRes]),
        timings: { vector: ..., obsidian: ..., episodic: ..., backlinks: ..., fusion: ... },
      }

geography:
  - "packages/msp/src/orchestrator/retrieval/index.ts"
  - "packages/msp/src/orchestrator/retrieval/types.ts"
  - "packages/msp/src/orchestrator/retrieval/fusion.ts"
  - "packages/msp/src/orchestrator/retrieval/sources/vector.ts"
  - "packages/msp/src/orchestrator/retrieval/sources/obsidian.ts"
  - "packages/msp/src/orchestrator/retrieval/sources/episodic.ts"
  - "packages/msp/src/orchestrator/retrieval/sources/backlinks.ts"
  - "packages/msp/test/orchestrator/retrieval/fusion.test.ts"          # ~12 tests
  - "packages/msp/test/orchestrator/retrieval/sources/vector.test.ts"  # ~5 (mock embedder + backend)
  - "packages/msp/test/orchestrator/retrieval/sources/obsidian.test.ts" # ~6 (mock client both modes)
  - "packages/msp/test/orchestrator/retrieval/sources/episodic.test.ts" # ~6 (fixture episodic memory)
  - "packages/msp/test/orchestrator/retrieval/sources/backlinks.test.ts" # ~6 (fixture backlinks.jsonl)
  - "packages/msp/test/orchestrator/retrieval/index.test.ts"          # ~10 end-to-end (mocks)

api_contracts:
  - name: recall
    signature: |
      function recall(opts: RecallOptions): Promise<RetrievalResult>
    types: |
      type SourceName = 'gks-vector' | 'obsidian-text' | 'grep' | 'episodic' | 'backlinks'

      interface RecallOptions {
        query: string
        root?: string                                  // default cwd
        namespace?: string                             // default 'evaAI'
        obsidian?: ObsidianClient                      // optional; if absent, source skipped
        topK?: number                                  // default 10
        timeoutMs?: number                             // total budget; default 1500
        perSourceTimeouts?: Partial<Record<SourceName, number>>
        weights?: Partial<Record<SourceName, number>>
        rrfK?: number                                  // default 60
        embedder?: Embedder                            // optional injection for tests
      }

      interface RetrievalHit {
        atomId: string
        source: SourceName
        score: number                                  // RRF-summed
        rank: number                                   // 1-based final
        snippet?: string
        perSourceRanks: Partial<Record<SourceName, number>>
      }

      interface RetrievalResult {
        hits: RetrievalHit[]
        semantic_available: boolean
        obsidian_available: boolean
        fallback_reasons: string[]
        timings: {
          vector?: number; obsidian?: number; episodic?: number; backlinks?: number;
          fusion: number
        }
      }

      interface SourceResult {
        source: SourceName
        hits: SourceHit[]
        latencyMs: number
        error?: string
        skipped?: string
      }
      interface SourceHit { atomId: string; rank: number; snippet?: string; source: SourceName }

  - name: rrfFuse (pure)
    signature: |
      function rrfFuse(
        perSource: SourceResult[],
        opts: { k: number; weights: Partial<Record<SourceName, number>>; topK: number },
      ): RetrievalHit[]

verification_plan:
  - vitest fusion: 12 tests
      - empty input → []
      - single source single hit → hit at rank 1
      - two sources same atom → score sums
      - two sources different atoms → distinct hits
      - tie-break by sourceCount
      - tie-break by min source rank
      - tie-break by atomId lexicographic
      - weights respected (vector=2.0 vs episodic=0.5)
      - rrfK=10 vs rrfK=60 produces different scores
      - topK slicing
      - rank field assigned 1..topK
      - perSourceRanks populated correctly
  - vitest vector source: 5 tests
      - mock embedder + mock backend returns hits
      - timeout returns empty + error
      - no embedder → empty + error
      - empty query → empty
      - latencyMs populated
  - vitest obsidian source: 6 tests
      - rest mode hits
      - filesystem mode (grep) hits
      - missing client → skipped
      - timeout → error
      - source label respects mode (rest → 'obsidian-text', fs → 'grep')
      - latencyMs populated
  - vitest episodic source: 6 tests
      - missing file → empty
      - tag-only match
      - summary-token-overlap match
      - tag bonus stacks on overlap
      - sorted desc
      - topK slicing
  - vitest backlinks source: 6 tests
      - missing file → empty
      - 1-hop expansion outbound
      - 1-hop expansion inbound
      - dedup neighbours
      - score by candidate-count
      - topK slicing
  - vitest end-to-end: 10 tests
      - 4 sources all populate → fused with provenance
      - 1 source fails → others fuse, fallback_reasons populated
      - all fail → empty hits + reasons
      - timeout total budget
      - opts.weights override
      - opts.rrfK respected
      - obsidian_available reflects client.mode
      - semantic_available reflects vector source
      - timings populated
      - idempotent (same input twice → same hits via mocks)

  Test count: 350 → ~395 (+45)

implementation_order:
  T1 TYPES         types.ts: RecallOptions, RetrievalHit, RetrievalResult, SourceResult, SourceName, defaults
  T2 FUSION        fusion.ts (pure RRF) + 12 tests
  T3 SOURCE_VECTOR sources/vector.ts + 5 tests (mock embedder + mock backend)
  T4 SOURCE_OBS    sources/obsidian.ts + 6 tests (mock ObsidianClient both modes)
  T5 SOURCE_EPI    sources/episodic.ts + 6 tests (fixture episodic_memory.json)
  T6 SOURCE_BL     sources/backlinks.ts + 6 tests (fixture backlinks.jsonl)
  T7 INDEX         index.ts (orchestrator) + 10 end-to-end tests (mocks)
  T8 AUDIT         AUDIT--RETRIEVAL-ORCHESTRATION atom recording shipped behaviour + counts
```

## Implementation notes for the implementer

- **Run `npm ci`** in the worktree before `npm test` (CLAUDE.md worktree caveat).
- **No new deps**. `node:fs/promises`, `node:path`, `perf_hooks` (Node built-in).
- **GKS API**: use whatever MemoryStore/recall surface 3.5.6 actually exposes (check `node_modules/@freshair129/gks/dist/src/memory/index.d.ts`). The vector source can degrade to `provider: 'mock'` for tests if `ollama` not configured.
- **Obsidian source**: must NOT instantiate its own `createObsidianClient` — caller provides it. Keeps M7c orthogonal to M7a's life-cycle.
- **Episodic source**: episodic file may not exist (fresh project). Treat ENOENT as empty, NOT error.
- **Backlinks source**: same — ENOENT = empty.
- **Snippets**: each source provides best-effort snippet; fusion does NOT re-extract. If multiple sources have the same atom, prefer the **first-seen non-empty** snippet (deterministic by source iteration order).
- **Two-phase fan-out**: phase A is the 3 query-driven sources in parallel. Phase B is backlinks 1-hop from phase-A candidates. Combined with `Promise.allSettled` for fault isolation.
- **Cancellation**: `Promise.race(call, timeout)` per source. The slow source's promise is left dangling — acceptable for M7c (no cleanup needed for in-memory ops; vector + obsidian have their own internal aborts).

## Implementer: do NOT do

- Add MCP tool wrapping (M7f scope)
- Cache results across calls (out of M7c)
- Re-rank via cross-encoder (M10c)
- Implement query rewriting / HyDE
- Mutate any source's data
- Tune default weights beyond the ADR (M9 PARAM atom)
- Persist anything (M7c is read-only)

## Connections
- [[FEAT--RETRIEVAL-ORCHESTRATION]]
- [[ADR--RETRIEVAL-RRF-FUSION]]
- [[CONCEPT--RETRIEVAL-ORCHESTRATION]]

