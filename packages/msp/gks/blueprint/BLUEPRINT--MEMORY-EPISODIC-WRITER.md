---
id: BLUEPRINT--MEMORY-EPISODIC-WRITER
phase: 3
type: blueprint
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — episodic writer implementation plan
tags:
  - msp
  - memory
  - episodic
  - blueprint
  - implementation
crosslinks: {"implements":["FEAT--MEMORY-EPISODIC-WRITER"],"references":["ADR--MEMORY-EPISODIC-WRITER","CONCEPT--MEMORY-EPISODIC"]}
linked_symbols:
  - {"file":"src/memory/episodic/writer.ts"}
  - {"file":"src/memory/episodic/types.ts"}
  - {"file":"src/memory/episodic/schema.ts"}
  - {"file":"src/memory/episodic/atomic-write.ts"}
  - {"file":"src/memory/episodic/summarisers/heuristic.ts"}
created_at: 2026-05-03T15:16:41.255+07:00
---

# BLUEPRINT — episodic writer

```yaml
metadata:
  title: "Memory episodic writer"
  parent_feat: FEAT--MEMORY-EPISODIC-WRITER

architectural_pattern: |
  Functional core (schema, atomic-write) + thin façade (writer.appendEpisode).
  Summarisers are plugins under summarisers/* — heuristic by default, LLM
  optional and provided by the orchestrator at call time.

data_logic: |
  appendEpisode(episode):
    1. schema.validate(episode) → throw EpisodicSchemaError on miss
    2. file = episodicMemoryPath(root, namespace)
    3. existing = await readJson(file) ?? []
    4. idx = existing.findIndex(e => e.episodicId === episode.episodicId)
    5. if idx >= 0: existing[idx] = episode  // overwrite (idempotent)
       else:        existing.push(episode); existing.sort(byTimestamp)
    6. await atomicWrite(file, JSON.stringify(existing, null, 2))

  appendEpisode.fromTurns({ episodicId, range, summariser, sessionRoot, ... }):
    1. read sessions/<episodicId>.jsonl with readline
    2. filter rows whose turnId in range
    3. summariser(turns) → EpisodeContent
    4. assemble episode object + call appendEpisode

geography:
  - "src/memory/episodic/writer.ts"             # public API
  - "src/memory/episodic/schema.ts"             # validate(episode)
  - "src/memory/episodic/atomic-write.ts"       # tmp + rename
  - "src/memory/episodic/summarisers/heuristic.ts"
  - "src/memory/episodic/types.ts"
  - "test/memory/episodic/writer.test.ts"
  - "test/memory/episodic/schema.test.ts"
  - "test/memory/episodic/atomic-write.test.ts"

api_contracts:
  - name: appendEpisode
    signature: |
      async function appendEpisode(episode: Episode, opts?: AppendOpts): Promise<void>
      appendEpisode.fromTurns(opts: FromTurnsOpts): Promise<void>
    types: |
      interface Episode {
        episodicId: string
        sessionId: string
        projectId: string
        timestamp?: string                 // defaults to now
        importance_score: number           // 0..1
        range: string[]                    // e.g. ['turnIdx-5..turnIdx-12']
        anchor?: { content: string; msgId: string }
        context?: { topic?: string; participants?: string[]; mood?: string }
        content: EpisodeContent
        tags?: string[]
        associations?: Associations
      }
      interface EpisodeContent {
        summary: string
        key_decisions?: string[]
        unresolved_questions?: string[]
      }
      interface Associations {
        related_event_ids?: string[]
        entity_links?: string[]
        knowledgeId?: string
        learnId?: string
      }
      type Summariser = (turns: SessionTurn[]) => Promise<EpisodeContent>
      class EpisodicSchemaError extends Error { reason: string }

verification_plan:
  - vitest: schema rejects missing/NaN/out-of-range importance_score
  - vitest: atomic-write recovers from partial writes (tmp file present, target unchanged)
  - vitest: heuristic summariser handles 0, 1, N turn inputs
  - integration: appendEpisode then re-appendEpisode with same episodicId → idempotent
  - integration: fromTurns with mock session JSONL fixture → expected episode shape
```

## Implementation order

T1 COLLECT-RANGE (read JSONL by turn range)
T2 SUMMARISE (heuristic plugin)
T3 APPEND-EPISODE (schema + atomic-write + idempotency)
