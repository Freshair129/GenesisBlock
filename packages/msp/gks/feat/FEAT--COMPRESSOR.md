---
id: FEAT--COMPRESSOR
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Context compressor — three-tier shrink-to-fit for token-budgeted episodes
tags:
  - msp
  - compression
  - compressor
  - token-budget
  - m7d
  - user-facing
crosslinks: {"implements":["ADR--COMPRESSOR-THREE-TIER"],"references":["CONCEPT--CONTEXT-COMPRESSION","FEAT--CONSOLIDATOR"]}
linked_symbols:
  - {"file":"src/orchestrator/compressor/index.ts"}
  - {"file":"src/orchestrator/compressor/types.ts"}
  - {"file":"src/orchestrator/compressor/tokens.ts"}
  - {"file":"src/orchestrator/compressor/trim.ts"}
  - {"file":"src/orchestrator/compressor/resummarise.ts"}
created_at: 2026-05-05T09:11:00.000Z
---

# Context compressor — three-tier shrink-to-fit for token-budgeted episodes

## User-facing API

```ts
import { compress } from '@/orchestrator/compressor'
import { createSlmClient } from '@/codegen/slm/factory'

const result = await compress({
  episodes: episodicHits,         // Episode[] from M7c or store
  budgetTokens: 4000,
  llm: createSlmClient({ provider: 'auto' }),  // optional
  preserveOrder: false,           // optional; default false (importance-desc)
  tokeniser: undefined,           // optional injection; defaults to char/3.5
})

result.compressed                  // CompressedEpisode[] — fits budget
result.totalTokensUsed             // number, ≤ budgetTokens
result.tierCounts                  // { keep: 3, trim: 2, resummarise: 1, truncated: 0, dropped: 1 }
```

```ts
interface CompressedEpisode {
  episodeRef: { sessionId: string; turnRange: [number, number] } | { atomId: string }
  text: string                    // joined turn text, post-compression
  originalTokens: number
  compressedTokens: number
  compressedBy: 'keep' | 'trim' | 'resummarise' | 'truncated'
  droppedTurnIndices: number[]    // empty for keep / resummarise
  score: number                   // carried from input episode
}
```

## Acceptance criteria

- [ ] `compress(opts)` returns `CompressResult` with `compressed[]`, `totalTokensUsed`, `tierCounts`
- [ ] **Total tokens ≤ budgetTokens** — enforced; selection drops episodes if even summaries don't fit
- [ ] **Importance-descending iteration** — high-importance gets keep/trim, low gets resummarise/truncate/drop
- [ ] **Tier choice** per `ADR--COMPRESSOR-THREE-TIER`:
  - whole-fits → keep
  - ≥ 30% droppable + trimmed-fits → trim
  - has llm → resummarise (target 0.6 × original)
  - no llm OR resummarise fails → truncate (drop last turns)
- [ ] **opts.preserveOrder** reorders OUTPUT to chronological after selection (default false → importance order)
- [ ] **opts.tokeniser** injection — defaults to `Math.ceil(s.length / 3.5)`
- [ ] **No LLM = headless OK** — truncate fallback always available
- [ ] **No mutation** — input episodes unchanged
- [ ] **Provenance preserved** — every output has `episodeRef`
- [ ] **Idempotent** with mock LLM
- [ ] Test target ~395 → ~430 (+35)

## Surfaces

| Surface | Form |
|---|---|
| TS API | `compress(opts: CompressOptions): Promise<CompressResult>` |
| Sub-modules | `tokens.ts` (estimate / token math), `trim.ts` (turn-drop), `resummarise.ts` (LLM call + truncate fallback) |
| Tests | `test/orchestrator/compressor/{tokens,trim,resummarise,index}.test.ts` |

## Out of scope

- MCP tool wrapping (`msp_compress`) → M7f
- Cross-episode dedup
- Hierarchical summary-of-summaries
- Vector-similarity-based filtering before compression
