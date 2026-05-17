---
id: FEAT--MEMORY-SESSIONS-WRITER
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Sessions writer API — open / appendTurn / close
tags: &a1
  - msp
  - memory
  - sessions
  - writer
  - user-facing
crosslinks: &a2
  implements:
    - ADR--MEMORY-SESSIONS-WRITER
  references:
    - CONCEPT--MEMORY-SESSIONS-WRITER
    - CONCEPT--MEMORY-SESSIONS
linked_symbols: &a3
  - file: packages/msp/src/memory/sessions/writer.ts
  - file: packages/msp/src/memory/sessions/types.ts
  - file: packages/msp/src/memory/sessions/lock.ts
created_at: 2026-05-03T14:16:39.008+07:00
aliases: &a4
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  id: FEAT--MEMORY-SESSIONS-WRITER
  phase: 2
  type: feat
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: Sessions writer API — open / appendTurn / close
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-03T14:16:39.008+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Feature spec
  attributes:
    id: FEAT--MEMORY-SESSIONS-WRITER
    phase: 2
    type: feat
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: Sessions writer API — open / appendTurn / close
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-03T14:16:39.008+07:00
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

# FEAT — sessions writer

## User-facing behaviour

```ts
import { openSession } from '@/memory/sessions/writer'

const session = await openSession({ episodicId: 'ep_001', root: '.' })
await session.appendTurn({
  sessionId: 'sess_001',
  episodicId: 'ep_001',
  turnId: 1,
  msgId: 'm_abc',
  speakerId: 'user',
  content: 'hello',
})
await session.close()
```

`openSession` acquires the file lock and returns a writer object. `close()` releases it. If the process crashes, a stale lock is auto-cleaned on the next `openSession` for the same episodic.

## Acceptance criteria

- [ ] `openSession` creates `<episodicId>.jsonl` if missing; appends if present
- [ ] Two `openSession` calls for the same episodicId in different processes → second throws `SessionLockedError` with the holder's PID
- [ ] `appendTurn` rejects rows missing required fields with a typed `SessionSchemaError`
- [ ] `appendTurn` escapes embedded newlines in `content` so each JSONL row is a single line
- [ ] After `close()`, the lock file is removed
- [ ] If a previous process died without close, `openSession` cleans the stale lock and proceeds (logs a warning)
- [ ] Reading the file back with `readline` yields exactly the rows in append order
- [ ] vitest unit + integration tests cover all cases

## Surfaces

| Surface | Form |
|---|---|
| TS API | `openSession({ episodicId, root, lock? }): Promise<Session>` + `Session.appendTurn(row): Promise<void>` + `Session.close(): Promise<void>` |
| CLI | none for M-knowledge phase; M3+ exposes `msp-session append` for orchestrator scripting |
| MCP | future — `msp_session_append` tool |

## Out of scope

- Reading sessions (separate, trivially small).
- Episode summarisation (`[[FEAT--MEMORY-EPISODIC-WRITER]]`).
- Cross-platform Windows lock — M3 follow-up.

## Connections
- [[ADR--MEMORY-SESSIONS-WRITER]]
- [[CONCEPT--MEMORY-SESSIONS-WRITER]]
- [[CONCEPT--MEMORY-SESSIONS]]

