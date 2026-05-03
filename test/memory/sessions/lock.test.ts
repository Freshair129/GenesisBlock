import { mkdtemp, readFile, writeFile } from 'node:fs/promises'
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
})
