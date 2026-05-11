import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

const REQUIRED_PROMOTION_FIELDS = ['promoted_from', 'promoted_at', 'promotion_adr'] as const

export function masterRequiresPromotion(
  atom: ParsedAtom,
  _ctx: ValidationContext,
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
  if (typeof atom.fm['learned_from'] === 'object' && atom.fm['learned_from'] !== null) {
    errors.push({
      rule: 'master-requires-promotion',
      severity: 'error',
      message: `tier:master atom must NOT carry 'learned_from'; provenance lives on the pre-promotion Genesis atom + the promotion ADR`,
    })
  }
  return errors
}
