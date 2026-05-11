/**
 * PROTO--MASTER-BODY-SCHEMA — body-section schema for `tier: master` atoms
 * (PR-5 of the 3-tier rollout).
 *
 * `FRAME--KNOWLEDGE-3-TIER` defines the Master body contract: every Master
 * atom must declare five top-level H2 sections in this order:
 *
 *   ## Intent          — 1–2 sentences of what behavior this Master enforces
 *   ## Why             — rationale, for human review
 *   ## Directives      — numbered, imperative — what the agent must do
 *   ## Apply when      — triggers — when this Master is relevant
 *   ## Conflicts with  — atom IDs that may contradict (may be empty)
 *
 * The predicate iterates `atomicIndex` for `tier === 'master'` entries, reads
 * each atom file from disk (`<repoRoot>/gks/<entry.path>`), strips the YAML
 * frontmatter, and asserts the five H2 headings exist by exact case-sensitive
 * string match. A missing section emits a `severity: 'error'` violation.
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

export const REQUIRED_SECTIONS = [
  '## Intent',
  '## Why',
  '## Directives',
  '## Apply when',
  '## Conflicts with',
] as const

/** Strip a leading `---\nYAML\n---\n` block; return body only. */
export function stripFrontmatter(raw: string): string {
  const m = raw.match(/^---\s*\n[\s\S]*?\n---\s*(?:\n|$)/)
  return m ? raw.slice(m[0].length) : raw
}

/** Find which H2 sections are missing from a Master body. */
export function missingSections(body: string): string[] {
  const lines = body.split(/\r?\n/)
  const seen = new Set<string>()
  for (const line of lines) {
    for (const section of REQUIRED_SECTIONS) {
      if (line === section || line.startsWith(`${section} `)) {
        seen.add(section)
      }
    }
  }
  const missing: string[] = []
  for (const section of REQUIRED_SECTIONS) {
    if (!seen.has(section)) missing.push(section)
  }
  return missing
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
    const missing = missingSections(body)
    if (missing.length > 0) {
      violations.push({
        atomId: entry.id,
        message: `${entry.id} (tier:master) missing required H2 section(s): ${missing.join(', ')}`,
        severity: 'error',
      })
    }
  }
  return { ok: violations.length === 0, violations }
}

export default predicate
