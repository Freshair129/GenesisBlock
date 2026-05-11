import { describe, expect, it } from 'vitest'

import { evidenceForDecisions } from '../../../src/validator/rules/evidence-for-decisions.js'
import type { ParsedAtom, ValidationContext } from '../../../src/validator/types.js'

const ctx: ValidationContext = { atomicIndex: new Map() }

function atom(body: string, type = 'adr'): ParsedAtom {
  return { fm: { type }, body, source: '', filepath: 'x.md' }
}

const FULL_ADR = `# ADR — foo

## Context

something.

## Decision

we did X.

## Consequences

Y happens.
`

describe('evidenceForDecisions', () => {
  it('passes a complete ADR with all three sections', () => {
    expect(evidenceForDecisions(atom(FULL_ADR), ctx)).toEqual([])
  })

  it('rejects an ADR missing Decision', () => {
    const body = '# ADR\n\n## Context\n\nx\n\n## Consequences\n\ny\n'
    const errs = evidenceForDecisions(atom(body), ctx)
    expect(errs).toHaveLength(1)
    expect(errs[0]!.offending).toBe('Decision')
  })

  it('rejects an ADR missing all three', () => {
    const errs = evidenceForDecisions(atom('# ADR\n\nbody only\n'), ctx)
    expect(errs).toHaveLength(3)
  })

  it('case-insensitive matches', () => {
    const body = '## CONTEXT\n## decision\n## Consequences\n'
    expect(evidenceForDecisions(atom(body), ctx)).toEqual([])
  })

  it('no-op for non-ADR types', () => {
    expect(evidenceForDecisions(atom('# CONCEPT\nbody\n', 'concept'), ctx)).toEqual([])
  })
})
