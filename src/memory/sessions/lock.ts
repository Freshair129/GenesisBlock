import { open, readFile, rm, writeFile } from 'node:fs/promises'

import { SessionLockedError } from './types.js'

interface LockHandle {
  release(): Promise<void>
}

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
 * holder's PID. A stale lock (PID no longer alive) is auto-cleaned and
 * acquired.
 */
export async function acquire(lockPath: string): Promise<LockHandle> {
  for (;;) {
    try {
      const fh = await open(lockPath, 'wx') // exclusive create
      await fh.write(String(process.pid))
      await fh.close()
      return {
        async release() {
          await rm(lockPath, { force: true })
        },
      }
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== 'EEXIST') throw err
    }

    // Lock exists. Check if holder is alive.
    const holderText = await readFile(lockPath, 'utf8').catch(() => '')
    const holderPid = Number.parseInt(holderText, 10)
    if (!Number.isFinite(holderPid) || !isAlive(holderPid)) {
      // Stale — remove and retry.
      await rm(lockPath, { force: true })
      continue
    }
    throw new SessionLockedError(holderPid, lockPath)
  }
}
