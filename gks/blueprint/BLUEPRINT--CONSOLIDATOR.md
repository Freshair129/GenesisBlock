---
id: BLUEPRINT--CONSOLIDATOR
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — consolidator implementation plan
tags:
  - msp
  - consolidator
  - blueprint
  - implementation
  - m7b
crosslinks: {"implements":["FEAT--CONSOLIDATOR"],"references":["ADR--CONSOLIDATOR-HYBRID-SCORING","CONCEPT--CONSOLIDATOR"]}
linked_symbols:
  - {"file":"packages/msp/src/orchestrator/consolidator/index.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/types.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/score.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/boundary.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/summarise.ts"}
  - {"file":"packages/msp/src/orchestrator/consolidator/llm.ts"}
  - {"file":"packages/msp/test/orchestrator/consolidator/score.test.ts"}
  - {"file":"packages/msp/test/orchestrator/consolidator/boundary.test.ts"}
  - {"file":"packages/msp/test/orchestrator/consolidator/summarise.test.ts"}
  - {"file":"packages/msp/test/orchestrator/consolidator/index.test.ts"}
created_at: 2026-05-04T17:06:30.000+07:00
---

# BLUEPRINT — consolidator implementation plan

```yaml
metadata:
  title: "MSP consolidator — hybrid (deterministic + LLM borderline)"
  parent_feat: FEAT--CONSOLIDATOR

architectural_pattern: |
  Five small pure modules, one orchestrator entry. No class hierarchy — each
  module exports plain functions over plain data.

    types.ts        - Turn, Chunk, Episode, ConsolidateOptions, Verdict
    score.ts        - tier-1 deterministic scoring (pure: turns → score)
    boundary.ts     - episode-boundary detection (pure: turns → ranges)
    llm.ts          - tier-2 caller (LlmClient → JSON parse → score+summary+tags)
    summarise.ts    - deterministic fallback summary (tier-1-keep chunks)
    index.ts        - consolidate() orchestrator: load turns, detect boundaries,
                      score each chunk, gate-or-call-LLM, emit Episode[]

  Re-uses existing infra:
    - LlmClient interface from src/codegen/slm/factory.ts (createSlmClient)
    - Sessions reader (src/memory/sessions/) — already exists
    - No new persistence — caller pipes into EpisodicWriter

data_logic: |
  src/orchestrator/consolidator/score.ts
    Tier-1 features (each is a pure function: chunk → number):
      - decisionMarkers(text)    - regex over "we'll | let's go with | decided | rejected"
      - codeArtifactMentions(text) - matches src/.../*.ts, fn(), ADR--/FEAT--/etc.
      - numericSpecificity(text)   - digits per kilo-byte
      - lengthNormalised(chunk, sessionStats) - z-score vs session mean
      - topicContinuity(chunk, prevChunk)     - cosine of token bag (drop stopwords)
      - deadEndMarkers(text)       - regex over "nevermind | scrap that | doesn't work"
      - greetingFiller(text)       - regex over "hi | thanks | got it | ok"
    scoreChunk() = sum(weight_i * feat_i(chunk)), clamped 0..1
    Verdict thresholds from opts.thresholds (default 0.30 / 0.65 from ADR).

  src/orchestrator/consolidator/boundary.ts
    detectBoundaries(turns, opts):
      window-of-3 turns at a time
      compute topic_continuity vs previous window
      if continuity < boundary_threshold (default 0.25): emit boundary
      always emit boundary at session start + end
    returns [[startIdx, endIdx], ...]

  src/orchestrator/consolidator/llm.ts
    callTier2(chunk, llm, opts):
      build prompt per ADR template
      AbortController with llmCallTimeoutMs
      try parse JSON: { score, summary, tags }
      on success: return { score, summary, tags, source: 'tier2' }
      on timeout/parse-error/no-llm: return { score: 0.6, summary: '', tags: [], source: 'tier2-default' }
        (default-keep — see ADR failure-modes table)

  src/orchestrator/consolidator/summarise.ts
    deterministicSummary(chunk):
      take first sentence of any user-turn (>= 12 chars)
      OR first 120 chars trimmed at word boundary
    extractDeterministicTags(chunk):
      pull frontmatter-style tags from any [[wikilink]] or ADR--/FEAT-- mentions
      cap at 5

  src/orchestrator/consolidator/index.ts
    consolidate(opts):
      1. load session turns (use existing src/memory/sessions/reader if exists,
         else read .brain/msp/projects/<ns>/sessions/<sessionId>.jsonl directly)
      2. compute sessionStats (mean turn length, token bag for topic continuity)
      3. detectBoundaries(turns) → ranges
      4. for each range:
           chunk = turns[start..end]
           tier1 = scoreChunk(chunk, sessionStats, opts.thresholds)
           if tier1.verdict === 'drop':  skip
           if tier1.verdict === 'keep':  push Episode {summary: deterministicSummary(...), tags: deterministicTags(...), score: tier1.score, source: 'tier1'}
           if tier1.verdict === 'borderline':
             if llmCallsUsed < opts.maxLlmCallsPerSession:
               t2 = await callTier2(chunk, opts.llm, opts)
               llmCallsUsed += 1
               if t2.score >= 0.5:
                 push Episode { ...t2, source: t2.source === 'tier2-default' ? 'tier2-default' : 'tier2' }
             else:
               // budget exhausted — default to keep with deterministic summary
               push Episode { ..., source: 'tier2-default' }
      5. return Episode[]

geography:
  - "src/orchestrator/consolidator/index.ts"
  - "src/orchestrator/consolidator/types.ts"
  - "src/orchestrator/consolidator/score.ts"
  - "src/orchestrator/consolidator/boundary.ts"
  - "src/orchestrator/consolidator/llm.ts"
  - "src/orchestrator/consolidator/summarise.ts"
  - "test/orchestrator/consolidator/score.test.ts"        # ~10 tests, 7 features + thresholds
  - "test/orchestrator/consolidator/boundary.test.ts"     # ~5 tests, single-topic / topic-shift / cliff
  - "test/orchestrator/consolidator/llm.test.ts"          # ~10 tests, success / timeout / parse-error / no-provider / budget cap
  - "test/orchestrator/consolidator/summarise.test.ts"    # ~5 tests, sentence pick / fallback / tag extraction
  - "test/orchestrator/consolidator/index.test.ts"        # ~5 fixture-driven end-to-end (mock LLM)

api_contracts:
  - name: consolidate
    signature: |
      function consolidate(opts: ConsolidateOptions): Promise<Episode[]>
    types: |
      interface ConsolidateOptions {
        sessionId: string
        root?: string  // defaults cwd
        llm?: LlmClient  // optional; if absent, all borderline default-keep
        thresholds?: { low?: number; high?: number; boundary?: number }
        maxLlmCallsPerSession?: number  // default 20
        llmCallTimeoutMs?: number       // default 8000
      }
      interface Episode {
        sessionId: string
        turnRange: [number, number]
        summary: string
        tags: string[]
        score: number
        scoreSource: 'tier1' | 'tier2' | 'tier2-default'
        createdAt: string  // ISO 8601
      }

  - name: scoreChunk (sub-module)
    signature: |
      function scoreChunk(chunk: Turn[], stats: SessionStats, thresholds: Thresholds): {
        score: number
        verdict: 'keep' | 'drop' | 'borderline'
        breakdown: Record<string, number>  // per-feature contributions, for debugging
      }

verification_plan:
  - vitest: each tier-1 feature returns expected sign + magnitude on fixture text
  - vitest: deterministic gate verdicts at each threshold edge
  - vitest: boundary detector splits topic-shift fixture into 2 ranges, single-topic into 1
  - vitest: llm.ts parses JSON correctly; timeout returns default-keep; no-provider returns default-keep
  - vitest: maxLlmCallsPerSession cap stops further LLM calls; remaining borderline → default-keep
  - vitest: integration on a realistic session fixture (~30 turns) with mock LLM produces stable Episode[] across runs
  - vitest: idempotent — same input twice → same output (mock LLM)

  Test count: 295 → ~330 (+35)

implementation_order:
  T1 TYPES        types.ts: Turn, Chunk, Episode, ConsolidateOptions, Verdict, SessionStats
  T2 BOUNDARY     boundary.ts + 5 tests
  T3 SCORE        score.ts (7 features + scoreChunk) + 10 tests
  T4 SUMMARISE    summarise.ts (deterministic fallback) + 5 tests
  T5 LLM          llm.ts (tier-2 caller, JSON parse, timeout, default-keep) + 10 tests
  T6 INDEX        index.ts (orchestrator) + 5 integration tests
  T7 AUDIT        AUDIT--CONSOLIDATOR atom recording shipped behaviour + counts
```

