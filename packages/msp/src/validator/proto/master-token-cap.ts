/**
 * PROTO--MASTER-TOKEN-CAP — body token-budget guard for `tier: master` atoms
 * (PR-5 of the 3-tier rollout).
 *
 * `FRAME--KNOWLEDGE-3-TIER` caps Master atom bodies so they stay
 * prompt-injectable. The heuristic token count is whitespace-split words
 * multiplied by 1.3 (a rough conservative ratio of tokens per word for the
 * BPE-style tokenisers most LLMs ship).
 *
 *   token_count = body.split(/\s+/).filter(Boolean).length * 1.3
 *
 * Thresholds (from the FRAME):
 *   > 400  → warning
 *   > 600  → error
 *
 * The predicate iterates `atomicIndex` for `tier === 'master'` entries, reads
 * each atom file from disk (`<repoRoot>/gks/<entry.path>`), strips the YAML
 * frontmatter, and emits a `warning` or `error` violation when the body
 * exceeds the corresponding threshold.
 */
import { readFile } from 'node:fs/promises'
import { isAbsolute, join, resolve } from 'node:path'

import type { AtomicIndexEntry } from '../types.js'

import type {
  Predicate,
  PredicateContext,
  PredicateResult,
  PredicateViolation,
} from './types.js'

export const TOKEN_RATIO = 1.3
export const WARN_THRESHOLD = 400
export const ERROR_THRESHOLD = 600

/** Strip a leading `---\nYAML\n---\n` block; return body only. */
export function stripFrontmatter(raw: string): string {
  const m = raw.match(/^---\s*\n[\s\S]*?\n---\s*(?:\n|$)/)
  return m ? raw.slice(m[0].length) : raw
}

/** Heuristic body-token count: whitespace-split words × TOKEN_RATIO. */
export function estimateTokens(body: string): number {
  const words = body.split(/\s+/).filter((w) => w.length > 0)
  return words.length * TOKEN_RATIO
}

function readField(entry: AtomicIndexEntry, key: string): unknown {
  return (entry as unknown as Record<string, unknown>)[key]
}

function resolveAtomPath(repoRoot: string, relPath: string): string {
  if (isAbsolute(relPath)) return relPath
  return resolve(join(repoRoot, 'gks', relPath))
}

const predicate: Predicate = async (
  ctx: PredicateContext,
): Promise<PredicateResult> => {
  const violations: PredicateViolation[] = []
  for (const entry of ctx.atomicIndex) {
    if (readField(entry, 'tier') !== 'master') continue
    const abs = resolveAtomPath(ctx.repoRoot, entry.path)
    let raw: string
    try {
      raw = await readFile(abs, 'utf8')
    } catch (err) {
      violations.push({
        atomId: entry.id,
        message: `${entry.id}: cannot read body at ${entry.path} (${(err as Error).message})`,
        severity: 'error',
      })
      continue
    }
    const body = stripFrontmatter(raw)
    const tokens = estimateTokens(body)
    if (tokens > ERROR_THRESHOLD) {
      violations.push({
        atomId: entry.id,
        message: `${entry.id} (tier:master) body ~${tokens.toFixed(0)} tokens exceeds error cap of ${ERROR_THRESHOLD}`,
        severity: 'error',
      })
    } else if (tokens > WARN_THRESHOLD) {
      violations.push({
        atomId: entry.id,
        message: `${entry.id} (tier:master) body ~${tokens.toFixed(0)} tokens exceeds warn cap of ${WARN_THRESHOLD}`,
        severity: 'warning',
      })
    }
  }
  return { ok: violations.length === 0, violations }
}

export default predicate
