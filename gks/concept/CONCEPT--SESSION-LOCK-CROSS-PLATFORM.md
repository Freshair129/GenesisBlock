---
id: CONCEPT--SESSION-LOCK-CROSS-PLATFORM
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Session lock cross-platform parity — max-age safeguard for Windows +
  zombie PIDs
tags: &a1
  - msp
  - sessions
  - lock
  - windows
  - cross-platform
  - m9f
crosslinks: &a2
  references:
    - FEAT--MEMORY-SESSIONS-WRITER
    - CONCEPT--MEMORY-SESSIONS
created_at: 2026-05-05T16:22:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--SESSION-LOCK-CROSS-PLATFORM
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Session lock cross-platform parity — max-age safeguard for Windows +
    zombie PIDs
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-05T16:22:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--SESSION-LOCK-CROSS-PLATFORM
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Session lock cross-platform parity — max-age safeguard for Windows +
      zombie PIDs
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-05T16:22:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--SESSION-LOCK-CROSS-PLATFORM
      phase: 1
      type: concept
      status: stable
      vault_id: default
      tier: genesis
      source_type: axiomatic
      title: Session lock cross-platform parity — max-age safeguard for Windows +
        zombie PIDs
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-05T16:22:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Strategic intent / PRD
      attributes:
        domain: concept
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — session lock cross-platform parity

## Problem

`src/memory/sessions/lock.ts` implements an advisory lockfile for session writes:

- Open `.lock` with `O_EXCL` create
- Write the holder's PID
- On lock contention, check if holder PID is still alive via `process.kill(pid, 0)`
- Stale (dead PID) → remove and acquire

This works on POSIX. On Windows the picture is rougher:

1. **Zombie PIDs** — `process.kill(pid, 0)` may return alive for processes that are technically dead but still have an entry in the kernel's process table (Windows holds these longer than POSIX in some cases)
2. **Antivirus / Search Indexer file locks** — third-party tools can keep handles open on `.lock` files briefly, breaking exclusive create
3. **Network filesystems** (Samba, OneDrive) — exclusive create semantics weaker; may permit two processes to "create" the same lockfile in rare race windows
4. **Crashed Node before write** — process exits between `open('wx')` and `write(pid)`. Resulting empty lockfile is correctly treated as stale, but the window is wider on Windows due to slower IO

The impl handles (4) already (empty file → NaN → stale). It does NOT handle (1), (2), (3).

## What M9f adds

A **max-age safeguard**: if a lockfile's mtime is older than a threshold (default 5 minutes), it's considered stale regardless of PID-liveness. Defends against zombie PIDs + crashes that left a populated-but-orphan lockfile.

```ts
export async function acquire(lockPath: string, opts: { maxAgeMs?: number } = {}): Promise<LockHandle> {
  const maxAgeMs = opts.maxAgeMs ?? 5 * 60 * 1000  // 5 min default
  // ... existing logic with one extra check before the PID-liveness one:
  //   const stat = await fs.stat(lockPath)
  //   if (now - stat.mtimeMs > maxAgeMs) → stale, remove + retry
}
```

Also: when writing the PID, also write the timestamp:

```
<pid>\n<iso timestamp>\n
```

This is forward-compatible (existing lockfiles with just `<pid>` still parse). Future tools can read the timestamp directly without `fs.stat`.

## Why not adopt `proper-lockfile`

`proper-lockfile` is the conventional Node library for this. But:

- Adds a runtime dep (project policy: no new deps without strong reason)
- The existing impl is ~50 LoC and handles 95% of cases; M9f's safeguard closes most of the remaining 5%
- `proper-lockfile` itself isn't bulletproof on Windows (uses similar PID-liveness)

If real-world Windows adoption hits problems past M9f, swap-in is straightforward:

- `proper-lockfile.lock(path)` → returns a release function
- API surface is similar; one ADR + one PR to swap

## Other improvements considered but deferred

| Improvement | Defer reason |
|---|---|
| Heartbeat (re-touch lockfile mtime every 30s) | Adds complexity for a rare problem; max-age alone is enough |
| Per-platform branches | Premature; max-age works on all platforms |
| Retry-with-backoff for `EBUSY` (Windows AV) | Rare; user can retry manually |
| Lockfile schema versioning | Trivial enough; not yet needed |

## Invariants (preserved + new)

- **(existing)** Lockfile contains the holder's PID (now also: ISO timestamp on second line)
- **(existing)** Stale = PID dead → reclaim
- **(new)** Stale = lockfile mtime older than `maxAgeMs` → reclaim
- **(new)** `acquire()` accepts optional `maxAgeMs` (default 5 min)
- **(unchanged)** No new deps
- **(unchanged)** All public API in `lock.ts` keeps backward-compat (additive opt only)

## Out of scope

- Adopting `proper-lockfile` library
- Heartbeat / refresh
- Lockfile schema versioning
- Cross-process notification when lock released (callers already poll/retry)

## Source

`[[CONCEPT--MSP-ROADMAP]]` §3 M9f, user direction (all-M planning), existing `src/memory/sessions/lock.ts` audit.

## Connections

- [[FEAT--MEMORY-SESSIONS-WRITER]]
- [[CONCEPT--MEMORY-SESSIONS]]
