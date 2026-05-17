---
id: AUDIT--MEMORY-EPISODIC-WRITER
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M3c-3 memory episodic writer acceptance audit
tags: &a1
  - msp
  - m3
  - m3c
  - audit
  - memory
  - episodic
crosslinks: &a2
  references:
    - FEAT--MEMORY-EPISODIC-WRITER
    - BLUEPRINT--MEMORY-EPISODIC-WRITER
    - ADR--MEMORY-EPISODIC-WRITER
linked_symbols: &a3
  - file: packages/msp/src/memory/episodic/writer.ts
  - file: packages/msp/src/memory/episodic/schema.ts
  - file: packages/msp/src/memory/episodic/atomic-write.ts
  - file: packages/msp/src/memory/episodic/summarisers/heuristic.ts
  - file: packages/msp/src/memory/episodic/types.ts
created_at: 2026-05-03T15:43:38.623+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--MEMORY-EPISODIC-WRITER
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: M3c-3 memory episodic writer acceptance audit
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-03T15:43:38.623+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--MEMORY-EPISODIC-WRITER
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: M3c-3 memory episodic writer acceptance audit
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-03T15:43:38.623+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      domain: audit
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — memory episodic writer

## Scope

Closes [[FEAT--MEMORY-EPISODIC-WRITER]]. Implementation follows BLUEPRINT geography exactly.

## Acceptance criteria from FEAT

| # | Criterion | Result |
|---|---|---|
| 1 | Rejects when `importance_score` missing/NaN/out-of-range | ✅ test (3 cases) |
| 2 | Same `episodicId` overwrites (idempotent), not duplicates | ✅ test |
| 3 | Atomic-rewrite via `<file>.tmp` + rename | ✅ test |
| 4 | `appendEpisode.fromTurns` reads session JSONL within range | ✅ test |
| 5 | Default heuristic summariser returns non-empty `summary` | ✅ test |
| 6 | Custom summariser plugin honoured | ✅ test |
| 7 | vitest unit + integration tests | ✅ 19/19 |

## Test summary

```
test/memory/episodic/schema.test.ts:    7/7 passing
test/memory/episodic/heuristic.test.ts: 5/5 passing
test/memory/episodic/writer.test.ts:    7/7 passing
total: 19/19
```

## Heuristic summariser behaviour

- Anchor: first non-trivial assistant turn (≥ 20 chars), else first turn.
- Decisions: sentences matching `\b(decided|chose|will|going to|let's|let us)\b`, capped at 5.
- Summary truncated to 240 chars (one line).

LLM-backed summarisers are pluggable via `opts.summariser`; orchestrator concern.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 19/19 tests
- Date: 2026-05-03

## Connections
- [[BLUEPRINT--MEMORY-EPISODIC-WRITER]]
- [[ADR--MEMORY-EPISODIC-WRITER]]

