import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

const REQUIRED_PROMOTION_FIELDS = ['promoted_from', 'promoted_at', 'promotion_adr'] as const

export function masterRequiresPromotion(
  atom: ParsedAtom,
  ctx: ValidationContext,
): ValidationError[] {
  const tier = atom.fm['tier']
  if (tier !== 'master') return []
  const errors: ValidationError[] = []
  for (const field of REQUIRED_PROMOTION_FIELDS) {
    const v = atom.fm[field]
    if (v === undefined || v === null || (typeof v === 'string' && v.length === 0)) {
      errors.push({
        rule: 'master-requires-promotion',
        severity: 'error',
        message: `tier:master atom must declare '${field}' (Master atoms are promoted from Genesis via ADR-evidence; they are not authored directly)`,
      })
    }
  }

  const promotionAdr = atom.fm['promotion_adr']
  if (typeof promotionAdr === 'string' && promotionAdr.length > 0) {
    const adrEntry = ctx.atomicIndex.get(promotionAdr)
    if (!adrEntry) {
      errors.push({
        rule: 'master-requires-promotion',
        severity: 'error',
        message: `promotion_adr '${promotionAdr}' does not resolve to any atom in the index`,
      })
    } else if (adrEntry.type !== 'adr') {
      errors.push({
        rule: 'master-requires-promotion',
        severity: 'error',
        message: `promotion_adr '${promotionAdr}' has type '${adrEntry.type}', expected 'adr'`,
      })
    } else if (adrEntry.status !== 'active' && adrEntry.status !== 'stable') {
      errors.push({
        rule: 'master-requires-promotion',
        severity: 'error',
        message: `promotion_adr '${promotionAdr}' has status '${adrEntry.status}', expected 'active' or 'stable' to prove user authorization`,
      })
    }
  }

  if (typeof atom.fm['learned_from'] === 'object' && atom.fm['learned_from'] !== null) {
    errors.push({
      rule: 'master-requires-promotion',
      severity: 'error',
      message: `tier:master atom must NOT carry 'learned_from'; provenance lives on the pre-promotion Genesis atom + the promotion ADR`,
    })
  }
  return errors
}
