import { buildAliases, lookupType } from '../utils/registry.js'
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

  const type = atom.fm['type']
  const list =
    typeof type === 'string'
      ? cfg.byType.get(type.toLowerCase()) ?? cfg.default
      : cfg.default
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
      continue
    }

    if (field === 'aliases') {
      const val = atom.fm[field]
      if (!Array.isArray(val) || val.length === 0 || typeof val[0] !== 'string') {
        errors.push({
          rule: 'required-fields',
          severity: 'error',
          message: `frontmatter field 'aliases' must be a non-empty string array`,
          offending: 'aliases',
        })
        continue
      }
      const expectedPrimary = (atom.fm.id ?? atom.fm.proposed_id) as string | undefined
      if (expectedPrimary !== undefined) {
        const root = ctx.root ?? process.cwd()
        const target = buildAliases(expectedPrimary, undefined, root)
        
        for (let i = 0; i < target.length; i++) {
          if (val[i] !== target[i]) {
            errors.push({
              rule: 'required-fields',
              severity: 'error',
              message: `alias at index ${i} must match the expected '${target[i]}'`,
              offending: 'aliases',
            })
          }
        }
      }
    }

    if (field === 'cluster' || field === 'role') {
      const val = atom.fm[field]
      if (typeof val !== 'string' || val.trim() === '') {
        errors.push({
          rule: 'required-fields',
          severity: 'error',
          message: `frontmatter field '${field}' must be a non-empty string`,
          offending: field,
        })
        continue
      }

      const expectedPrimary = (atom.fm.id ?? atom.fm.proposed_id) as string | undefined
      if (expectedPrimary !== undefined) {
        const root = ctx.root ?? process.cwd()
        const prefix = expectedPrimary.split('-')[0]!
        const typeDef = lookupType(prefix, root)
        if (typeDef) {
          const expected = field === 'cluster' ? typeDef.cluster : typeDef.role
          if (val !== expected) {
            errors.push({
              rule: 'required-fields',
              severity: 'error',
              message: `frontmatter field '${field}' must match registry value '${expected}'`,
              offending: field,
            })
          }
        }
      }
    }
  }
  return errors
}
