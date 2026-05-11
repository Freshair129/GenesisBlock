import type { SessionTurn } from './types.js'
import { SessionSchemaError } from './types.js'

const REQUIRED: Array<keyof SessionTurn> = [
  'sessionId',
  'episodicId',
  'turnId',
  'msgId',
  'speakerId',
  'content',
]

export function validateTurn(row: unknown): SessionTurn {
  if (!row || typeof row !== 'object' || Array.isArray(row)) {
    throw new SessionSchemaError(REQUIRED.map(String))
  }
  const r = row as Partial<SessionTurn>
  const missing: string[] = []
  for (const f of REQUIRED) {
    const v = r[f]
    if (v === undefined || v === null) {
      missing.push(f)
      continue
    }
    if (f === 'turnId') {
      if (typeof v !== 'number' || !Number.isFinite(v)) missing.push(f)
    } else if (typeof v !== 'string' || v.length === 0) {
      missing.push(f)
    }
  }
  if (missing.length > 0) throw new SessionSchemaError(missing)
  return r as SessionTurn
}

/**
 * JSONL is one row per line. Embedded newlines in `content` would split the
 * row across lines. JSON.stringify already escapes them — this is a defence-
 * in-depth check.
 */
export function serialiseTurn(row: SessionTurn): string {
  return JSON.stringify(row)
}
