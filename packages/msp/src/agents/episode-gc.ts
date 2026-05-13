import { mkdir, readFile, readdir, rename, stat, unlink } from 'node:fs/promises'
import { join, resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'

/**
 * Phase F4 — Episode retention + GC.
 *
 * Contract: see CONCEPT--EPISODE-RETENTION + ADR--EPISODE-GC-POLICY.
 *
 * gcEpisodes(root, opts?) scans `<root>/gks/episode/` (top-level only) and,
 * for each `EPISODE--*.md`, decides whether to keep / archive / delete based
 * on the inferred age + ok-ness + severity of the episode.
 *
 * Write side (`result-recorder.ts`) is intentionally untouched.
 */

export interface GcOpts {
  /** Episodes newer than `now − keep_days` are always kept. Default 30. */
  keep_days?: number
  /** If true, eligible episodes are unlink()ed instead of archived. Default false. */
  delete?: boolean
  /** If true, plan only — never touch the filesystem. Default false. */
  dry_run?: boolean
}

export interface GcReport {
  total_scanned: number
  archived: number
  deleted: number
  kept: number
  errors: string[]
}

const ARCHIVE_DIRNAME = '_archive'
const EPISODE_PREFIX = 'EPISODE--'
const MD_SUFFIX = '.md'
const DEFAULT_KEEP_DAYS = 30
const MS_PER_DAY = 24 * 60 * 60 * 1000

interface ParsedEpisode {
  filename: string
  abspath: string
  created_at: Date
  ok: boolean
  severity: 'critical' | 'regular' | 'low'
}

/**
 * Run garbage collection over <root>/gks/episode/.
 */
export async function gcEpisodes(
  root: string,
  opts: GcOpts = {},
): Promise<GcReport> {
  const keep_days = opts.keep_days ?? DEFAULT_KEEP_DAYS
  const wantDelete = opts.delete === true
  const dryRun = opts.dry_run === true

  const report: GcReport = {
    total_scanned: 0,
    archived: 0,
    deleted: 0,
    kept: 0,
    errors: [],
  }

  const episodeDir = resolve(root, 'gks', 'episode')

  // Resolve directory; if it doesn't exist there's nothing to GC.
  let entries: string[]
  try {
    entries = await readdir(episodeDir)
  } catch (err) {
    const e = err as NodeJS.ErrnoException
    if (e.code === 'ENOENT') return report
    report.errors.push(`readdir(${episodeDir}): ${e.message}`)
    return report
  }

  const cutoff = new Date(Date.now() - keep_days * MS_PER_DAY)

  for (const name of entries) {
    if (name === ARCHIVE_DIRNAME) continue
    if (!name.startsWith(EPISODE_PREFIX)) continue
    if (!name.endsWith(MD_SUFFIX)) continue

    const abspath = join(episodeDir, name)

    // Skip directories that happen to match the prefix (unlikely but defensive).
    let isFile = false
    try {
      const st = await stat(abspath)
      isFile = st.isFile()
    } catch (err) {
      const e = err as Error
      report.errors.push(`stat(${name}): ${e.message}`)
      continue
    }
    if (!isFile) continue

    report.total_scanned += 1

    let parsed: ParsedEpisode
    try {
      parsed = await parseEpisode(name, abspath)
    } catch (err) {
      const e = err as Error
      // Malformed — keep (conservative: when in doubt, do not destroy).
      report.errors.push(`parse(${name}): ${e.message}`)
      report.kept += 1
      continue
    }

    if (shouldKeep(parsed, cutoff)) {
      report.kept += 1
      continue
    }

    // Eligible for archive-or-delete.
    if (wantDelete) {
      if (dryRun) {
        report.deleted += 1
        continue
      }
      try {
        await unlink(abspath)
        report.deleted += 1
      } catch (err) {
        const e = err as Error
        report.errors.push(`unlink(${name}): ${e.message}`)
        report.kept += 1
      }
      continue
    }

    // Archive.
    const yyyymm = monthKey(parsed.created_at)
    const archiveSubdir = join(episodeDir, ARCHIVE_DIRNAME, yyyymm)
    const target = join(archiveSubdir, name)

    if (dryRun) {
      report.archived += 1
      continue
    }

    try {
      await mkdir(archiveSubdir, { recursive: true })
      await rename(abspath, target)
      report.archived += 1
    } catch (err) {
      const e = err as Error
      report.errors.push(`archive(${name}): ${e.message}`)
      report.kept += 1
    }
  }

  return report
}

/**
 * Decide whether to keep an episode given the cutoff date + parsed flags.
 *
 * - Recent (created_at ≥ cutoff) → always keep.
 * - Old + ok=false → keep (error audit trail).
 * - Old + severity=critical → keep (critical-as-evidence).
 * - Else → eligible for archive/delete.
 */
function shouldKeep(parsed: ParsedEpisode, cutoff: Date): boolean {
  if (parsed.created_at.getTime() >= cutoff.getTime()) return true
  if (parsed.ok === false) return true
  if (parsed.severity === 'critical') return true
  return false
}

/**
 * Parse an episode atom enough to make the keep/archive/delete decision.
 */
async function parseEpisode(
  filename: string,
  abspath: string,
): Promise<ParsedEpisode> {
  const raw = await readFile(abspath, 'utf8')
  const { frontmatter, body } = splitFrontmatter(raw)

  const fm = parseYaml(frontmatter) as Record<string, unknown> | null
  if (fm === null || typeof fm !== 'object') {
    throw new Error('frontmatter is not a YAML mapping')
  }

  const createdRaw = fm['created_at']
  if (typeof createdRaw !== 'string' && !(createdRaw instanceof Date)) {
    throw new Error('missing created_at')
  }
  const created_at = createdRaw instanceof Date ? createdRaw : new Date(createdRaw)
  if (Number.isNaN(created_at.getTime())) {
    throw new Error(`unparseable created_at: ${String(createdRaw)}`)
  }

  const ok = inferOk(fm, body)
  const severity = inferSeverity(fm, body)

  return {
    filename,
    abspath,
    created_at,
    ok,
    severity,
  }
}

/**
 * Inference rule for `ok` — see CONCEPT--EPISODE-RETENTION §"Inferring `ok`".
 *
 * Priority (first match wins):
 *   1. Frontmatter `ok: false` → false.
 *   2. Frontmatter `exit_code: <n>` with n !== 0 → false.
 *   3. No `- tier_used:` body bullet → false (malformed; keep).
 *   4. Empty `## Output` block → false.
 *   5. Otherwise → true.
 */
function inferOk(
  fm: Record<string, unknown>,
  body: string,
): boolean {
  if (fm['ok'] === false) return false

  const exitCode = fm['exit_code']
  if (typeof exitCode === 'number' && exitCode !== 0) return false

  if (!/^[ \t]*-[ \t]+tier_used:/m.test(body)) return false

  const outputContent = extractOutputBlock(body)
  if (outputContent !== null && outputContent.trim() === '') return false

  return true
}

/**
 * Inference rule for `severity` — see CONCEPT--EPISODE-RETENTION.
 */
function inferSeverity(
  fm: Record<string, unknown>,
  body: string,
): 'critical' | 'regular' | 'low' {
  const fmSev = fm['severity']
  if (typeof fmSev === 'string') {
    if (fmSev === 'critical' || fmSev === 'regular' || fmSev === 'low') {
      return fmSev
    }
  }

  const bodyMatch = body.match(/^[ \t]*-[ \t]+task\.severity:[ \t]*(\S+)/m)
  if (bodyMatch !== null) {
    const v = bodyMatch[1]
    if (v === 'critical' || v === 'regular' || v === 'low') return v
  }

  return 'regular'
}

/**
 * Extract the content of the first ```...``` fence under a `## Output` heading.
 * Returns null when no such block exists.
 */
function extractOutputBlock(body: string): string | null {
  const headerMatch = body.match(/^##[ \t]+Output[ \t]*$/m)
  if (headerMatch === null || headerMatch.index === undefined) return null
  const afterHeader = body.slice(headerMatch.index + headerMatch[0].length)
  const fenceMatch = afterHeader.match(/```[^\n]*\n([\s\S]*?)\n```/)
  if (fenceMatch === null) return null
  return fenceMatch[1] ?? ''
}

/**
 * Split a markdown atom into frontmatter (YAML body) + post-frontmatter body.
 */
function splitFrontmatter(raw: string): { frontmatter: string; body: string } {
  if (!raw.startsWith('---')) {
    throw new Error('atom does not start with --- frontmatter delimiter')
  }
  // Locate the closing --- on its own line.
  const closing = raw.indexOf('\n---', 3)
  if (closing === -1) {
    throw new Error('frontmatter has no closing --- delimiter')
  }
  const fmStart = raw.indexOf('\n', 3) + 1 // first content line of frontmatter
  const frontmatter = raw.slice(fmStart, closing)
  // Body starts after the closing ---\n
  const bodyStart = raw.indexOf('\n', closing + 4)
  const body = bodyStart === -1 ? '' : raw.slice(bodyStart + 1)
  return { frontmatter, body }
}

/**
 * Return YYYY-MM (UTC) for a Date — used to bucket the archive subdir.
 */
function monthKey(d: Date): string {
  const yyyy = d.getUTCFullYear().toString().padStart(4, '0')
  const mm = (d.getUTCMonth() + 1).toString().padStart(2, '0')
  return `${yyyy}-${mm}`
}
