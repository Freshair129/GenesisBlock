import { spawnSync } from 'node:child_process'
import { mkdtemp, readFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

import { openSession } from '../../../src/memory/sessions/writer.js'
import { SessionLockedError, SessionSchemaError } from '../../../src/memory/sessions/types.js'

const repoRoot = fileURLToPath(new URL('../../..', import.meta.url))

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-sessions-'))
}

const TURN = {
  sessionId: 'sess_001',
  episodicId: 'ep_001',
  turnId: 1,
  msgId: 'm_001',
  speakerId: 'user',
  content: 'hello',
}

describe('openSession + appendTurn', () => {
  it('creates the session file and appends one row', async () => {
    const root = await freshRoot()
    const s = await openSession({ root, episodicId: 'ep_001' })
    await s.appendTurn(TURN)
    await s.close()
    const path = join(root, '.brain/msp/projects/evaAI/sessions/ep_001.jsonl')
    const text = await readFile(path, 'utf8')
    const lines = text.trim().split('\n')
    expect(lines).toHaveLength(1)
    expect(JSON.parse(lines[0])).toMatchObject(TURN)
  })

  it('appends multiple rows in order', async () => {
    const root = await freshRoot()
    const s = await openSession({ root, episodicId: 'ep_002' })
    await s.appendTurn({ ...TURN, turnId: 1, content: 'one' })
    await s.appendTurn({ ...TURN, turnId: 2, content: 'two' })
    await s.appendTurn({ ...TURN, turnId: 3, content: 'three' })
    await s.close()
    const path = join(root, '.brain/msp/projects/evaAI/sessions/ep_002.jsonl')
    const text = await readFile(path, 'utf8')
    const turns = text.trim().split('\n').map((l) => JSON.parse(l))
    expect(turns.map((t) => t.turnId)).toEqual([1, 2, 3])
    expect(turns.map((t) => t.content)).toEqual(['one', 'two', 'three'])
  })

  it('appends across multiple openSession calls in sequence', async () => {
    const root = await freshRoot()
    const a = await openSession({ root, episodicId: 'ep_003' })
    await a.appendTurn(TURN)
    await a.close()

    const b = await openSession({ root, episodicId: 'ep_003' })
    await b.appendTurn({ ...TURN, turnId: 2 })
    await b.close()

    const path = join(root, '.brain/msp/projects/evaAI/sessions/ep_003.jsonl')
    const text = await readFile(path, 'utf8')
    expect(text.trim().split('\n')).toHaveLength(2)
  })

  it('escapes embedded newlines to keep one row per line', async () => {
    const root = await freshRoot()
    const s = await openSession({ root, episodicId: 'ep_nl' })
    await s.appendTurn({ ...TURN, content: 'multi\nline\nturn' })
    await s.close()
    const path = join(root, '.brain/msp/projects/evaAI/sessions/ep_nl.jsonl')
    const text = await readFile(path, 'utf8')
    expect(text.trim().split('\n')).toHaveLength(1)
    expect(JSON.parse(text.trim()).content).toBe('multi\nline\nturn')
  })

  it('rejects schema-invalid rows', async () => {
    const root = await freshRoot()
    const s = await openSession({ root, episodicId: 'ep_bad' })
    await expect(s.appendTurn({ sessionId: 's' } as never)).rejects.toBeInstanceOf(SessionSchemaError)
    await s.close()
  })

  it('throws after close()', async () => {
    const root = await freshRoot()
    const s = await openSession({ root, episodicId: 'ep_closed' })
    await s.close()
    await expect(s.appendTurn(TURN)).rejects.toThrow(/closed/)
  })

  it('two concurrent openSession calls on the same episodic → second throws SessionLockedError', async () => {
    const root = await freshRoot()
    const a = await openSession({ root, episodicId: 'ep_concur' })
    try {
      await expect(openSession({ root, episodicId: 'ep_concur' })).rejects.toBeInstanceOf(SessionLockedError)
    } finally {
      await a.close()
    }
  })

  it('cross-process: stale lock from a dead PID is auto-cleaned', async () => {
    // Use a child node process to acquire and abort, leaving the lock behind.
    const root = await freshRoot()
    const sessionPath = join(root, '.brain/msp/projects/evaAI/sessions/ep_stale.jsonl')
    const lockPath = `${sessionPath}.lock`

    // Spawn a child that acquires + kills itself before release.
    const script = `
      import { mkdir, writeFile } from 'node:fs/promises'
      import { dirname } from 'node:path'
      const lockPath = ${JSON.stringify(lockPath)}
      await mkdir(dirname(lockPath), { recursive: true })
      await writeFile(lockPath, String(process.pid), { flag: 'wx' })
      process.exit(0)
    `
    const r = spawnSync('node', ['--input-type=module', '-e', script], { encoding: 'utf8' })
    expect(r.status).toBe(0)

    // Now openSession should detect dead PID, clean, and acquire.
    const s = await openSession({ root, episodicId: 'ep_stale' })
    await s.appendTurn({ ...TURN, episodicId: 'ep_stale' })
    await s.close()
    const text = await readFile(sessionPath, 'utf8')
    expect(text.trim()).toMatch(/"turnId":1/)
  })
})
