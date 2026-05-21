---
id: BLUEPRINT--COMPRESSOR
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — context compressor implementation plan
tags: &a1
  - msp
  - compressor
  - blueprint
  - implementation
  - m7d
crosslinks: &a2
  implements:
    - FEAT--COMPRESSOR
  references:
    - ADR--COMPRESSOR-THREE-TIER
    - CONCEPT--CONTEXT-COMPRESSION
linked_symbols: &a3
  - file: packages/msp/src/orchestrator/compressor/index.ts
  - file: packages/msp/src/orchestrator/compressor/types.ts
  - file: packages/msp/src/orchestrator/compressor/tokens.ts
  - file: packages/msp/src/orchestrator/compressor/trim.ts
  - file: packages/msp/src/orchestrator/compressor/resummarise.ts
  - file: packages/msp/test/orchestrator/compressor/tokens.test.ts
  - file: packages/msp/test/orchestrator/compressor/trim.test.ts
  - file: packages/msp/test/orchestrator/compressor/resummarise.test.ts
  - file: packages/msp/test/orchestrator/compressor/index.test.ts
created_at: 2026-05-05T16:11:00.000+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--COMPRESSOR
  phase: 3
  type: blueprint
  scale_level: L2
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: BLUEPRINT — context compressor implementation plan
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-05T16:11:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--COMPRESSOR
    phase: 3
    type: blueprint
    scale_level: L2
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: BLUEPRINT — context compressor implementation plan
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-05T16:11:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      id: BLUEPRINT--COMPRESSOR
      phase: 3
      type: blueprint
      scale_level: L2
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: BLUEPRINT — context compressor implementation plan
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-05T16:11:00.000+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Implementation plan
      attributes:
        domain: blueprint
      domain: blueprint
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: aws_secret
      leak_risk: high
      encryption_level: none
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: aws_secret
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: aws_secret
  leak_risk: high
  encryption_level: none
---

# BLUEPRINT — compressor implementation plan

