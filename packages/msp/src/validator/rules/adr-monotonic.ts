import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

const ADR_NUMERIC_RE = /^ADR-(\d{3})$/

function getId(fm: Record<string, unknown>): string | undefined {
  const id = fm['id']
  if (typeof id === 'string' && id.length > 0) return id
  const proposed = fm['proposed_id']
  if (typeof proposed === 'string' && proposed.length > 0) return proposed
  return undefined
}

export function adrMonotonic(
  atom: ParsedAtom,
  ctx: ValidationContext,
): ValidationError[] {
  const id = getId(atom.fm)
  if (!id) return []
  const m = ADR_NUMERIC_RE.exec(id)
  if (!m) return []

  const newNum = Number.parseInt(m[1]!, 10)

  let max = 0
  let found = false
  for (const key of ctx.atomicIndex.keys()) {
    const existing = ADR_NUMERIC_RE.exec(key)
    if (!existing) continue
    if (key === id) continue
    found = true
    const n = Number.parseInt(existing[1]!, 10)
    if (n > max) max = n
  }

  const expected = found ? max + 1 : 1
  if (newNum !== expected) {
    return [
      {
        rule: 'adr-monotonic',
        severity: 'error',
        message: `ADR id '${id}' must be ADR-${String(expected).padStart(3, '0')} (max existing + 1)`,
        offending: id,
      },
    ]
  }
  return []
}
