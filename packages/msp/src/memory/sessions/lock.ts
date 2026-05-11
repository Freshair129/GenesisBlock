import { open, readFile, rm, stat } from 'node:fs/promises'

import { SessionLockedError } from './types.js'

interface LockHandle {
  release(): Promise<void>
}

export interface AcquireOptions {
  /**
   * Max age of a lockfile before it's considered stale regardless of
   * holder-PID liveness. Defends against zombie PIDs (Windows), reused
   * PIDs after long-running uptime, and antivirus / network-FS edge cases.
   * Default: 5 minutes.
   */
  maxAgeMs?: number
  /** Injectable clock for tests; default Date.now. */
  now?: () => number
}

const DEFAULT_MAX_AGE_MS = 5 * 60 * 1000

function isAlive(pid: number): boolean {
  try {
    // signal 0 is a permission/existence probe — does not actually kill.
    process.kill(pid, 0)
    return true
  } catch (err) {
    // ESRCH = no such process. EPERM = exists but we can't signal.
    return (err as NodeJS.ErrnoException).code === 'EPERM'
  }
}

/**
 * Acquire a per-file advisory lock. Sibling `<path>.lock` records the
 * holder's PID + acquisition timestamp.
 *
 * Stale lock detection (in order):
 *   1. mtime older than `opts.maxAgeMs` (default 5 min) → reclaim
 *   2. holder PID dead → reclaim
 *   3. holder PID alive AND mtime fresh → throw SessionLockedError
 *
 * The max-age safeguard is M9f's contribution — closes the
 * Windows / zombie-PID gap without adding new deps.
 */
export async function acquire(
  lockPath: string,
  opts: AcquireOptions = {},
): Promise<LockHandle> {
  const maxAgeMs = opts.maxAgeMs ?? DEFAULT_MAX_AGE_MS
  const now = opts.now ?? Date.now

  for (;;) {
    try {
      const fh = await open(lockPath, 'wx') // exclusive create
      // Line 1: PID (back-compat; older readers parseInt the first line).
      // Line 2: ISO timestamp (M9f addition; future tools can read directly).
      await fh.write(`${process.pid}\n${new Date(now()).toISOString()}\n`)
      await fh.close()
      return {
        async release() {
          await rm(lockPath, { force: true })
        },
      }
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== 'EEXIST') throw err
    }

    // Lock exists. Two stale checks before honouring it.

    // (1) Max-age safeguard — handles zombie PIDs / reused PIDs / crashed
    //     processes whose lockfile is left behind on Windows.
    let lockStat
    try {
      lockStat = await stat(lockPath)
    } catch {
      // Race: lockfile vanished between EEXIST and stat. Loop and retry create.
      continue
    }
    if (now() - lockStat.mtimeMs > maxAgeMs) {
      await rm(lockPath, { force: true })
      continue
    }

    // (2) PID-liveness — handles the common "process exited cleanly" case.
    const holderText = await readFile(lockPath, 'utf8').catch(() => '')
    const holderPid = Number.parseInt(holderText, 10)
    if (!Number.isFinite(holderPid) || !isAlive(holderPid)) {
      // Stale by liveness — remove and retry.
      await rm(lockPath, { force: true })
      continue
    }

    throw new SessionLockedError(holderPid, lockPath)
  }
}
