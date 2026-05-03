import { describe, expect, it } from 'vitest'

import { requiredFields } from '../../../src/validator/rules/required-fields.js'
import type {
  ParsedAtom,
  RequiredFieldsConfig,
  ValidationContext,
} from '../../../src/validator/types.js'

function ctx(cfg?: RequiredFieldsConfig): ValidationContext {
  return { atomicIndex: new Map(), requiredFields: cfg }
}

function atom(fm: Record<string, unknown>): ParsedAtom {
  return { fm, body: '', source: '', filepath: 'x.md' }
}

const cfg: RequiredFieldsConfig = {
  default: ['id', 'phase', 'type'],
  byType: new Map([
    ['adr', ['id', 'phase', 'type', 'title', 'tags']],
    ['blueprint', ['id', 'phase', 'type', 'title', 'linked_symbols']],
  ]),
}

describe('requiredFields', () => {
  it('no-op when context has no requiredFields config', () => {
    expect(requiredFields(atom({ id: 'X' }), ctx())).toEqual([])
  })

  it('passes when default fields are present', () => {
    expect(
      requiredFields(atom({ id: 'X', phase: 1, type: 'concept' }), ctx(cfg)),
    ).toEqual([])
  })

  it('flags missing default field', () => {
    const errs = requiredFields(atom({ id: 'X', phase: 1 }), ctx(cfg))
    expect(errs).toHaveLength(1)
    expect(errs[0]!.offending).toBe('type')
  })

  it('uses by-type list when present', () => {
    // ADR requires title + tags; provide neither
    const errs = requiredFields(
      atom({ id: 'X', phase: 2, type: 'adr' }),
      ctx(cfg),
    )
    expect(errs.map((e) => e.offending).sort()).toEqual(['tags', 'title'])
  })

  it('treats empty string as missing', () => {
    const errs = requiredFields(atom({ id: '', phase: 1, type: 'concept' }), ctx(cfg))
    expect(errs.map((e) => e.offending)).toEqual(['id'])
  })

  it('treats empty array as missing', () => {
    const errs = requiredFields(
      atom({ id: 'X', phase: 3, type: 'blueprint', title: 't', linked_symbols: [] }),
      ctx(cfg),
    )
    expect(errs.map((e) => e.offending)).toEqual(['linked_symbols'])
  })

  it('all errors have rule=required-fields', () => {
    const errs = requiredFields(atom({}), ctx(cfg))
    for (const e of errs) expect(e.rule).toBe('required-fields')
  })
})
