import { mkdtemp, readFile, utimes, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { acquire } from '../../../src/memory/sessions/lock.js'
import { SessionLockedError } from '../../../src/memory/sessions/types.js'

async function freshLockPath(): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), 'msp-lock-'))
  return join(dir, 'session.lock')
}

describe('acquire', () => {
  it('creates the lock file holding our PID', async () => {
    const path = await freshLockPath()
    const h = await acquire(path)
    const text = await readFile(path, 'utf8')
    expect(Number.parseInt(text, 10)).toBe(process.pid)
    await h.release()
  })

  it('release removes the lock file', async () => {
    const path = await freshLockPath()
    const h = await acquire(path)
    await h.release()
    await expect(readFile(path)).rejects.toThrow()
  })

  it('throws SessionLockedError when held by a live process', async () => {
    const path = await freshLockPath()
    const a = await acquire(path)
    await expect(acquire(path)).rejects.toBeInstanceOf(SessionLockedError)
    await a.release()
  })

  it('cleans a stale lock (PID not alive) and acquires', async () => {
    const path = await freshLockPath()
    // Plant a lock with an obviously dead PID. PID 999999 is unlikely to exist.
    await writeFile(path, '999999')
    const h = await acquire(path)
    const text = await readFile(path, 'utf8')
    expect(Number.parseInt(text, 10)).toBe(process.pid)
    await h.release()
  })

  it('cleans a lock with garbage contents and acquires', async () => {
    const path = await freshLockPath()
    await writeFile(path, 'not-a-pid')
    const h = await acquire(path)
    await h.release()
  })

  // M9f — max-age safeguard for Windows / zombie-PID parity.

  it('writes lockfile in two-line PID + ISO-timestamp format (M9f)', async () => {
    const path = await freshLockPath()
    const h = await acquire(path)
    const text = await readFile(path, 'utf8')
    const [line1, line2] = text.split('\n')
    expect(Number.parseInt(line1, 10)).toBe(process.pid)
    expect(line2).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/) // ISO 8601 prefix
    await h.release()
  })

  it('reclaims a lock whose mtime is older than maxAgeMs (M9f)', async () => {
    const path = await freshLockPath()
    // Plant a lock with a very-much-alive PID (our own) so PID-liveness
    // would NOT mark it stale; max-age must be the safeguard that kicks in.
    await writeFile(path, `${process.pid}\n2026-01-01T00:00:00.000Z\n`)
    // Backdate mtime to 1 hour ago.
    const oneHourAgo = new Date(Date.now() - 60 * 60 * 1000)
    await utimes(path, oneHourAgo, oneHourAgo)
    // maxAgeMs = 1 minute → file is 1 hour old → stale → reclaim.
    const h = await acquire(path, { maxAgeMs: 60 * 1000 })
    const text = await readFile(path, 'utf8')
    expect(Number.parseInt(text, 10)).toBe(process.pid)
    await h.release()
  })

  it('does NOT reclaim when mtime is fresh (within maxAgeMs)', async () => {
    const path = await freshLockPath()
    // Plant a lock held by our own (live) PID with current mtime.
    await writeFile(path, `${process.pid}\n${new Date().toISOString()}\n`)
    // Lock is fresh + holder is alive → must throw.
    await expect(
      acquire(path, { maxAgeMs: 60 * 60 * 1000 }),
    ).rejects.toBeInstanceOf(SessionLockedError)
  })

  it('max-age check beats PID-liveness (zombie PID parity)', async () => {
    const path = await freshLockPath()
    // PID is alive (our own) AND time is old → max-age fires first.
    await writeFile(path, `${process.pid}\n2026-01-01T00:00:00.000Z\n`)
    const oldMtime = new Date(Date.now() - 24 * 60 * 60 * 1000) // 1 day ago
    await utimes(path, oldMtime, oldMtime)
    const h = await acquire(path, { maxAgeMs: 60 * 1000 })
    await h.release()
  })

  it('respects injected `now()` clock (deterministic tests)', async () => {
    const path = await freshLockPath()
    // Plant lock with PID alive (ours), mtime = "now-realtime".
    await writeFile(path, `${process.pid}\n${new Date().toISOString()}\n`)
    // With now() returning a value 10 minutes ahead of mtime + maxAge=5min,
    // the lock should be considered stale.
    const futureNow = Date.now() + 10 * 60 * 1000
    const h = await acquire(path, {
      maxAgeMs: 5 * 60 * 1000,
      now: () => futureNow,
    })
    await h.release()
  })
})
