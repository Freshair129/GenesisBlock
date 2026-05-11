import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

export function futureDate(
  atom: ParsedAtom,
  ctx: ValidationContext,
): ValidationError[] {
  const value = atom.fm['created_at']
  if (typeof value !== 'string' || value.length === 0) return []
  const ts = Date.parse(value)
  if (!Number.isFinite(ts)) {
    return [
      {
        rule: 'future-date',
        severity: 'error',
        message: `created_at '${value}' is not a valid ISO-8601 date`,
        offending: value,
      },
    ]
  }
  const now = (ctx.now ?? new Date()).getTime()
  if (ts > now) {
    return [
      {
        rule: 'future-date',
        severity: 'error',
        message: `created_at '${value}' is in the future`,
        offending: value,
      },
    ]
  }
  return []
}
