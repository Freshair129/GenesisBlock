import { describe, expect, it, vi } from 'vitest'
import { predicate } from '../../../src/cognitive/scale-gate.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'
import * as fs from 'node:fs/promises'

vi.mock('node:fs/promises', async () => {
  const actual = await vi.importActual<typeof import('node:fs/promises')>('node:fs/promises')
  return {
    ...actual,
    readFile: vi.fn(),
  }
})

function blueprint(id: string, status: string = 'active'): AtomicIndexEntry {
  return {
    id,
    type: 'blueprint',
    status,
    path: `blueprint/${id}.md`,
  } as AtomicIndexEntry
}

describe('PROTO--SCALE-LEVEL-GATE predicate', () => {
  it('passes when all active blueprints have scale_level', async () => {
    const index = [blueprint('BLUEPRINT--VALID')]
    vi.mocked(fs.readFile).mockResolvedValue('---\nid: BLUEPRINT--VALID\nscale_level: L2\n---\n')

    const result = await predicate({
      atomicIndex: index,
      repoRoot: '/tmp',
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('warns when an active blueprint is missing scale_level', async () => {
    const index = [blueprint('BLUEPRINT--INVALID')]
    vi.mocked(fs.readFile).mockResolvedValue('---\nid: BLUEPRINT--INVALID\nstatus: active\n---\n')

    const result = await predicate({
      atomicIndex: index,
      repoRoot: '/tmp',
    })
    expect(result.ok).toBe(false)
    expect(result.violations).toHaveLength(1)
    expect(result.violations[0].message).toMatch(/missing a valid 'scale_level' field/)
  })

  it('skips non-blueprint atoms', async () => {
    const index = [{ id: 'FEAT--X', type: 'feat', status: 'active', path: 'feat/FEAT--X.md' } as AtomicIndexEntry]
    const result = await predicate({
      atomicIndex: index,
      repoRoot: '/tmp',
    })
    expect(result.ok).toBe(true)
  })
})
