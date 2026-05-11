import { describe, expect, it } from 'vitest'

import { summaryMin } from '../../../src/validator/rules/summary-min.js'
import type { ParsedAtom, ValidationContext } from '../../../src/validator/types.js'

const ctx: ValidationContext = { atomicIndex: new Map() }

function atom(fm: Record<string, unknown>): ParsedAtom {
  return { fm, body: '', source: '', filepath: 'x.md' }
}

describe('summaryMin', () => {
  it('passes a normal summary', () => {
    expect(summaryMin(atom({ summary: 'A reasonable description.' }), ctx)).toEqual([])
  })

  it('rejects too-short summary', () => {
    const errs = summaryMin(atom({ summary: 'short' }), ctx)
    expect(errs).toHaveLength(1)
    expect(errs[0]!.rule).toBe('summary-min')
  })

  it('rejects too-long summary', () => {
    const errs = summaryMin(atom({ summary: 'x'.repeat(400) }), ctx)
    expect(errs).toHaveLength(1)
  })

  it('rejects placeholder text', () => {
    const errs = summaryMin(atom({ summary: 'TBD will fill later' }), ctx)
    expect(errs).toHaveLength(1)
  })

  it('skips when summary is missing', () => {
    expect(summaryMin(atom({}), ctx)).toEqual([])
  })
})
