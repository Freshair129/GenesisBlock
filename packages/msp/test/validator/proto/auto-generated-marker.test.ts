import { describe, expect, it, vi, beforeEach } from 'vitest'
import { predicate } from '../../../src/cognitive/compose.js'
import type { PredicateContext } from '../../../src/validator/proto/types.js'
import * as fs from 'node:fs/promises'
import { join } from 'node:path'

vi.mock('node:fs/promises', async () => {
  const actual = await vi.importActual<typeof import('node:fs/promises')>('node:fs/promises')
  return {
    ...actual,
    readdir: vi.fn(),
    readFile: vi.fn(),
    stat: vi.fn(),
  }
})

describe('PROTO--AUTO-GENERATED-MARKER predicate', () => {
  const repoRoot = '/repo'
  const ctx: PredicateContext = {
    repoRoot,
    atomicIndex: [],
  }

  beforeEach(() => {
    vi.resetAllMocks()
    vi.mocked(fs.readdir).mockImplementation(async (path: any) => {
      if (path === join(repoRoot, 'packages')) return ['msp'] as any
      if (path === join(repoRoot, 'packages/msp/src')) return [] as any
      return [] as any
    })
  })

  it('passes when no files have the marker', async () => {
    const result = await predicate(ctx)
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('passes when files have a valid marker', async () => {
    vi.mocked(fs.readdir).mockImplementation(async (path: any) => {
      if (path === join(repoRoot, 'packages')) return ['msp'] as any
      if (path === join(repoRoot, 'packages/msp/src')) return [{ name: 'gen.ts', isDirectory: () => false, isFile: () => true }] as any
      return [] as any
    })
    vi.mocked(fs.readFile).mockResolvedValue(
      '// AUTO-GENERATED — DO NOT EDIT\n// Source: manifest.yaml\n// Regenerate: npm run msp:run-task FEAT--X\n\nexport const x = 1'
    )

    const result = await predicate(ctx)
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('warns when a file has a malformed marker (missing line 2)', async () => {
    vi.mocked(fs.readdir).mockImplementation(async (path: any) => {
      if (path === join(repoRoot, 'packages')) return ['msp'] as any
      if (path === join(repoRoot, 'packages/msp/src')) return [{ name: 'bad.ts', isDirectory: () => false, isFile: () => true }] as any
      return [] as any
    })
    vi.mocked(fs.readFile).mockResolvedValue(
      '// AUTO-GENERATED — DO NOT EDIT\n// Oops: manifest.yaml\n// Regenerate: npm run msp:run-task FEAT--X\n'
    )

    const result = await predicate(ctx)
    expect(result.ok).toBe(false)
    expect(result.violations[0].message).toMatch(/missing 'Source:' on line 2/)
  })
})
