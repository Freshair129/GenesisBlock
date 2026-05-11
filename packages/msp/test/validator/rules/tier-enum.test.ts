import { describe, expect, it } from 'vitest'

import { tierEnum } from '../../../src/validator/rules/tier-enum.js'
import type { ParsedAtom, ValidationContext } from '../../../src/validator/types.js'

const ctx: ValidationContext = { atomicIndex: new Map() }

function atom(fm: Record<string, unknown>): ParsedAtom {
  return { fm, body: '', source: '', filepath: 'x.md' }
}

describe('tierEnum', () => {
  it('passes when tier is one of the allowed values', () => {
    for (const t of ['safety', 'master', 'genesis', 'process']) {
      expect(tierEnum(atom({ tier: t, source_type: 'axiomatic' }), ctx)).toEqual([])
    }
  })

  it('warns when tier is missing', () => {
    const errs = tierEnum(atom({ id: 'CONCEPT--FOO' }), ctx)
    expect(errs).toHaveLength(1)
    expect(errs[0]!.severity).toBe('warning')
    expect(errs[0]!.message).toMatch(/missing 'tier'/)
  })

  it('warns when tier is not in the allowed set', () => {
    const errs = tierEnum(atom({ tier: 'bogus' }), ctx)
    expect(errs).toHaveLength(1)
    expect(errs[0]!.severity).toBe('warning')
    expect(errs[0]!.message).toMatch(/not in allowed set/)
  })

  it('warns when source_type is set but invalid', () => {
    const errs = tierEnum(atom({ tier: 'genesis', source_type: 'guessed' }), ctx)
    expect(errs).toHaveLength(1)
    expect(errs[0]!.message).toMatch(/source_type/)
  })

  it('passes when source_type is omitted (still optional)', () => {
    expect(tierEnum(atom({ tier: 'genesis' }), ctx)).toEqual([])
  })

  it('accepts source_type:learned', () => {
    expect(tierEnum(atom({ tier: 'genesis', source_type: 'learned' }), ctx)).toEqual([])
  })
})
