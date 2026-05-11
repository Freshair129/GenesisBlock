import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

export const FORBIDDEN_FIELDS: ReadonlySet<string> = new Set([
  // Identity forgery
  'commit_hash',
  'merge_commit',
  'tenant_id',
  'pr_number',
  'reviewer_approved_at',
  // Authority fields (MSP-only)
  'promotion_level',
  'validated_at',
  'validated_by',
  'msp_signature',
  'hash',
  // Runtime metrics
  'execution_count',
  'last_error',
  'uptime',
  'latency_p50',
  // Fabrication risk
  'adr_number_override',
  'feature_id_override',
  'incident_id',
])

export function forbiddenFields(
  atom: ParsedAtom,
  ctx: ValidationContext,
): ValidationError[] {
  const banned = ctx.forbiddenFields ?? FORBIDDEN_FIELDS
  const errors: ValidationError[] = []
  for (const key of Object.keys(atom.fm)) {
    if (banned.has(key)) {
      errors.push({
        rule: 'forbidden-fields',
        severity: 'error',
        message: `frontmatter contains forbidden field '${key}' — agents must not set this directly`,
        offending: key,
      })
    }
  }
  return errors
}
