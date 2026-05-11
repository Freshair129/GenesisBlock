import { readFile, readdir, stat } from 'node:fs/promises'
import { join, resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'

/**
 * One Master atom successfully composed: id, body (everything after the
 * frontmatter close `---`), and a heuristic token estimate.
 */
export interface ComposedMaster {
  id: string
  body: string
  tokenCount: number
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
 */
export interface ComposeResult {
  composed: ComposedMaster[]
  totalTokens: number
  missing: string[]
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
 *
 * Intentionally simple — tiktoken-grade accuracy is unnecessary for v1; the
 * Master token cap (warn 400 / error 600 per `FRAME--KNOWLEDGE-3-TIER`) is
 * itself a soft budget, not a hard one. Empty / whitespace-only bodies
 * return 0.
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

/**
 * Locate the Markdown file for a given atom id. Tries `gks/master/<id>.md`
 * first (the canonical home for Master atoms). If that misses, scans `gks/`
 * recursively as a fallback so atoms living elsewhere (e.g. promoted but
 * not yet relocated) are still discoverable. Returns `null` if no file is
 * found.
 */
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
 * Compose a list of Master atom bodies for prompt injection.
 *
 * For each requested id:
 * - looks up `gks/master/<id>.md` (or scans `gks/` if not at canonical path)
 * - parses frontmatter; if `tier !== 'master'`, the id is added to `missing`
 *   instead of `composed`
 * - extracts body (everything after the closing `---`)
 * - estimates token count via {@link estimateTokens}
 *
 * The output preserves the input order. Ids not found on disk are also
 * placed in `missing`. This function is pure: no caching, no side effects.
 */
export async function composeMasterAtoms(
  ids: string[],
  root: string,
): Promise<ComposeResult> {
  const composed: ComposedMaster[] = []
  const missing: string[] = []
  let totalTokens = 0

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
    if (parsed === null) {
      missing.push(id)
      continue
    }
    if (parsed.fm.tier !== 'master') {
      missing.push(id)
      continue
    }
    const body = parsed.body.replace(/\s+$/, '')
    const tokenCount = estimateTokens(body)
    composed.push({ id, body, tokenCount })
    totalTokens += tokenCount
  }

  return { composed, totalTokens, missing }
}

/**
 * Format a {@link ComposeResult} as a single prompt fragment string.
 *
 * Each atom's body is preceded by an HTML comment marker (`<!-- {id} -->`)
 * so downstream consumers can locate / strip individual atoms; atoms are
 * separated by `\n\n---\n\n`. Returns the empty string when nothing was
 * composed.
 */
export function formatAsPromptFragment(result: ComposeResult): string {
  if (result.composed.length === 0) return ''
  return result.composed
    .map((atom) => `<!-- ${atom.id} -->\n${atom.body}`)
    .join('\n\n---\n\n')
}
