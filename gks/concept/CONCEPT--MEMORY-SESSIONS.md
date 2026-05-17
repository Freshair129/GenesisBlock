---
id: CONCEPT--MEMORY-SESSIONS
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Memory sessions — append-only JSONL turn log
tags:
  - msp
  - memory
  - sessions
  - jsonl
crosslinks: {"references":["CONCEPT--MEMORY-SUBSYSTEM"]}
created_at: 2026-05-03T14:01:53.330+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — memory sessions

Linear, append-only record of every turn in a session. One JSONL row per turn. Used to replay an exact conversation; never queried for retrieval (use episodic + vector for that).

## Path

```
.brain/msp/projects/<ns>/sessions/<episodicId>.jsonl
```

`<episodicId>` is a stable handle for the session — typically `sess_<timestamp>` or `sess_<ulid>`. Using `episodicId` as the filename makes it cheap to find all turns of one session without grep across files.

## Schema (one row per turn)

```json
{
  "sessionId":   "<string>",
  "episodicId":  "<string>",
  "turnId":      <number>,
  "msgId":       "<string>",
  "speakerId":   "user | MSP-AGT-XXX",
  "content":     "<string>",
  "learnId":     "<knowledge-id-if-any>"
}
```

| Field | Required | Notes |
|---|---|---|
| `sessionId` | yes | matches the session's launch identifier |
| `episodicId` | yes | groups turns into an episode (often = sessionId for short sessions) |
| `turnId` | yes | monotonically increasing integer per session |
| `msgId` | yes | unique per turn; used to anchor episodic summaries |
| `speakerId` | yes | `user`, `MSP-AGT-CLAUDE-OPUS-4-7`, `MSP-AGT-GEMINI-2`, etc. |
| `content` | yes | raw turn content; can be long |
| `learnId` | no | atomic ID this turn produced or referenced (e.g. `[[FEAT--MSP-VALIDATOR]]`) |

## Append semantics

- Files are append-only — never rewrite or delete rows in place.
- One process writes one session at a time (no concurrent writers per file).
- Truncation is allowed only in offline maintenance scripts, never at runtime.

## Retention

Spec doesn't mandate; orchestrators typically keep 30–90 days then archive to cold storage. This is the orchestrator's call, not MSP's.

## Source

`msp_spec.md` §7.1 (Linear Session History).

## Connections
- [[CONCEPT--MEMORY-SUBSYSTEM]]

