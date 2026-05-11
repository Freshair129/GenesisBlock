import { describe, expect, it } from 'vitest'

import { edgesFromAtom, sortEdges } from '../../../src/memory/backlinks/edges.js'

describe('edgesFromAtom', () => {
  it('returns empty when no crosslinks', () => {
    expect(edgesFromAtom({ id: 'CONCEPT--FOO' })).toEqual([])
  })

  it('returns empty when crosslinks is empty object', () => {
    expect(edgesFromAtom({ id: 'CONCEPT--FOO', crosslinks: {} })).toEqual([])
  })

  it('emits one edge per implements value', () => {
    const out = edgesFromAtom({
      id: 'FEAT--FOO',
      crosslinks: { implements: ['ADR--BAR', 'ADR--BAZ'] },
    })
    expect(out).toHaveLength(2)
    expect(out[0]).toEqual({ from: 'FEAT--FOO', to: 'ADR--BAR', type: 'implements' })
    expect(out[1]).toEqual({ from: 'FEAT--FOO', to: 'ADR--BAZ', type: 'implements' })
  })

  it('handles multiple predicates simultaneously', () => {
    const out = edgesFromAtom({
      id: 'FEAT--FOO',
      crosslinks: {
        implements: ['ADR--A'],
        references: ['CONCEPT--B'],
        used_by: ['BLUEPRINT--C'],
      },
    })
    expect(out).toHaveLength(3)
    expect(out.map((e) => e.type).sort()).toEqual(['implements', 'references', 'used_by'])
  })

  it('skips non-string targets', () => {
    const out = edgesFromAtom({
      id: 'FEAT--FOO',
      crosslinks: { implements: ['ADR--A', 42, null, 'ADR--B'] },
    })
    expect(out).toHaveLength(2)
  })

  it('returns empty when id is missing or wrong type', () => {
    expect(edgesFromAtom({ crosslinks: { implements: ['X'] } })).toEqual([])
    expect(edgesFromAtom({ id: 42, crosslinks: { implements: ['X'] } })).toEqual([])
  })

  it('skips unknown predicates', () => {
    const out = edgesFromAtom({
      id: 'FEAT--FOO',
      crosslinks: { madeUpKey: ['X'], implements: ['Y'] },
    })
    expect(out).toEqual([{ from: 'FEAT--FOO', to: 'Y', type: 'implements' }])
  })
})

describe('sortEdges', () => {
  it('sorts by (from, to, type)', () => {
    const out = sortEdges([
      { from: 'B', to: 'X', type: 'implements' },
      { from: 'A', to: 'Z', type: 'references' },
      { from: 'A', to: 'Y', type: 'implements' },
      { from: 'A', to: 'Y', type: 'references' },
    ])
    expect(out.map((e) => `${e.from}/${e.to}/${e.type}`)).toEqual([
      'A/Y/implements',
      'A/Y/references',
      'A/Z/references',
      'B/X/implements',
    ])
  })

  it('does not mutate input', () => {
    const input = [{ from: 'B', to: 'X', type: 't' }, { from: 'A', to: 'Y', type: 't' }]
    const before = input.map((e) => ({ ...e }))
    sortEdges(input)
    expect(input).toEqual(before)
  })
})
