/**
 * Tests for `src/master/registry.ts`. Uses tmpdir fixtures to avoid
 * touching the live `gks/master/registry.jsonl` (which is gitignored
 * anyway).
 */
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import {
  appendRegistry,
  findActiveMaster,
  readRegistry,
  type MasterEntry,
} from '../../src/master/registry.js'

const tmpRoots: string[] = []

afterEach(async () => {
  for (const dir of tmpRoots.splice(0)) {
    await rm(dir, { recursive: true, force: true })
  }
})

async function freshRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-registry-'))
  tmpRoots.push(root)
  return root
}

describe('readRegistry', () => {
  it('returns [] when the registry file does not exist', async () => {
    const root = await freshRoot()
    const entries = await readRegistry(root)
    expect(entries).toEqual([])
  })

  it('returns [] when the file exists but is empty', async () => {
    const root = await freshRoot()
    await mkdir(join(root, 'gks', 'master'), { recursive: true })
    await writeFile(join(root, 'gks', 'master', 'registry.jsonl'), '', 'utf8')
    const entries = await readRegistry(root)
    expect(entries).toEqual([])
  })

  it('parses valid JSONL lines into MasterEntry objects', async () => {
    const root = await freshRoot()
    await mkdir(join(root, 'gks', 'master'), { recursive: true })
    const lines = [
      JSON.stringify({
        block_id: 'IDENTITY-ENGINE',
        promoted_at: '2026-05-14T04:00:00.000Z',
        status: 'active',
      }),
      JSON.stringify({
        block_id: 'CONTRADICTION-DETECTION',
        promoted_at: '2026-05-15T04:00:00.000Z',
        promotion_pr: '#125',
        status: 'active',
      }),
      '',
    ].join('\n')
    await writeFile(
      join(root, 'gks', 'master', 'registry.jsonl'),
      lines,
      'utf8',
    )
    const entries = await readRegistry(root)
    expect(entries).toHaveLength(2)
    expect(entries[0]!.block_id).toBe('IDENTITY-ENGINE')
    expect(entries[1]!.promotion_pr).toBe('#125')
  })

  it('silently skips malformed lines mixed with valid entries', async () => {
    const root = await freshRoot()
    await mkdir(join(root, 'gks', 'master'), { recursive: true })
    const lines = [
      'not-json-at-all',
      JSON.stringify({ block_id: 'OK', promoted_at: 'X', status: 'active' }),
      '{"block_id": "missing-status"}',
      JSON.stringify({
        block_id: 'OK2',
        promoted_at: '2026-01-01T00:00:00.000Z',
        status: 'unknown', // bad enum value
      }),
      JSON.stringify({
        block_id: 'OK3',
        promoted_at: '2026-01-01T00:00:00.000Z',
        status: 'archived',
      }),
    ].join('\n')
    await writeFile(
      join(root, 'gks', 'master', 'registry.jsonl'),
      lines,
      'utf8',
    )
    const entries = await readRegistry(root)
    expect(entries.map((e) => e.block_id)).toEqual(['OK', 'OK3'])
  })
})

describe('appendRegistry', () => {
  it('creates the gks/master/ directory if missing', async () => {
    const root = await freshRoot()
    expect(existsSync(join(root, 'gks', 'master'))).toBe(false)
    await appendRegistry(root, {
      block_id: 'FOO',
      promoted_at: '2026-05-14T04:00:00.000Z',
      status: 'active',
    })
    expect(existsSync(join(root, 'gks', 'master', 'registry.jsonl'))).toBe(true)
  })

  it('appends one valid JSONL line per call', async () => {
    const root = await freshRoot()
    await appendRegistry(root, {
      block_id: 'FIRST',
      promoted_at: '2026-05-14T04:00:00.000Z',
      status: 'active',
    })
    await appendRegistry(root, {
      block_id: 'SECOND',
      promoted_at: '2026-05-15T04:00:00.000Z',
      promotion_pr: '#200',
      status: 'active',
    })
    const raw = await readFile(
      join(root, 'gks', 'master', 'registry.jsonl'),
      'utf8',
    )
    const lines = raw.split('\n').filter((l) => l.length > 0)
    expect(lines).toHaveLength(2)
    const parsed0 = JSON.parse(lines[0]!) as MasterEntry
    const parsed1 = JSON.parse(lines[1]!) as MasterEntry
    expect(parsed0.block_id).toBe('FIRST')
    expect(parsed1.promotion_pr).toBe('#200')
  })

  it('round-trips through readRegistry', async () => {
    const root = await freshRoot()
    const a: MasterEntry = {
      block_id: 'A',
      promoted_at: '2026-05-14T04:00:00.000Z',
      status: 'active',
    }
    const b: MasterEntry = {
      block_id: 'B',
      promoted_at: '2026-05-15T04:00:00.000Z',
      status: 'active',
    }
    await appendRegistry(root, a)
    await appendRegistry(root, b)
    const entries = await readRegistry(root)
    expect(entries).toEqual([a, b])
  })
})

describe('findActiveMaster', () => {
  it('returns null when the registry is missing', async () => {
    const root = await freshRoot()
    const hit = await findActiveMaster(root, 'FOO')
    expect(hit).toBeNull()
  })

  it('returns null when no entry matches the blockId', async () => {
    const root = await freshRoot()
    await appendRegistry(root, {
      block_id: 'OTHER',
      promoted_at: '2026-05-14T04:00:00.000Z',
      status: 'active',
    })
    const hit = await findActiveMaster(root, 'FOO')
    expect(hit).toBeNull()
  })

  it('returns the active entry for a matching blockId', async () => {
    const root = await freshRoot()
    await appendRegistry(root, {
      block_id: 'IDENTITY-ENGINE',
      promoted_at: '2026-05-14T04:00:00.000Z',
      status: 'active',
    })
    const hit = await findActiveMaster(root, 'IDENTITY-ENGINE')
    expect(hit).not.toBeNull()
    expect(hit!.block_id).toBe('IDENTITY-ENGINE')
    expect(hit!.status).toBe('active')
  })

  it('returns null when every entry for the blockId is archived', async () => {
    const root = await freshRoot()
    await appendRegistry(root, {
      block_id: 'OLD',
      promoted_at: '2026-04-01T04:00:00.000Z',
      status: 'archived',
    })
    const hit = await findActiveMaster(root, 'OLD')
    expect(hit).toBeNull()
  })

  it('returns the latest active entry when multiple exist (last-write-wins)', async () => {
    const root = await freshRoot()
    await appendRegistry(root, {
      block_id: 'X',
      promoted_at: '2026-05-14T04:00:00.000Z',
      promotion_pr: '#100',
      status: 'active',
    })
    await appendRegistry(root, {
      block_id: 'X',
      promoted_at: '2026-05-20T04:00:00.000Z',
      promotion_pr: '#200',
      status: 'active',
    })
    const hit = await findActiveMaster(root, 'X')
    expect(hit).not.toBeNull()
    expect(hit!.promotion_pr).toBe('#200')
  })
})
