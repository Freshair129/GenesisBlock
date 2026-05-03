import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

function isPresent(v: unknown): boolean {
  if (v === undefined || v === null) return false
  if (typeof v === 'string') return v.length > 0
  if (Array.isArray(v)) return v.length > 0
  if (typeof v === 'object') return Object.keys(v).length > 0
  return true
}

/**
 * Per ADR--ATOMIC-CONTRACT-SCHEMA (loaded via atomic_contract.yaml):
 * every atom must have all fields required by its type. The required
 * field set is `required_fields.by_type[<type>]` if defined, otherwise
 * `required_fields.default`.
 *
 * No-op when the contract didn't supply a required-fields config (e.g.
 * fresh project without atomic_contract.yaml). Fail-open per `ADR--FORBIDDEN-FIELDS-LIST`'s graceful-degradation precedent.
 */
export function requiredFields(
  atom: ParsedAtom,
  ctx: ValidationContext,
): ValidationError[] {
  const cfg = ctx.requiredFields
  if (!cfg) return []

  const type = typeof atom.fm.type === 'string' ? atom.fm.type.toLowerCase() : ''
  const list = cfg.byType.get(type) ?? cfg.default
  if (list.length === 0) return []

  const errors: ValidationError[] = []
  for (const field of list) {
    if (!isPresent(atom.fm[field])) {
      errors.push({
        rule: 'required-fields',
        severity: 'error',
        message: `frontmatter is missing required field '${field}' (for type '${type || '<unknown>'}')`,
        offending: field,
      })
    }
  }
  return errors
}
