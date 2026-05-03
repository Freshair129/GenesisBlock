import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

const HEADING_RE = (h: string) => new RegExp(`^##\\s+${h}\\b`, 'im')

/**
 * Per ADR--ANTI-HALLUCINATION-RULES (rule #4): every ADR atom body must contain
 * three sections: Context, Decision, Consequences. Case-insensitive headings.
 *
 * The check is structural: ## Context, ## Decision, ## Consequences must each
 * appear at least once. Order doesn't matter. Sub-sections (### …) under each
 * are fine but don't count as a top-level heading match.
 */
export function evidenceForDecisions(
  atom: ParsedAtom,
  _ctx: ValidationContext,
): ValidationError[] {
  const type = atom.fm.type
  if (typeof type !== 'string' || type.toLowerCase() !== 'adr') return []

  const errors: ValidationError[] = []
  const required = ['Context', 'Decision', 'Consequences']
  for (const heading of required) {
    if (!HEADING_RE(heading).test(atom.body)) {
      errors.push({
        rule: 'evidence-for-decisions',
        severity: 'error',
        message: `ADR body must include a '## ${heading}' section`,
        offending: heading,
      })
    }
  }
  return errors
}
