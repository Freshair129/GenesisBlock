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

  describe('aliases validation', () => {
    const aliasCfg: RequiredFieldsConfig = {
      default: ['aliases'],
      byType: new Map([
        ['concept', ['aliases']],
      ]),
    }

    it('passes when aliases is present, is a non-empty array of strings, and primary matches type prefix', () => {
      expect(
        requiredFields(
          atom({ id: 'CONCEPT--FOO', aliases: ['CONCEPT', 'custom'] }),
          ctx(aliasCfg),
        ),
      ).toEqual([])
    })

    it('flags missing aliases', () => {
      const errs = requiredFields(atom({ id: 'CONCEPT--FOO' }), ctx(aliasCfg))
      expect(errs).toHaveLength(1)
      expect(errs[0]!.offending).toBe('aliases')
    })

    it('flags aliases that is not an array', () => {
      const errs = requiredFields(atom({ id: 'CONCEPT--FOO', aliases: 'not-an-array' }), ctx(aliasCfg))
      expect(errs).toHaveLength(1)
      expect(errs[0]!.message).toContain('must be a non-empty string array')
    })

    it('flags empty aliases array', () => {
      const errs = requiredFields(atom({ id: 'CONCEPT--FOO', aliases: [] }), ctx(aliasCfg))
      expect(errs).toHaveLength(1)
      expect(errs[0]!.message).toContain("missing required field 'aliases'")
    })

    it('flags aliases array where first item is not string', () => {
      const errs = requiredFields(atom({ id: 'CONCEPT--FOO', aliases: [123] }), ctx(aliasCfg))
      expect(errs).toHaveLength(1)
      expect(errs[0]!.message).toContain('must be a non-empty string array')
    })

    it('flags aliases array where primary alias does not match ID type prefix', () => {
      const errs = requiredFields(
        atom({ id: 'CONCEPT--FOO', aliases: ['FEAT'] }),
        ctx(aliasCfg),
      )
      expect(errs).toHaveLength(1)
      expect(errs[0]!.message).toContain("alias at index 0 must match the expected 'CONCEPT'")
    })

    it('checks proposed_id prefix if id is not present (for candidates)', () => {
      expect(
        requiredFields(
          atom({ proposed_id: 'CONCEPT--FOO', aliases: ['CONCEPT'] }),
          ctx(aliasCfg),
        ),
      ).toEqual([])
    })
  })

  describe('cluster and role validation', () => {
    const metaCfg: RequiredFieldsConfig = {
      default: ['cluster', 'role'],
      byType: new Map([
        ['concept', ['cluster', 'role']],
      ]),
    }

    it('passes when cluster and role match the registry', () => {
      expect(
        requiredFields(
          atom({ id: 'CONCEPT--FOO', cluster: 'implementation_flow', role: 'Strategic intent / PRD' }),
          ctx(metaCfg),
        ),
      ).toEqual([])
    })

    it('flags missing cluster or role', () => {
      const errs = requiredFields(atom({ id: 'CONCEPT--FOO', role: 'Strategic intent / PRD' }), ctx(metaCfg))
      expect(errs).toHaveLength(1)
      expect(errs[0]!.offending).toBe('cluster')
    })

    it('flags incorrect cluster value', () => {
      const errs = requiredFields(
        atom({ id: 'CONCEPT--FOO', cluster: 'incorrect_cluster', role: 'Strategic intent / PRD' }),
        ctx(metaCfg),
      )
      expect(errs).toHaveLength(1)
      expect(errs[0]!.message).toContain("field 'cluster' must match registry value 'implementation_flow'")
    })

    it('flags incorrect role value', () => {
      const errs = requiredFields(
        atom({ id: 'CONCEPT--FOO', cluster: 'implementation_flow', role: 'Incorrect Role' }),
        ctx(metaCfg),
      )
      expect(errs).toHaveLength(1)
      expect(errs[0]!.message).toContain("field 'role' must match registry value 'Strategic intent / PRD'")
    })
  })
})

