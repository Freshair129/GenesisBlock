import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

const ALLOWED_TIERS = new Set(['safety', 'master', 'genesis', 'process'])
const ALLOWED_SOURCE_TYPES = new Set(['axiomatic', 'learned'])

export function tierEnum(
  atom: ParsedAtom,
  _ctx: ValidationContext,
): ValidationError[] {
  const errors: ValidationError[] = []
  const tier = atom.fm['tier']
  if (tier === undefined || tier === null) {
    errors.push({
      rule: 'tier-enum',
      severity: 'warning',
      message: `atom missing 'tier' field; expected one of ${[...ALLOWED_TIERS].join(' | ')}`,
    })
  } else if (typeof tier !== 'string' || !ALLOWED_TIERS.has(tier)) {
    errors.push({
      rule: 'tier-enum',
      severity: 'warning',
      message: `tier '${String(tier)}' not in allowed set { ${[...ALLOWED_TIERS].join(', ')} }`,
      offending: String(tier),
    })
  }
  const sourceType = atom.fm['source_type']
  if (sourceType !== undefined && sourceType !== null) {
    if (typeof sourceType !== 'string' || !ALLOWED_SOURCE_TYPES.has(sourceType)) {
      errors.push({
        rule: 'tier-enum',
        severity: 'warning',
        message: `source_type '${String(sourceType)}' not in allowed set { ${[...ALLOWED_SOURCE_TYPES].join(', ')} }`,
        offending: String(sourceType),
      })
    }
  }
  return errors
}