```yaml
metadata:
  title: "MSP context compressor (three-tier: keep / trim / resummarise)"
  parent_feat: FEAT--COMPRESSOR

architectural_pattern: |
  Five small modules, plain functions over plain data.

    types.ts         CompressOptions, CompressedEpisode, CompressResult, Tokeniser
    tokens.ts        estimateTokens (char/3.5 default), Tokeniser type
    trim.ts          trimEpisode (drop low-score turns until fits)
    resummarise.ts   resummariseLLM (call + parse) + truncate fallback
    index.ts         compress() orchestrator: importance-sort, greedy fill, tier choice

  Re-uses:
    - Episode shape + tier1 scoring infrastructure from M7b consolidator
    - createSlmClient (LlmClient) — same as M7b tier-2

data_logic: |
  src/orchestrator/compressor/tokens.ts
    DEFAULT_TOKENISER = (s) => Math.ceil(s.length / 3.5)
    interface Tokeniser { (s: string): number }
    estimateText(text, tokeniser) → number

  src/orchestrator/compressor/trim.ts
    trimEpisode(episode, target, tokeniser, opts):
      score each turn via tier1ScoreSingleTurn (re-use from M7b score.ts; export
      a single-turn-friendly variant if not yet present)
      candidates = turns where score < TRIM_THRESHOLD (0.3 default)
      sort candidates by score asc (lowest first = most droppable)
      result = [...turns]
      droppedIndices = []
      while result not yet ≤ target AND candidates not empty:
        candidate = candidates.shift()
        result = result.filter(t !== candidate)
        droppedIndices.push(candidate.index)
      finalText = joinTurns(result preserving original order)
      return { text: finalText, droppedIndices, fits: estimateText(finalText) ≤ target }

  src/orchestrator/compressor/resummarise.ts
    resummariseLLM(episode, target, llm, tokeniser, timeoutMs):
      build prompt (see prompt template below)
      raceTimeout(llm({ prompt }), timeoutMs)
      try parse text response → returns text directly (not JSON)
      if estimateText(parsed) ≤ target: return { text: parsed, by: 'resummarise' }
      else: fall through to truncate

    truncate(episode, target, tokeniser):
      // Keep most-recent turns first (typically most relevant)
      result = []
      for turn of episode.turns reversed:
        candidate = [turn, ...result]
        if estimateText(joinTurns(candidate)) ≤ target:
          result = candidate
        else: break
      return { text: joinTurns(result preserving chronological order),
               by: 'truncated',
               droppedIndices: indices not in result }

    prompt template:
      """
      Re-summarise the following conversation chunk in approximately {target} tokens.
      Preserve key decisions, facts, and code references. Drop greetings, dead ends,
      and conversational filler.

      Chunk:
      ---
      {episode text}
      ---

      Return ONLY the summary text (no preamble, no markdown fences).
      """

  src/orchestrator/compressor/index.ts
    compress(opts):
      tokeniser = opts.tokeniser ?? DEFAULT_TOKENISER
      sorted = [...opts.episodes].sort((a, b) => b.score - a.score)
      compressed: CompressedEpisode[] = []
      tokensUsed = 0
      tierCounts = { keep: 0, trim: 0, resummarise: 0, truncated: 0, dropped: 0 }

      for episode of sorted:
        remaining = opts.budgetTokens - tokensUsed
        summaryTokens = tokeniser(episode.summary || '')
        if remaining < summaryTokens:
          tierCounts.dropped += 1
          continue

        fullText = joinTurns(episode.turns)
        fullTokens = tokeniser(fullText)

        // tier 1: keep
        if fullTokens ≤ remaining:
          push { ...episode, text: fullText, compressedBy: 'keep', droppedIndices: [] }
          tokensUsed += fullTokens
          tierCounts.keep += 1
          continue

        // tier 2: trim
        trimResult = trimEpisode(episode, remaining, tokeniser)
        if trimResult.fits AND trimResult.droppedIndices.length / episode.turns.length ≥ 0.30:
          push { ...episode, text: trimResult.text, compressedBy: 'trim', droppedIndices: trimResult.droppedIndices }
          tokensUsed += tokeniser(trimResult.text)
          tierCounts.trim += 1
          continue

        // tier 3: resummarise OR truncate
        target = Math.min(Math.floor(fullTokens * 0.6), remaining)
        if opts.llm:
          try:
            r = await resummariseLLM(episode, target, opts.llm, tokeniser, opts.llmTimeoutMs ?? 8000)
            if r:
              push { ...episode, text: r.text, compressedBy: 'resummarise', droppedIndices: [] }
              tokensUsed += tokeniser(r.text)
              tierCounts.resummarise += 1
              continue
          catch: // fall through to truncate

        // tier 3 fallback: truncate
        tr = truncate(episode, remaining, tokeniser)
        push { ...episode, text: tr.text, compressedBy: 'truncated', droppedIndices: tr.droppedIndices }
        tokensUsed += tokeniser(tr.text)
        tierCounts.truncated += 1

      if opts.preserveOrder:
        compressed.sort by episodeRef.turnRange[0] asc

      return { compressed, totalTokensUsed: tokensUsed, tierCounts }

geography:
  - "packages/msp/src/orchestrator/compressor/index.ts"
  - "packages/msp/src/orchestrator/compressor/types.ts"
  - "packages/msp/src/orchestrator/compressor/tokens.ts"
  - "packages/msp/src/orchestrator/compressor/trim.ts"
  - "packages/msp/src/orchestrator/compressor/resummarise.ts"
  - "packages/msp/test/orchestrator/compressor/tokens.test.ts"        # ~5 tests
  - "packages/msp/test/orchestrator/compressor/trim.test.ts"          # ~7 tests
  - "packages/msp/test/orchestrator/compressor/resummarise.test.ts"   # ~10 tests (LLM success / timeout / parse / truncate fallback)
  - "packages/msp/test/orchestrator/compressor/index.test.ts"         # ~13 end-to-end (mocks)

api_contracts:
  - name: compress
    signature: |
      function compress(opts: CompressOptions): Promise<CompressResult>
    types: |
      type Tokeniser = (s: string) => number

      interface CompressOptions {
        episodes: Episode[]                // from M7b/M7c
        budgetTokens: number
        llm?: LlmClient
        llmTimeoutMs?: number              // default 8000
        preserveOrder?: boolean            // default false
        tokeniser?: Tokeniser              // default char/3.5
      }
      interface CompressedEpisode {
        episodeRef: EpisodeRef
        text: string
        originalTokens: number
        compressedTokens: number
        compressedBy: 'keep' | 'trim' | 'resummarise' | 'truncated'
        droppedTurnIndices: number[]
        score: number
      }
      interface CompressResult {
        compressed: CompressedEpisode[]
        totalTokensUsed: number
        tierCounts: Record<'keep' | 'trim' | 'resummarise' | 'truncated' | 'dropped', number>
      }

verification_plan:
  - vitest tokens: 5 tests — char/3.5 default, custom tokeniser injection, empty string, mixed-language, integer ceiling
  - vitest trim: 7 tests — keep all if no candidates, drop low-score until fits, never drop high-score, ≥30% threshold gate, fits=false signal, preserves chronological order in output, idempotent
  - vitest resummarise: 10 tests — LLM success path, timeout → null, parse-error → null, no-llm → null, truncate keeps recent turns, truncate respects budget exactly, prompt formatted correctly, droppedIndices computed, mock LLM determinism
  - vitest index (end-to-end): 13 tests — all keep, all trim, all resummarise, mixed tiers, drop-last-can't-fit, importance-descending iteration, preserveOrder reorders output, totalTokensUsed correct, tierCounts correct, no-llm headless mode, idempotent, budget=0 → all dropped

  Test count: ~395 → ~430 (+35)

implementation_order:
  T1 TYPES         types.ts
  T2 TOKENS        tokens.ts + 5 tests
  T3 TRIM          trim.ts + 7 tests (re-uses tier1ScoreSingleTurn from M7b)
  T4 RESUMMARISE   resummarise.ts + 10 tests (LLM + truncate)
  T5 INDEX         index.ts orchestrator + 13 end-to-end
  T6 AUDIT         AUDIT--COMPRESSOR atom
```

## Implementation notes for the implementer

- **Run `npm ci` first** in worktree (CLAUDE.md caveat).
- **Re-use tier1ScoreSingleTurn from M7b** — `src/orchestrator/consolidator/score.ts` has `scoreChunk` for chunks; you may need to export a single-turn variant or call with `chunk.length === 1`.
- **No new deps**.
- **No Episode shape changes** — read-only consumer of M7b output.
- **Truncate keeps RECENT turns** (most recent first), not earliest. This is intentional: in chat history, recent context is usually most relevant.
- **Mock LlmClient for tests** — `createSlmClient({ provider: 'mock' })`.

## Implementer: do NOT do

- Mutate input `Episode[]`
- Add MCP wrapping (M7f)
- Add cross-episode dedup
- Add hierarchical summary-of-summaries
- Use a real tokeniser library by default (heuristic only; user can inject)
- Modify `src/orchestrator/consolidator/` or `src/memory/episodic/`

## Connections

- [[FEAT--COMPRESSOR]]
- [[ADR--COMPRESSOR-THREE-TIER]]
- [[CONCEPT--CONTEXT-COMPRESSION]]
