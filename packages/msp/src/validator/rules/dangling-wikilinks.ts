import { extractWikilinks } from '../parse.js'
import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

const CROSSLINK_KEYS = [
  'implements',
  'references',
  'used_by',
  'contradicts',
  'supersedes',
  'partially_supersedes',
  'superseded_by',
  'partially_superseded_by',
  'resolves',
] as const

function selfId(fm: Record<string, unknown>): string | undefined {
  const id = fm['id']
  if (typeof id === 'string' && id.length > 0) return id
  const proposed = fm['proposed_id']
  if (typeof proposed === 'string' && proposed.length > 0) return proposed
  return undefined
}

export function danglingWikilinks(
  atom: ParsedAtom,
  ctx: ValidationContext,
): ValidationError[] {
  const errors: ValidationError[] = []
  const me = selfId(atom.fm)
  const exists = (id: string): boolean => id === me || ctx.atomicIndex.has(id)

  for (const link of extractWikilinks(atom.body)) {
    if (!exists(link.id)) {
      errors.push({
        rule: 'dangling-wikilink',
        severity: 'error',
        line: link.line,
        column: link.column,
        message: `wikilink [[${link.id}]] does not resolve to any atom in the index`,
        offending: link.id,
      })
    }
  }

  const crosslinks = atom.fm['crosslinks']
  if (
    crosslinks &&
    typeof crosslinks === 'object' &&
    !Array.isArray(crosslinks)
  ) {
    const cl = crosslinks as Record<string, unknown>
    for (const key of CROSSLINK_KEYS) {
      const list = cl[key]
      if (!Array.isArray(list)) continue
      for (const id of list) {
        if (typeof id !== 'string') continue
        if (!exists(id)) {
          errors.push({
            rule: 'dangling-wikilink',
            severity: 'error',
            message: `crosslinks.${key} references unknown atom '${id}'`,
            offending: id,
          })
        }
      }
    }
  }

  return errors
}
