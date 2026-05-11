import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import predicate from '../../../src/validator/proto/valid-until.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-proto-valid-until-'))
}

interface AtomSpec {
  id: string
  type: string
  status: string
  /** repo-relative path UNDER `gks/`. */
  pathUnderGks: string
  validUntil?: string
  body?: string
}

async function writeAtomFile(
  root: string,
  spec: AtomSpec,
): Promise<AtomicIndexEntry> {
  const fullPath = join(root, 'gks', spec.pathUnderGks)
  await mkdir(join(fullPath, '..'), { recursive: true })
  const fmLines = [
    '---',
    `id: ${spec.id}`,
    `type: ${spec.type}`,
    `status: ${spec.status}`,
    `phase: 1`,
    `vault_id: default`,
  ]
  if (spec.validUntil !== undefined) {
    // Wrap in quotes so YAML treats it as a string, not a Date.
    fmLines.push(`valid_until: "${spec.validUntil}"`)
  }
  fmLines.push('---', '', spec.body ?? '# body', '')
  await writeFile(fullPath, fmLines.join('\n'))
  return {
    id: spec.id,
    type: spec.type,
    status: spec.status,
    path: spec.pathUnderGks,
  } as AtomicIndexEntry
}

describe('PROTO--VALID-UNTIL predicate', () => {
  const originalNow = process.env['MSP_NOW']

  beforeEach(() => {
    delete process.env['MSP_NOW']
  })

  afterEach(() => {
    if (originalNow === undefined) delete process.env['MSP_NOW']
    else process.env['MSP_NOW'] = originalNow
  })

  it('passes vacuously when no atoms declare valid_until', async () => {
    const root = await freshRoot()
    const a = await writeAtomFile(root, {
      id: 'CONCEPT--A',
      type: 'concept',
      status: 'stable',
      pathUnderGks: 'concept/CONCEPT--A.md',
    })
    const b = await writeAtomFile(root, {
      id: 'CONCEPT--B',
      type: 'concept',
      status: 'stable',
      pathUnderGks: 'concept/CONCEPT--B.md',
    })

    const result = await predicate({
      atomicIndex: [a, b],
      repoRoot: root,
    })

    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('emits a warning when an atom is expired', async () => {
    process.env['MSP_NOW'] = '2099-01-01T00:00:00.000Z'
    const root = await freshRoot()
    const expired = await writeAtomFile(root, {
      id: 'CONCEPT--EXPIRED',
      type: 'concept',
      status: 'stable',
      pathUnderGks: 'concept/CONCEPT--EXPIRED.md',
      validUntil: '2026-01-01',
    })

    const result = await predicate({
      atomicIndex: [expired],
      repoRoot: root,
    })

    expect(result.violations).toHaveLength(1)
    expect(result.violations[0]!.severity).toBe('warning')
    expect(result.violations[0]!.atomId).toBe('CONCEPT--EXPIRED')
    expect(result.violations[0]!.message).toMatch(/expired \d+ days ago/)
    expect(result.ok).toBe(true) // warnings don't flip ok
  })

  it('emits an info violation when an atom is near expiry (< 30 days)', async () => {
    process.env['MSP_NOW'] = '2026-05-01T00:00:00.000Z'
    const root = await freshRoot()
    const nearExpiry = await writeAtomFile(root, {
      id: 'CONCEPT--SOON',
      type: 'concept',
      status: 'stable',
      pathUnderGks: 'concept/CONCEPT--SOON.md',
      // ~14 days after MSP_NOW
      validUntil: '2026-05-15',
    })

    const result = await predicate({
      atomicIndex: [nearExpiry],
      repoRoot: root,
    })

    expect(result.violations).toHaveLength(1)
    expect(result.violations[0]!.severity).toBe('info')
    expect(result.violations[0]!.atomId).toBe('CONCEPT--SOON')
    expect(result.violations[0]!.message).toMatch(/expires in 1[34] days/)
    expect(result.ok).toBe(true)
  })

  it('does not emit when expiry is far in the future (> 30 days)', async () => {
    process.env['MSP_NOW'] = '2026-01-01T00:00:00.000Z'
    const root = await freshRoot()
    const farFuture = await writeAtomFile(root, {
      id: 'CONCEPT--FUTURE',
      type: 'concept',
      status: 'stable',
      pathUnderGks: 'concept/CONCEPT--FUTURE.md',
      validUntil: '2030-12-31',
    })

    const result = await predicate({
      atomicIndex: [farFuture],
      repoRoot: root,
    })

    expect(result.violations).toEqual([])
    expect(result.ok).toBe(true)
  })

  it('skips superseded atoms even when expired', async () => {
    process.env['MSP_NOW'] = '2099-01-01T00:00:00.000Z'
    const root = await freshRoot()
    const supersededExpired = await writeAtomFile(root, {
      id: 'CONCEPT--OLD',
      type: 'concept',
      status: 'superseded',
      pathUnderGks: 'concept/CONCEPT--OLD.md',
      validUntil: '2026-01-01',
    })

    const result = await predicate({
      atomicIndex: [supersededExpired],
      repoRoot: root,
    })

    expect(result.violations).toEqual([])
    expect(result.ok).toBe(true)
  })

  it('respects the MSP_NOW env override (deterministic time travel)', async () => {
    const root = await freshRoot()
    const atom = await writeAtomFile(root, {
      id: 'CONCEPT--SAME',
      type: 'concept',
      status: 'stable',
      pathUnderGks: 'concept/CONCEPT--SAME.md',
      validUntil: '2026-06-01',
    })

    // First run: MSP_NOW long after expiry -> warning
    process.env['MSP_NOW'] = '2099-01-01T00:00:00.000Z'
    const past = await predicate({
      atomicIndex: [atom],
      repoRoot: root,
    })
    expect(past.violations).toHaveLength(1)
    expect(past.violations[0]!.severity).toBe('warning')

    // Second run: MSP_NOW long before expiry -> no violation
    process.env['MSP_NOW'] = '2020-01-01T00:00:00.000Z'
    const before = await predicate({
      atomicIndex: [atom],
      repoRoot: root,
    })
    expect(before.violations).toEqual([])
  })
})
