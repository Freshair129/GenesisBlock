---
id: CONCEPT--MEMORY-EPISODIC
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Memory episodic — high-importance event summaries with associations
tags:
  - msp
  - memory
  - episodic
crosslinks: {"references":["CONCEPT--MEMORY-SUBSYSTEM","CONCEPT--MEMORY-SESSIONS"]}
created_at: 2026-05-03T14:01:53.831+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — memory episodic

Where the session JSONL gives you exact rows-by-turn, the episodic store summarises the *important* moments — decisions, surprises, blockers — into a queryable form. Each episode wraps a `range` of turns with a one-paragraph summary, key decisions, unresolved questions, and links to other knowledge.

## Path

```
.brain/msp/projects/<ns>/memory/episodic_memory.json
```

A single JSON file with an array of episode objects (or one object per file in folder mode — orchestrator's choice).

## Schema (one episode)

```json
{
  "episodicId":       "ep_001",
  "sessionId":        "sess_001",
  "projectId":        "evaAI",
  "timestamp":        "2026-04-18T10:30:00Z",
  "importance_score": 0.85,
  "range":            ["turnIdx-x"],
  "anchor":           { "content": "...", "msgId": "..." },
  "context":          { "topic": "...", "participants": [], "mood": "..." },
  "content": {
    "summary":             "...",
    "key_decisions":       [],
    "unresolved_questions":[]
  },
  "tags": [],
  "associations": {
    "related_event_ids":   [],
    "entity_links":        [],
    "knowledgeId":         "FEAT--xxx",
    "learnId":             "ALGO--xxx"
  }
}
```

| Field | Required | Notes |
|---|---|---|
| `episodicId` | yes | matches the file in `sessions/<episodicId>.jsonl` |
| `importance_score` | yes | 0..1; cheap heuristic OR LLM-rated; drives retrieval ranking |
| `range` | yes | `["turnIdx-12..turnIdx-19"]` style — which turns this episode covers |
| `anchor` | yes | the single turn that best represents the episode |
| `content.summary` | yes | one-paragraph natural language; this is what semantic search hits |
| `content.key_decisions` | yes | bullet list of what was decided; usually atom IDs |
| `content.unresolved_questions` | yes | open threads the next session must pick up |
| `associations.knowledgeId` | no | atom this episode produced (e.g. a new ADR) |

## Read pattern

```
1. semantic search (vector) over .summary  →  candidate episodicIds
2. for each candidate, fetch full episode  →  decide which to load
3. for the chosen episode, optionally pull turn range from sessions/<id>.jsonl
```

This three-step ladder is why MSP keeps the JSONL + JSON split — the episodic file is small enough to vector-index; the JSONL stays compact and unread until needed.

## Source

`msp_spec.md` §7.2 (Rich Episodic Memory).

## Connections
- [[CONCEPT--MEMORY-SUBSYSTEM]]
- [[CONCEPT--MEMORY-SESSIONS]]