## Implementation notes for the implementer

- **No new persistence**. The consolidator's output is `Episode[]` returned in-memory. Caller pipes to `EpisodicWriter`. Do NOT write to `episodic_memory.json` from inside `consolidate()`.

- **LlmClient is optional**. If `opts.llm` is undefined, all borderline chunks → default-keep. This keeps the consolidator usable in headless / no-Ollama setups (tests, CI).

- **Mock provider for tests**. Use `createSlmClient({ provider: 'mock' })` in tests so they're deterministic + fast. Mock client returns fixed `{ score: 0.7, summary: 'mock', tags: ['x','y'] }` unless a fixture overrides.

- **Don't introduce new deps**. Everything you need is already in `package.json` (vitest, GKS adapter, codegen SLM factory).

- **Stay inside `src/orchestrator/consolidator/`**. Don't modify `src/memory/episodic/` or `src/memory/sessions/` — they exist and work; the consolidator consumes their output.

## Implementer: do NOT do

- Write to `episodic_memory.json` (caller's job)
- Mutate `session.jsonl` (immutable input)
- Add a competing LLM bundle (use `createSlmClient`)
- Implement MCP tool wrapping (M7f)
- Tune thresholds beyond defaults from the ADR (M9)

## Connections
- [[FEAT--CONSOLIDATOR]]
- [[ADR--CONSOLIDATOR-HYBRID-SCORING]]
- [[CONCEPT--CONSOLIDATOR]]

