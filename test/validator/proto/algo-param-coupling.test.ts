import { describe, expect, it } from 'vitest'

import predicate from '../../../src/validator/proto/algo-param-coupling.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'

function atom(
  id: string,
  type: string,
  crosslinks?: Record<string, string[]>,
): AtomicIndexEntry {
  return {
    id,
    type,
    status: 'stable',
    path: `${type}/${id}.md`,
    phase: 0,
    vault_id: 'default',
    crosslinks,
  } as AtomicIndexEntry
}

describe('PROTO--ALGO-PARAM-COUPLING predicate', () => {
  it('vacuous-passes when no ALGO/PARAM atoms exist', async () => {
    const result = await predicate({
      atomicIndex: [
        atom('FRAME--ARCH', 'frame'),
        atom('CONCEPT--FOO', 'concept'),
      ],
      repoRoot: '/tmp/anything',
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('passes when ALGO and PARAM mutually reciprocate', async () => {
    const result = await predicate({
      atomicIndex: [
        atom('ALGO--RANK', 'algo', { tunable_by: ['PARAM--TOP-K'] }),
        atom('PARAM--TOP-K', 'param', { tunes: ['ALGO--RANK'] }),
      ],
      repoRoot: '/tmp/anything',
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('errors when reciprocal tunes points to wrong ALGO', async () => {
    const result = await predicate({
      atomicIndex: [
        atom('ALGO--RANK', 'algo', { tunable_by: ['PARAM--TOP-K'] }),
        // PARAM--TOP-K names a different ALGO instead of ALGO--RANK
        atom('PARAM--TOP-K', 'param', { tunes: ['ALGO--OTHER'] }),
        atom('ALGO--OTHER', 'algo', { tunable_by: ['PARAM--TOP-K'] }),
      ],
      repoRoot: '/tmp/anything',
    })
    expect(result.ok).toBe(false)
    // ALGO--RANK side detects the missing reciprocal back-link.
    expect(
      result.violations.some(
        (v) =>
          v.atomId === 'ALGO--RANK' &&
          v.severity === 'error' &&
          /tunable_by PARAM--TOP-K/.test(v.message) &&
          /reciprocal tunes ALGO--RANK/.test(v.message),
      ),
    ).toBe(true)
  })

  it('errors when tunable_by references a non-PARAM id', async () => {
    const result = await predicate({
      atomicIndex: [
        atom('ALGO--RANK', 'algo', { tunable_by: ['CONCEPT--NOT-A-PARAM'] }),
      ],
      repoRoot: '/tmp/anything',
    })
    expect(result.ok).toBe(false)
    expect(result.violations).toHaveLength(1)
    expect(result.violations[0]!.severity).toBe('error')
    expect(result.violations[0]!.message).toMatch(
      /tunable_by references non-PARAM 'CONCEPT--NOT-A-PARAM'/,
    )
  })

  it('errors when tunes references a non-ALGO id', async () => {
    const result = await predicate({
      atomicIndex: [
        atom('PARAM--TOP-K', 'param', { tunes: ['FEAT--NOT-AN-ALGO'] }),
      ],
      repoRoot: '/tmp/anything',
    })
    expect(result.ok).toBe(false)
    expect(result.violations).toHaveLength(1)
    expect(result.violations[0]!.severity).toBe('error')
    expect(result.violations[0]!.message).toMatch(
      /tunes references non-ALGO 'FEAT--NOT-AN-ALGO'/,
    )
  })

  it('does not error when partner atom is missing from index (existence is GKS scope)', async () => {
    // ALGO references PARAM--MISSING which isn't in the index.
    // gks validate --links would catch this; this PROTO must NOT double-enforce.
    const result = await predicate({
      atomicIndex: [
        atom('ALGO--RANK', 'algo', { tunable_by: ['PARAM--MISSING'] }),
      ],
      repoRoot: '/tmp/anything',
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('checks reciprocals for every PARAM when an ALGO is tuned by multiple', async () => {
    const result = await predicate({
      atomicIndex: [
        atom('ALGO--RANK', 'algo', {
          tunable_by: ['PARAM--TOP-K', 'PARAM--TEMP', 'PARAM--SEED'],
        }),
        atom('PARAM--TOP-K', 'param', { tunes: ['ALGO--RANK'] }),
        // PARAM--TEMP is missing the back-link.
        atom('PARAM--TEMP', 'param', { tunes: [] }),
        atom('PARAM--SEED', 'param', { tunes: ['ALGO--RANK'] }),
      ],
      repoRoot: '/tmp/anything',
    })
    expect(result.ok).toBe(false)
    const algoViolations = result.violations.filter(
      (v) => v.atomId === 'ALGO--RANK',
    )
    // Only PARAM--TEMP is missing its reciprocal.
    expect(algoViolations).toHaveLength(1)
    expect(algoViolations[0]!.message).toMatch(/PARAM--TEMP/)
    expect(algoViolations[0]!.message).toMatch(/reciprocal tunes ALGO--RANK/)
  })
})
