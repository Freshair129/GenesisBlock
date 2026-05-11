import { describe, expect, it } from 'vitest'

import { idFilenameMatch } from '../../../src/validator/rules/id-filename-match.js'
import type { ParsedAtom, ValidationContext } from '../../../src/validator/types.js'

const ctx: ValidationContext = { atomicIndex: new Map() }

function atom(fm: Record<string, unknown>, filepath: string): ParsedAtom {
  return { fm, body: '', source: '', filepath }
}

describe('idFilenameMatch', () => {
  it('passes when id matches filename stem', () => {
    expect(
      idFilenameMatch(
        atom({ id: 'CONCEPT--MSP-VALIDATOR' }, '/x/gks/concept/CONCEPT--MSP-VALIDATOR.md'),
        ctx,
      ),
    ).toEqual([])
  })

  it('rejects when id does not match filename', () => {
    const errs = idFilenameMatch(
      atom({ id: 'CONCEPT--BAR' }, '/x/CONCEPT--FOO.md'),
      ctx,
    )
    expect(errs).toHaveLength(1)
    expect(errs[0]!.rule).toBe('id-filename-match')
  })

  it('strips .rev-XXXX suffix from legacy filenames (back-compat for archived inbound revisions)', () => {
    expect(
      idFilenameMatch(
        atom(
          { proposed_id: 'CONCEPT--FOO' },
          '/x/legacy/CONCEPT--FOO.rev-abc123-deadbeef.md',
        ),
        ctx,
      ),
    ).toEqual([])
  })
})
