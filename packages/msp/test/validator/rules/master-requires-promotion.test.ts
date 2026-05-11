import { describe, expect, it } from 'vitest'

import { masterRequiresPromotion } from '../../../src/validator/rules/master-requires-promotion.js'
import type { ParsedAtom, ValidationContext } from '../../../src/validator/types.js'

const ctx: ValidationContext = { atomicIndex: new Map() }

function atom(fm: Record<string, unknown>): ParsedAtom {
  return { fm, body: '', source: '', filepath: 'x.md' }
}

describe('masterRequiresPromotion', () => {
  it('passes when atom is not tier:master (rule does not apply)', () => {
    expect(masterRequiresPromotion(atom({ tier: 'genesis' }), ctx)).toEqual([])
    expect(masterRequiresPromotion(atom({ tier: 'safety' }), ctx)).toEqual([])
    expect(masterRequiresPromotion(atom({}), ctx)).toEqual([])
  })

  it('passes when tier:master atom has all promotion fields', () => {
    expect(
      masterRequiresPromotion(
        atom({
          tier: 'master',
          promoted_from: 'CONCEPT--FOO',
          promoted_at: '2026-05-09T07:00:00.000Z',
          promotion_adr: 'ADR--MASTER-PROMOTION-FOO',
        }),
        ctx,
      ),
    ).toEqual([])
  })

  it('errors for each missing promotion field on a tier:master atom', () => {
    const errs = masterRequiresPromotion(atom({ tier: 'master' }), ctx)
    expect(errs).toHaveLength(3)
    expect(errs.every((e) => e.severity === 'error')).toBe(true)
    expect(errs.map((e) => e.message).join('|')).toMatch(/promoted_from/)
    expect(errs.map((e) => e.message).join('|')).toMatch(/promoted_at/)
    expect(errs.map((e) => e.message).join('|')).toMatch(/promotion_adr/)
  })

  it('errors when tier:master carries learned_from (Master is instinct, not learned)', () => {
    const errs = masterRequiresPromotion(
      atom({
        tier: 'master',
        promoted_from: 'CONCEPT--X',
        promoted_at: '2026-05-09T07:00:00.000Z',
        promotion_adr: 'ADR--MASTER-PROMOTION-X',
        learned_from: { sessionId: 'S-1' },
      }),
      ctx,
    )
    expect(errs).toHaveLength(1)
    expect(errs[0]!.message).toMatch(/learned_from/)
  })
})
