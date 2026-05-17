---
id: FEAT--CONSOLIDATOR
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Consolidator — session → episode hybrid-scored gate
tags: &a1
  - msp
  - consolidator
  - importance
  - summarisation
  - m7b
  - user-facing
crosslinks: &a2
  implements:
    - ADR--CONSOLIDATOR-HYBRID-SCORING
  references:
    - CONCEPT--CONSOLIDATOR
    - FEAT--MEMORY-EPISODIC-WRITER
    - FEAT--MEMORY-SESSIONS-WRITER
linked_symbols: &a3
  - file: packages/msp/src/orchestrator/consolidator/index.ts
  - file: packages/msp/src/orchestrator/consolidator/types.ts
  - file: packages/msp/src/orchestrator/consolidator/score.ts
  - file: packages/msp/src/orchestrator/consolidator/boundary.ts
  - file: packages/msp/src/orchestrator/consolidator/summarise.ts
created_at: 2026-05-04T17:06:00.000+07:00
aliases: &a4
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  id: FEAT--CONSOLIDATOR
  phase: 2
  type: feat
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: Consolidator — session → episode hybrid-scored gate
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-04T17:06:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Feature spec
  attributes:
    id: FEAT--CONSOLIDATOR
    phase: 2
    type: feat
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: Consolidator — session → episode hybrid-scored gate
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-04T17:06:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Feature spec
    attributes:
      domain: feat
    domain: feat
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: feat
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# Consolidator — session → episode hybrid-scored gate

## User-facing API

```ts
import { consolidate } from '@/orchestrator/consolidator'

const episodes = await consolidate({
  sessionId: 'sess-2026-05-04-abc',
  root: process.cwd(),
  llm: createSlmClient({ provider: 'auto' }),  // optional
  options: {
    thresholds: { low: 0.30, high: 0.65 },     // optional, defaults from ADR
    maxLlmCallsPerSession: 20,                 // optional
    llmCallTimeoutMs: 8000,                    // optional
  },
})
// episodes is Episode[]; pass to EpisodicWriter to persist.
```

```ts
interface Episode {
  sessionId: string
  turnRange: [number, number]    // inclusive indices into session.jsonl
  summary: string                // 1–3 sentences
  tags: string[]                 // 3–5 keywords
  score: number                  // final importance 0..1
  scoreSource: 'tier1' | 'tier2' | 'tier2-default'
  createdAt: string              // ISO 8601
}
```

## Acceptance criteria

- [ ] `consolidate(opts)` reads session turns from `.brain/msp/projects/<ns>/sessions/<sessionId>.jsonl` via existing reader
- [ ] Returns `Episode[]` — does NOT write to episodic store (caller decides persistence)
- [ ] **Tier-1 deterministic** scoring: 7 features per `[[ADR--CONSOLIDATOR-HYBRID-SCORING]]` (decision_markers, code_artifact_mentions, numeric_specificity, length_normalised, topic_continuity, dead_end_markers, greeting_filler)
- [ ] **Tier-1 verdicts**: `< low_thresh` → drop; `> high_thresh` → keep; otherwise → tier-2
- [ ] **Tier-2 LLM**: only invoked for borderline; bounded by `maxLlmCallsPerSession` + `llmCallTimeoutMs`; timeout / parse-error / no-provider → default to keep
- [ ] **Boundary detection**: chunks turns into contiguous episodes via topic-continuity threshold
- [ ] **Summariser**: tier-2 LLM also returns summary + tags (single call covers both jobs); tier-1-keep chunks get a deterministic summary (first sentence + frontmatter title-extraction)
- [ ] **Idempotent**: same session twice → same Episode[] (modulo LLM-side non-determinism, mitigated via `mock` provider in tests)
- [ ] **No mutation** of source `session.jsonl`
- [ ] All 4 sub-modules unit-testable in isolation
- [ ] Test target: 295 → ~330 (+35: ~5 boundary, ~10 score-features, ~10 LLM-caller, ~5 integration, ~5 fixture-driven end-to-end)

## Surfaces

| Surface | Form |
|---|---|
| TS API | `consolidate(opts: ConsolidateOptions): Promise<Episode[]>` |
| Sub-module: scorer | `scoreChunk(turns, opts) → { score, verdict, source }` |
| Sub-module: boundary | `detectBoundaries(turns, opts) → [start, end][]` |
| Sub-module: summariser | `summarise(turns, llm?, opts) → { summary, tags }` |
| Tests | `test/orchestrator/consolidator/{score,boundary,summarise,index}.test.ts` |

## Out of scope

- MCP tool wrapping (`msp_remember`) → M7f
- Session-end hook auto-call → agent harness
- Cross-session episode dedup → M7c
- `valid_until` decay / forgetting → M9
- Threshold tuning (`PARAM--` atom) → M9

## Connections
- [[CONCEPT--CONSOLIDATOR]]
- [[FEAT--MEMORY-EPISODIC-WRITER]]
- [[FEAT--MEMORY-SESSIONS-WRITER]]

