import { describe, expect, it } from 'vitest'

import { idFormat, ID_PATTERN } from '../../../src/validator/rules/id-format.js'
import type { ParsedAtom, ValidationContext } from '../../../src/validator/types.js'

const ctx: ValidationContext = { atomicIndex: new Map() }

function atom(fm: Record<string, unknown>): ParsedAtom {
  return { fm, body: '', source: '', filepath: 'x.md' }
}

describe('ID_PATTERN', () => {
  it('matches canonical TYPE--SLUG with uppercase', () => {
    expect(ID_PATTERN.test('CONCEPT--MSP-VALIDATOR')).toBe(true)
    expect(ID_PATTERN.test('FEAT--FOO-BAR')).toBe(true)
    expect(ID_PATTERN.test('BLUEPRINT--MSP-VALIDATOR')).toBe(true)
  })

  it('matches numeric ADR form', () => {
    expect(ID_PATTERN.test('ADR-007')).toBe(true)
    expect(ID_PATTERN.test('ADR-099')).toBe(true)
  })

  it('rejects lowercase slug', () => {
    expect(ID_PATTERN.test('concept--foo')).toBe(false)
  })

  it('rejects single-hyphen TYPE-SLUG', () => {
    expect(ID_PATTERN.test('FEAT-FOO')).toBe(false)
  })

  it('rejects whitespace and other punctuation', () => {
    expect(ID_PATTERN.test('FEAT--foo bar')).toBe(false)
    expect(ID_PATTERN.test('FEAT--foo.bar')).toBe(false)
  })

  it('accepts HOTFIX--<hex> form (lowercase hex per gks hotfix open)', () => {
    expect(ID_PATTERN.test('HOTFIX--abc1234')).toBe(true)
    expect(ID_PATTERN.test('HOTFIX--0000000')).toBe(true)
    expect(ID_PATTERN.test('HOTFIX--deadbeef')).toBe(true)
  })

  it('rejects HOTFIX with non-hex chars', () => {
    expect(ID_PATTERN.test('HOTFIX--abcXYZ1')).toBe(false)
    expect(ID_PATTERN.test('HOTFIX--testfix')).toBe(false)
  })
})

describe('idFormat rule', () => {
  it('passes a valid id', () => {
    expect(idFormat(atom({ id: 'CONCEPT--MSP-VALIDATOR' }), ctx)).toEqual([])
  })

  it('rejects bad format with rule=id-format', () => {
    const errors = idFormat(atom({ id: 'foo-bar' }), ctx)
    expect(errors).toHaveLength(1)
    expect(errors[0]!.rule).toBe('id-format')
  })

  it('rejects when id is missing entirely', () => {
    const errors = idFormat(atom({ title: 'no id' }), ctx)
    expect(errors).toHaveLength(1)
    expect(errors[0]!.rule).toBe('id-format')
  })

  it('falls back to proposed_id for inbound atoms', () => {
    expect(idFormat(atom({ proposed_id: 'CONCEPT--FOO' }), ctx)).toEqual([])
  })
})
