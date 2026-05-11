import { basename } from 'node:path'

import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

const REVISION_SUFFIX_RE = /\.rev-[a-z0-9-]+$/

function expectedId(fm: Record<string, unknown>): string | undefined {
  const id = fm['id']
  if (typeof id === 'string' && id.length > 0) return id
  const proposed = fm['proposed_id']
  if (typeof proposed === 'string' && proposed.length > 0) return proposed
  return undefined
}

function stemFromPath(filepath: string): string {
  const base = basename(filepath)
  const noExt = base.replace(/\.md$/, '')
  return noExt.replace(REVISION_SUFFIX_RE, '')
}

export function idFilenameMatch(
  atom: ParsedAtom,
  _ctx: ValidationContext,
): ValidationError[] {
  const id = expectedId(atom.fm)
  if (id === undefined) return []
  const stem = stemFromPath(atom.filepath)
  if (stem !== id) {
    return [
      {
        rule: 'id-filename-match',
        severity: 'error',
        message: `id '${id}' does not match filename stem '${stem}'`,
        offending: id,
      },
    ]
  }
  return []
}
