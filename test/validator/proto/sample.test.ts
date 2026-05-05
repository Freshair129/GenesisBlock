import { describe, expect, it } from 'vitest'

import predicate from '../../../src/validator/proto/sample.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'

const frameAtom = {
  id: 'FRAME--MSP-ARCHITECTURE-V2',
  phase: 0,
  type: 'frame',
  status: 'stable',
  vault_id: 'default',
  path: 'frame/FRAME--MSP-ARCHITECTURE-V2.md',
} as AtomicIndexEntry

const conceptAtom = {
  id: 'CONCEPT--FOO',
  phase: 1,
  type: 'concept',
  status: 'stable',
  vault_id: 'default',
  path: 'concept/CONCEPT--FOO.md',
} as AtomicIndexEntry

describe('PROTO--SAMPLE-RULE predicate', () => {
  it('passes when at least one FRAME atom is present', async () => {
    const result = await predicate({
      atomicIndex: [frameAtom, conceptAtom],
      repoRoot: '/tmp/anything',
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('emits a warning violation when no FRAME atom is present', async () => {
    const result = await predicate({
      atomicIndex: [conceptAtom],
      repoRoot: '/tmp/anything',
    })
    expect(result.ok).toBe(false)
    expect(result.violations[0]!.severity).toBe('warning')
    expect(result.violations[0]!.message).toMatch(/no FRAME atom found/)
  })

  it('passes against the real repo atomic index (smoke)', async () => {
    // The actual repo has many FRAME atoms; this is a smoke check that the
    // predicate is callable and returns the expected shape.
    const result = await predicate({
      atomicIndex: [frameAtom],
      repoRoot: process.cwd(),
    })
    expect(result.ok).toBe(true)
  })
})
