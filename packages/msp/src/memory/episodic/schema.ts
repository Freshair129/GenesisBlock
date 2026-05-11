import type { Episode } from './types.js'
import { EpisodicSchemaError } from './types.js'

function isNonEmptyString(v: unknown): v is string {
  return typeof v === 'string' && v.length > 0
}

export function validateEpisode(e: unknown): Episode {
  if (!e || typeof e !== 'object' || Array.isArray(e)) {
    throw new EpisodicSchemaError('episode must be an object')
  }
  const o = e as Record<string, unknown>

  for (const k of ['episodicId', 'sessionId', 'projectId']) {
    if (!isNonEmptyString(o[k])) {
      throw new EpisodicSchemaError(`missing or empty ${k}`)
    }
  }

  const score = o.importance_score
  if (typeof score !== 'number' || !Number.isFinite(score) || score < 0 || score > 1) {
    throw new EpisodicSchemaError(
      `importance_score must be a number in [0, 1], got ${String(score)}`,
    )
  }

  if (!Array.isArray(o.range) || o.range.length === 0) {
    throw new EpisodicSchemaError('range must be a non-empty array')
  }
  if (!o.range.every((x) => typeof x === 'string')) {
    throw new EpisodicSchemaError('range entries must be strings')
  }

  const content = o.content
  if (!content || typeof content !== 'object' || Array.isArray(content)) {
    throw new EpisodicSchemaError('content must be an object')
  }
  if (!isNonEmptyString((content as Record<string, unknown>).summary)) {
    throw new EpisodicSchemaError('content.summary must be a non-empty string')
  }

  return e as Episode
}
