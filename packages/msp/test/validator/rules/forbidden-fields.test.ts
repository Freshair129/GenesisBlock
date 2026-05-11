import { describe, expect, it } from 'vitest'

import { forbiddenFields } from '../../../src/validator/rules/forbidden-fields.js'
import type { ParsedAtom, ValidationContext } from '../../../src/validator/types.js'

const ctx: ValidationContext = { atomicIndex: new Map() }

function atom(fm: Record<string, unknown>): ParsedAtom {
  return { fm, body: '', source: '', filepath: 'x.md' }
}

describe('forbiddenFields', () => {
  it('returns empty when no forbidden field is present', () => {
    expect(forbiddenFields(atom({ id: 'CONCEPT--FOO', title: 'ok' }), ctx)).toEqual([])
  })

  it('rejects commit_hash', () => {
    const errors = forbiddenFields(atom({ id: 'X', commit_hash: 'abc' }), ctx)
    expect(errors).toHaveLength(1)
    expect(errors[0]!.rule).toBe('forbidden-fields')
    expect(errors[0]!.offending).toBe('commit_hash')
  })

  it('rejects multiple forbidden fields', () => {
    const errors = forbiddenFields(
      atom({ commit_hash: 'a', validated_by: 'b', promotion_level: 1 }),
      ctx,
    )
    expect(errors).toHaveLength(3)
  })

  it('honours ctx.forbiddenFields override', () => {
    const errors = forbiddenFields(atom({ commit_hash: 'a' }), {
      atomicIndex: new Map(),
      forbiddenFields: new Set(),
    })
    expect(errors).toHaveLength(0)
  })
})
