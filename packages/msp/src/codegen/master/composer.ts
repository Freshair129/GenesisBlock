import { readFile, readdir, stat } from 'node:fs/promises'
import { join, resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'
import { enforcePolicy, type PepOptions } from '../../policy/pep.js'
import { makeResource } from '../../policy/types.js'
import { assignResolutionTiers, type ResolutionTier } from '../../orchestrator/resolution/tier.js'
import { enforceResolutionBudget } from '../../orchestrator/resolution/budget.js'

/**
 * One Master atom successfully composed: id, body (everything after the
 * frontmatter close `---`), and a heuristic token estimate.
 */
export interface ComposedMaster {
  id: string
  body: string
  tokenCount: number
  tier: ResolutionTier
  /** §4 — Domain-specific attributes for policy checks. */
  attributes?: Record<string, any>
}

/**
 * Result of composing a list of Master atoms.
 *
 * - `composed` preserves the order of the input ids; missing ids are skipped
 *   (and surfaced separately on `missing`).
 * - `totalTokens` is the sum of all `composed[].tokenCount`.
 * - `missing` lists the requested ids that either don't exist on disk or
 *   exist but are not `tier: master`. Order is best-effort (matches the
 *   order ids were resolved).
 * - `dropped` lists ids that were found but rejected by policy (PEP).
 */
export interface ComposeResult {
  composed: ComposedMaster[]
  totalTokens: number
  missing: string[]
  dropped?: string[]
}

interface ParsedAtom {
  fm: Record<string, unknown>
  body: string
}

const FRONTMATTER_DELIM = '---'

/**
 * Parse a markdown atom into `{ fm, body }`. Returns `null` if the file is
 * malformed (missing or unclosed frontmatter, non-object YAML).
 */
function parseAtom(source: string): ParsedAtom | null {
  if (!source.startsWith(FRONTMATTER_DELIM)) return null
  const end = source.indexOf(`\n${FRONTMATTER_DELIM}`, FRONTMATTER_DELIM.length)
  if (end === -1) return null
  const fmText = source.slice(FRONTMATTER_DELIM.length, end).trim()
  let fm: unknown
  try {
    fm = parseYaml(fmText)
  } catch {
    return null
  }
  if (!fm || typeof fm !== 'object' || Array.isArray(fm)) return null
  const bodyStart = end + `\n${FRONTMATTER_DELIM}`.length
  const body = source.slice(bodyStart).replace(/^\n/, '')
  return { fm: fm as Record<string, unknown>, body }
}

/**
 * Heuristic token count: whitespace-split word count * 1.3, rounded.
 */
export function estimateTokens(text: string): number {
  const trimmed = text.trim()
  if (trimmed === '') return 0
  const words = trimmed.split(/\s+/).length
  return Math.round(words * 1.3)
}

async function pathExists(p: string): Promise<boolean> {
  try {
    await stat(p)
    return true
  } catch {
    return false
  }
}

async function findAtomFile(id: string, root: string): Promise<string | null> {
  const canonical = resolve(root, 'gks/master', `${id}.md`)
  if (await pathExists(canonical)) return canonical

  const gksDir = resolve(root, 'gks')
  if (!(await pathExists(gksDir))) return null
  return scanForId(gksDir, `${id}.md`)
}

async function scanForId(dir: string, filename: string): Promise<string | null> {
  let entries: import('node:fs').Dirent[]
  try {
    entries = await readdir(dir, { withFileTypes: true })
  } catch {
    return null
  }
  for (const entry of entries) {
    const full = join(dir, entry.name)
    if (entry.isDirectory()) {
      const found = await scanForId(full, filename)
      if (found) return found
    } else if (entry.isFile() && entry.name === filename) {
      return full
    }
  }
  return null
}

/**
 * Compose a list of Master atom bodies for prompt injection with Resolution Gradient (Phase 3).
 *
 * For each requested id:
 * - looks up atom on disk.
 * - parses frontmatter and extract attributes.
 * - evaluates policy via PEP; if rejected, the id is added to `dropped`.
 * - assigns tier (FULL / MENTION).
 * - enforces token budget (Layer 5).
 * - extracts body iff tier === FULL.
 */
export async function composeMasterAtoms(
  ids: string[],
  root: string,
  opts: { pep?: PepOptions; maxTokens?: number } = {},
): Promise<ComposeResult> {
  const candidates: Array<{ id: string; body: string; attributes: Record<string, any> }> = []
  const missing: string[] = []
  const dropped: string[] = []

  for (const id of ids) {
    const filepath = await findAtomFile(id, root)
    if (filepath === null) {
      missing.push(id)
      continue
    }
    let raw: string
    try {
      raw = await readFile(filepath, 'utf8')
    } catch {
      missing.push(id)
      continue
    }
    const parsed = parseAtom(raw)
    if (parsed === null || parsed.fm.tier !== 'master') {
      missing.push(id)
      continue
    }

    const attributes = (parsed.fm.attributes as Record<string, any>) ?? {}

    // 1. PEP enforcement (Phase 2)
    if (opts.pep) {
      const resource = makeResource('atom', id, {}, attributes)
      const { permitted } = await enforcePolicy(resource, opts.pep)
      if (!permitted) {
        dropped.push(id)
        continue
      }
    }

    candidates.push({ id, body: parsed.body.replace(/\s+$/, ''), attributes })
  }

  // 2. Assign Tiers (Phase 3 - Layer 4)
  const hitsWithTiers = assignResolutionTiers(
    candidates.map((c) => ({ atomId: c.id, attributes: c.attributes } as any)),
  )

  // 3. Enforce Budget (Phase 3 - Layer 5)
  const budgetedHits = enforceResolutionBudget(
    hitsWithTiers.map((h, i) => ({
      ...h,
      body: candidates[i]!.body,
    })),
    { maxTokens: opts.maxTokens },
  )

  const composed: ComposedMaster[] = []
  let totalTokens = 0

  for (const hit of budgetedHits) {
    const candidate = candidates.find((c) => c.id === hit.atomId)!
    const body = hit.tier === 'FULL' ? candidate.body : `[MENTION: ${hit.atomId}]`
    const tokenCount = estimateTokens(body)
    composed.push({
      id: hit.atomId,
      body,
      tokenCount,
      tier: hit.tier,
      attributes: candidate.attributes,
    })
    totalTokens += tokenCount
  }

  return { composed, totalTokens, missing, dropped }
}

/**
 * Format a {@link ComposeResult} as a single prompt fragment string.
 */
export function formatAsPromptFragment(result: ComposeResult): string {
  if (result.composed.length === 0) return ''
  return result.composed
    .map((atom) => `<!-- ${atom.id} [${atom.tier}] -->\n${atom.body}`)
    .join('\n\n---\n\n')
}
