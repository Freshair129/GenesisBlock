import { describe, expect, it } from 'vitest'

import { futureDate } from '../../../src/validator/rules/future-date.js'
import type { ParsedAtom, ValidationContext } from '../../../src/validator/types.js'

const now = new Date('2026-05-01T00:00:00Z')
const ctx: ValidationContext = { atomicIndex: new Map(), now }

function atom(fm: Record<string, unknown>): ParsedAtom {
  return { fm, body: '', source: '', filepath: 'x.md' }
}

describe('futureDate', () => {
  it('passes when created_at is in the past', () => {
    expect(futureDate(atom({ created_at: '2026-04-01T00:00:00Z' }), ctx)).toEqual([])
  })

  it('rejects when created_at is in the future', () => {
    const errs = futureDate(atom({ created_at: '2099-01-01T00:00:00Z' }), ctx)
    expect(errs).toHaveLength(1)
    expect(errs[0]!.rule).toBe('future-date')
  })

  it('rejects an unparseable date string', () => {
    const errs = futureDate(atom({ created_at: 'not-a-date' }), ctx)
    expect(errs).toHaveLength(1)
  })

  it('skips when created_at is missing', () => {
    expect(futureDate(atom({}), ctx)).toEqual([])
  })
})
