---
id: ADR--SESSION-LOCK-MAX-AGE
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Session lock — max-age safeguard (Windows / zombie PID parity)
tags:
  - msp
  - sessions
  - lock
  - windows
  - decision
  - m9f
crosslinks: {"references":["CONCEPT--SESSION-LOCK-CROSS-PLATFORM"],"implements":["FEAT--MEMORY-SESSIONS-WRITER"]}
created_at: 2026-05-05T09:22:00.000Z
---

# ADR — session lock max-age

## Context

`acquire(lockPath)` in `src/memory/sessions/lock.ts` uses PID-liveness probing. On Windows + edge cases (zombie PIDs, network filesystems, antivirus interference), the PID-alive probe is unreliable, leading to permanent wedged locks.

Per `CONCEPT--SESSION-LOCK-CROSS-PLATFORM`, a max-age safeguard closes most of this gap without adding deps.

## Decision

Add `maxAgeMs?: number` option to `acquire()` (default 5 minutes). If the lockfile's mtime is older than the threshold, treat it as stale regardless of PID-liveness, and reclaim.

Also extend the lockfile content shape to include an ISO timestamp (forward-compatible with existing PID-only lockfiles):

```
<pid>
<isoTimestamp>
```

Existing readers see `parseInt(holderText, 10)` which only consumes the first line — backward-compatible.

## Implementation

### `acquire(lockPath, opts)`

```ts
export async function acquire(
  lockPath: string,
  opts: { maxAgeMs?: number; now?: () => number } = {},
): Promise<LockHandle>
```

- `now?: () => number` — injectable clock for tests; default `Date.now`
- `maxAgeMs?: number` — default `5 * 60 * 1000` (5 minutes)

Pseudo-code:

```ts
for (;;) {
  // 1. Try exclusive create (unchanged from before)
  try {
    const fh = await open(lockPath, 'wx')
    await fh.write(`${process.pid}\n${new Date().toISOString()}\n`)
    await fh.close()
    return { release() { return rm(lockPath, { force: true }) } }
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code !== 'EEXIST') throw err
  }

  // 2. NEW: max-age check
  let stat: Stats | null
  try { stat = await fs.stat(lockPath) } catch { stat = null }
  if (stat && (now() - stat.mtimeMs > maxAgeMs)) {
    await rm(lockPath, { force: true })
    continue
  }

  // 3. Existing PID-liveness check
  const text = await readFile(lockPath, 'utf8').catch(() => '')
  const pid = Number.parseInt(text, 10)
  if (!Number.isFinite(pid) || !isAlive(pid)) {
    await rm(lockPath, { force: true })
    continue
  }
  throw new SessionLockedError(pid, lockPath)
}
```

### Lockfile format

```
12345
2026-05-05T09:25:30.000Z
```

Line 1: holder PID (existing).
Line 2: ISO 8601 timestamp of acquisition (new). Future tools can use this directly without `fs.stat`.

Both lines preserved across writers; readers tolerate either format.

## Why max-age before PID-liveness

Order matters:
1. Old lockfiles (process long-dead, OS reused PID) → max-age catches this; PID-liveness would falsely report "alive" because some other process now holds that PID
2. Zombie PIDs (process gone but kernel-table-resident) → max-age catches this regardless of zombie state
3. Crashed process with empty lockfile → existing NaN-stale path still works; max-age doesn't even fire

## Consequences

**Positive**
- No new deps
- Forward-compat lockfile shape (existing PID-only files still parse)
- 5-minute default chosen so that legitimate long-running consolidations (M7b — ~30s) don't trip; only abandoned locks
- Fixes the Windows zombie-PID class without platform branches

**Negative**
- A legitimate caller that holds the lock > 5 minutes will be evicted. Mitigated: 5 min is way more than any realistic session-write op; callers needing more should pass `maxAgeMs` explicitly.
- Heartbeat (re-touch mtime mid-hold) NOT implemented — would catch the long-held-legitimate case but adds complexity. Deferred.
- `fs.stat` is one extra syscall per contention loop. Acceptable.

## Alternatives considered

1. **Adopt `proper-lockfile`** — adds runtime dep; existing impl + max-age covers ≥ 95% of cases.
2. **Heartbeat** — re-touch lockfile every 30s while held. Adds complexity; defer to M9-followup if needed.
3. **Reduce default to 1 minute** — too aggressive; would clip legitimate longer ops.
4. **Per-platform branches** — premature.

## What this ADR does NOT change

- Public API signature is additive (new optional opts)
- `release()` semantics unchanged
- `SessionLockedError` semantics unchanged
- Cross-process lock visibility unchanged
- No changes to writers / consumers of `acquire()`

## Source

`CONCEPT--SESSION-LOCK-CROSS-PLATFORM`, `CONCEPT--MSP-ROADMAP` §3 M9f, audit of existing `src/memory/sessions/lock.ts`.
