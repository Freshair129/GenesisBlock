---
id: ADR--MEMORY-SESSIONS-WRITER
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Sessions writer is append-only with file-level lock
tags:
  - msp
  - memory
  - sessions
  - writer
  - decision
crosslinks: {"references":["CONCEPT--MEMORY-SESSIONS-WRITER","CONCEPT--MEMORY-SESSIONS"]}
created_at: 2026-05-03T14:16:38.566+07:00
---

# ADR — sessions writer concurrency model

## Context

Two writers can race on the same `<episodicId>.jsonl` and produce torn lines (interleaved bytes mid-row). On rotational disks, append is atomic for writes < `PIPE_BUF` (4 KiB on Linux), but our content field can exceed that easily. Python and Node both reuse the same fd across async turns, which is fine; the actual risk is two *processes* opening the same episodic.

## Decision

**File-level advisory lock per `episodicId`** plus **append-only writes**:

1. On `openSession(episodicId)`:
   - Acquire `flock` (Linux) / equivalent on `<episodicId>.jsonl.lock` (sibling file).
   - If lock held by another PID → throw `SessionLockedError` with the holder's PID.
   - Hold the lock until `closeSession()` or process exit.
2. On `appendTurn(episodicId, row)`:
   - Validate row against schema (sessionId, episodicId, turnId, msgId, speakerId, content present; learnId optional).
   - Serialise to one line (no embedded newlines — escape if content contains them).
   - `fs.appendFile` (single syscall; atomic for our line sizes after escape).
3. On `closeSession()`:
   - Flush, release the flock, delete the lock file.

Re-entry on crash: a stale `.lock` file with a dead PID is removed on next `openSession`.

## Consequences

**Positive**
- One writer per episodic enforced cheaply.
- Append-only means historical rows are immutable — no torn writes from in-place edits.
- Recovery from crash is automatic on next open.

**Negative**
- Lock files visible in directory listings — slightly noisy. Acceptable.
- `flock` is POSIX; Windows needs different syscall (use `proper-lockfile` package or similar). Cross-platform is M3 work.

## Alternatives considered

1. **Per-row file (one file per turn).** Rejected — explodes file count; defeats `cat session.jsonl` workflow.
2. **SQLite per session.** Considered. Too heavy for a flat append log; we don't need queries on this layer.
3. **No lock; trust single-writer convention.** Rejected — silent corruption in concurrent scenarios is worse than a clear lock error.

## Source

`[[CONCEPT--MEMORY-SESSIONS-WRITER]]` + `[[CONCEPT--MEMORY-SESSIONS]]`.
