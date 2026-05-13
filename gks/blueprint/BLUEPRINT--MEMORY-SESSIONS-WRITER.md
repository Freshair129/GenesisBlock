---
id: BLUEPRINT--MEMORY-SESSIONS-WRITER
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — sessions writer implementation plan
tags:
  - msp
  - memory
  - sessions
  - blueprint
  - implementation
crosslinks: {"implements":["FEAT--MEMORY-SESSIONS-WRITER"],"references":["ADR--MEMORY-SESSIONS-WRITER","CONCEPT--MEMORY-SESSIONS"]}
linked_symbols:
  - {"file":"src/memory/sessions/writer.ts"}
  - {"file":"src/memory/sessions/types.ts"}
  - {"file":"src/memory/sessions/lock.ts"}
  - {"file":"src/memory/sessions/schema.ts"}
created_at: 2026-05-03T14:16:39.446+07:00
---

# BLUEPRINT — sessions writer

```yaml
metadata:
  title: "Memory sessions writer"
  parent_feat: FEAT--MEMORY-SESSIONS-WRITER

architectural_pattern: |
  Stateful façade (Session class) over four pure modules:
    - lock.ts     : flock-based advisory lock + stale-PID cleanup
    - schema.ts   : runtime row validation + content escape
    - writer.ts   : openSession factory + Session.appendTurn/close
    - types.ts    : TS interfaces shared with reader

data_logic: |
  openSession({ episodicId, root }):
    1. resolve sessionFile = `${root}/.brain/msp/projects/<ns>/sessions/${episodicId}.jsonl`
    2. acquire lock via lock.acquire(sessionFile + '.lock')
       - if held by live PID → throw SessionLockedError
       - if held by dead PID → cleanup + acquire
    3. mkdir -p parent dir if missing
    4. return Session instance with appendTurn + close

  Session.appendTurn(row):
    1. schema.validate(row) → throw SessionSchemaError on miss
    2. line = JSON.stringify(escapeNewlines(row))
    3. await fs.appendFile(sessionFile, line + '\n')

  Session.close():
    1. lock.release()
    2. mark session disposed; further calls throw

geography:
  - "src/memory/sessions/writer.ts"
  - "src/memory/sessions/lock.ts"
  - "src/memory/sessions/schema.ts"
  - "src/memory/sessions/types.ts"
  - "test/memory/sessions/writer.test.ts"
  - "test/memory/sessions/lock.test.ts"
  - "test/memory/sessions/schema.test.ts"

api_contracts:
  - name: openSession
    signature: |
      async function openSession(opts: OpenOpts): Promise<Session>
    types: |
      interface OpenOpts { episodicId: string; root: string; namespace?: string }
      interface Session {
        appendTurn(row: SessionTurn): Promise<void>
        close(): Promise<void>
      }
      interface SessionTurn {
        sessionId: string
        episodicId: string
        turnId: number
        msgId: string
        speakerId: string
        content: string
        learnId?: string
      }
      class SessionLockedError extends Error { holderPid: number }
      class SessionSchemaError extends Error { missingFields: string[] }

verification_plan:
  - vitest: schema accept/reject cases (one per required field omission + content with embedded \n)
  - vitest: lock.acquire holds; second acquire throws; release frees
  - vitest: stale lock (PID not alive) auto-cleaned on next acquire
  - integration: openSession + appendTurn × N + close → re-read with readline → exact row order
  - integration: two concurrent openSession on same episodicId → second throws SessionLockedError
```

## Implementation order

T1 OPEN-SESSION (lock + writer factory)
T2 APPEND-TURN (schema + serialise + appendFile)
T3 SCHEMA-VALIDATE (extracted module + tests)
