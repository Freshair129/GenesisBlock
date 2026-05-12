---
id: FEAT--MEMORY-EPISODIC-WRITER
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Episodic writer API — appendEpisode + summariser plugin
tags:
  - msp
  - memory
  - episodic
  - writer
  - user-facing
crosslinks: {"implements":["ADR--MEMORY-EPISODIC-WRITER"],"references":["CONCEPT--MEMORY-EPISODIC-WRITER","CONCEPT--MEMORY-EPISODIC"]}
linked_symbols:
  - {"file":"src/memory/episodic/writer.ts"}
  - {"file":"src/memory/episodic/types.ts"}
  - {"file":"src/memory/episodic/summarisers/heuristic.ts"}
created_at: 2026-05-03T15:16:40.829+07:00
---

# FEAT — episodic writer

## User-facing behaviour

```ts
import { appendEpisode, heuristicSummariser } from '@/memory/episodic/writer'

await appendEpisode({
  episodicId: 'ep_001',
  sessionId:  'sess_001',
  projectId:  'evaAI',
  range:      ['turnIdx-5..turnIdx-12'],
  importance_score: 0.85,
  content: {
    summary: 'Decided on token-bucket rate limiter per tenant.',
    key_decisions: ['ADR--RATE-LIMIT'],
    unresolved_questions: ['What happens at tenant boundary?'],
  },
  tags: ['rate-limit'],
})
```

Or with a summariser plugin over the session JSONL:

```ts
await appendEpisode.fromTurns({
  episodicId: 'ep_002',
  sessionId:  'sess_002',
  projectId:  'evaAI',
  range:      ['turnIdx-1..turnIdx-9'],
  importance_score: 0.6,
  summariser: heuristicSummariser,
  sessionRoot: '.',
})
```

## Acceptance criteria

- [ ] `appendEpisode` rejects when `importance_score` is missing, NaN, or outside `[0, 1]`
- [ ] Calling with the same `episodicId` twice overwrites the existing entry (idempotent), not duplicates
- [ ] Atomic-rewrite via `<file>.tmp` + `rename` — partial writes cannot leave the JSON malformed
- [ ] `appendEpisode.fromTurns` reads the session JSONL within `range` and feeds turns into the summariser
- [ ] Default `heuristicSummariser` returns content with at least `summary` non-empty
- [ ] Custom summariser plugin honoured if passed
- [ ] vitest unit + integration tests cover all cases

## Surfaces

| Surface | Form |
|---|---|
| TS API | `appendEpisode(episode)`, `appendEpisode.fromTurns(opts)`, `heuristicSummariser` |
| CLI | none initially; M3+ exposes `msp-episode append` |
| MCP | future — `msp_episode_append` tool |

## Out of scope

- LLM-backed summariser (orchestrator plugin).
- Vector embedding (orchestrator concern).
- Reading episodes (small reader module added with the writer; not a separate FEAT).
