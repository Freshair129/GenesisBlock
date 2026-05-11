import { describe, expect, it } from 'vitest'

import { adrMonotonic } from '../../../src/validator/rules/adr-monotonic.js'
import type {
  AtomicIndexEntry,
  ParsedAtom,
  ValidationContext,
} from '../../../src/validator/types.js'

function entry(id: string): AtomicIndexEntry {
  return { id, type: 'adr', status: 'stable', path: `adr/${id}.md` }
}

function ctxWithAdrs(ids: string[]): ValidationContext {
  return { atomicIndex: new Map(ids.map((id) => [id, entry(id)])) }
}

function atom(fm: Record<string, unknown>): ParsedAtom {
  return { fm, body: '', source: '', filepath: 'x.md' }
}

describe('adrMonotonic', () => {
  it('passes when new ADR is max + 1', () => {
    const ctx = ctxWithAdrs(['ADR-001', 'ADR-002', 'ADR-003'])
    expect(adrMonotonic(atom({ id: 'ADR-004' }), ctx)).toEqual([])
  })

  it('rejects when new ADR is greater than max + 1', () => {
    const ctx = ctxWithAdrs(['ADR-001', 'ADR-002', 'ADR-003'])
    const errs = adrMonotonic(atom({ id: 'ADR-007' }), ctx)
    expect(errs).toHaveLength(1)
    expect(errs[0]!.rule).toBe('adr-monotonic')
    expect(errs[0]!.message).toContain('ADR-004')
  })

  it('rejects when new ADR collides with existing', () => {
    const ctx = ctxWithAdrs(['ADR-001', 'ADR-002'])
    const errs = adrMonotonic(atom({ id: 'ADR-001' }), ctx)
    expect(errs).toHaveLength(1)
  })

  it('passes when new ADR is ADR-001 with empty index', () => {
    const ctx = ctxWithAdrs([])
    expect(adrMonotonic(atom({ id: 'ADR-001' }), ctx)).toEqual([])
  })

  it('skips slug-form ADR--FOO (no monotonic constraint)', () => {
    const ctx = ctxWithAdrs(['ADR-001'])
    expect(adrMonotonic(atom({ id: 'ADR--MSP-VALIDATOR' }), ctx)).toEqual([])
  })
})
