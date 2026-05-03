---
id: AUDIT--MEMORY-SESSIONS-WRITER
phase: 6
type: audit
status: stable
vault_id: default
title: M3c-2 memory sessions writer acceptance audit
tags:
  - msp
  - m3
  - m3c
  - audit
  - memory
  - sessions
crosslinks: {"references":["FEAT--MEMORY-SESSIONS-WRITER","BLUEPRINT--MEMORY-SESSIONS-WRITER","ADR--MEMORY-SESSIONS-WRITER"]}
linked_symbols:
  - {"file":"src/memory/sessions/writer.ts"}
  - {"file":"src/memory/sessions/lock.ts"}
  - {"file":"src/memory/sessions/schema.ts"}
  - {"file":"src/memory/sessions/types.ts"}
created_at: 2026-05-03T08:43:38.014Z
---

# AUDIT — memory sessions writer

## Scope

Closes FEAT--MEMORY-SESSIONS-WRITER. Implementation follows BLUEPRINT geography exactly.

## Acceptance criteria from FEAT

| # | Criterion | Result |
|---|---|---|
| 1 | `openSession` creates `<episodicId>.jsonl` if missing | ✅ test |
| 2 | Concurrent open → 2nd throws `SessionLockedError(holderPid)` | ✅ test (in-process) |
| 3 | `appendTurn` rejects missing fields with `SessionSchemaError` | ✅ test |
| 4 | Embedded newlines in `content` escaped (one row per line) | ✅ test |
| 5 | After `close()`, lock file removed | ✅ test |
| 6 | Stale lock from dead PID → cleaned + acquired | ✅ test (cross-process) |
| 7 | Re-read with readline → exact append order | ✅ test |
| 8 | vitest unit + integration tests | ✅ 20/20 |

## Test summary

```
test/memory/sessions/schema.test.ts: 7/7 passing
test/memory/sessions/lock.test.ts:    5/5 passing
test/memory/sessions/writer.test.ts:  8/8 passing
total: 20/20
```

## Cross-platform note

Lock implementation uses POSIX `fs.open(wx)` + `process.kill(pid, 0)`. Windows compatibility (proper-lockfile or similar) deferred per ADR--MEMORY-SESSIONS-WRITER.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 20/20 tests including cross-process stale-lock recovery
- Date: 2026-05-03
