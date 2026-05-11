import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

const VALID_STATUSES = new Set([
  'stub',
  'raw',
  'draft',
  'active',
  'stable',
  'deprecated',
  'superseded',
  'partial',
])

export function phaseStatus(
  atom: ParsedAtom,
  _ctx: ValidationContext,
): ValidationError[] {
  const errors: ValidationError[] = []
  const status = atom.fm['status']
  const phase = atom.fm['phase']

  if (typeof status === 'string' && status.length > 0 && !VALID_STATUSES.has(status)) {
    errors.push({
      rule: 'phase-status',
      severity: 'error',
      message: `status '${status}' is not one of: ${Array.from(VALID_STATUSES).join(', ')}`,
      offending: status,
    })
  }

  const phaseNum = typeof phase === 'number' ? phase : Number.NaN
  if (Number.isFinite(phaseNum) && (phaseNum < 0 || phaseNum > 6)) {
    errors.push({
      rule: 'phase-status',
      severity: 'error',
      message: `phase ${phase} out of range (must be 0..6)`,
      offending: String(phase),
    })
  }

  if (typeof phase === 'number' && phase <= 1 && status === 'stable') {
    // P0/P1 atoms can be 'stable' if they're concept-locked. No error here.
  }

  return errors
}
