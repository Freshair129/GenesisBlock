import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

// GKS canonical pattern (atomic-id.js in @freshair129/gks): TYPE--SLUG with
// UPPERCASE letters, digits, hyphens and underscores. We extend with:
//   - ADR-NNN (numeric form per msp_spec.md §4.4)
//   - HOTFIX--<hex>  (gks hotfix open emits 7-char SHA, lowercase hex —
//                     discovered via M5b dogfood; GksV3 accepts this shape
//                     in its own atomic_index even though atomic-id.js
//                     would reject it on input. We accept on read.)
export const ID_PATTERN =
  /^(?:ADR-\d{3}|HOTFIX--[a-f0-9]+|[A-Z][A-Z0-9_]*--[A-Z0-9][A-Z0-9_-]*)$/

function getId(fm: Record<string, unknown>): string | undefined {
  const id = fm['id']
  if (typeof id === 'string' && id.length > 0) return id
  const proposed = fm['proposed_id']
  if (typeof proposed === 'string' && proposed.length > 0) return proposed
  return undefined
}

export function idFormat(
  atom: ParsedAtom,
  _ctx: ValidationContext,
): ValidationError[] {
  const id = getId(atom.fm)
  if (id === undefined) {
    return [
      {
        rule: 'id-format',
        severity: 'error',
        message: 'frontmatter is missing both `id` and `proposed_id`',
      },
    ]
  }
  if (!ID_PATTERN.test(id)) {
    return [
      {
        rule: 'id-format',
        severity: 'error',
        message: `id '${id}' does not match the canonical pattern (TYPE--SLUG or ADR-NNN)`,
        offending: id,
      },
    ]
  }
  return []
}
