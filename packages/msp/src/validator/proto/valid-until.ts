/**
 * PROTO--VALID-UNTIL — decision atrophy guard (M9a).
 *
 * Scans every atom in the index for a frontmatter `valid_until:` field. For
 * atoms whose date has passed, emits a `warning` violation. For atoms expiring
 * within the next 30 days, emits an `info` violation. Atoms without a
 * `valid_until` field are skipped, as are `superseded` atoms (already known
 * to be obsolete).
 *
 * Rationale: see `CONCEPT--DECISION-ATROPHY-GUARDS`. ADRs / planning concepts
 * with explicit lifetimes go silently stale otherwise.
 *
 * The atomic index does not always carry `valid_until`, so this predicate
 * reads each atom file from disk and parses its frontmatter directly.
 *
 * Time injection: tests can override "now" via the `MSP_NOW` environment
 * variable (an ISO 8601 string). Default is the current wall clock.
 */

import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'

import type {
  Predicate,
  PredicateContext,
  PredicateResult,
  PredicateViolation,
} from './types.js'

const MS_PER_DAY = 86_400_000
const NEAR_EXPIRY_DAYS = 30

function resolveNow(): Date {
  const override = process.env['MSP_NOW']
  if (typeof override === 'string' && override.length > 0) {
    const candidate = new Date(override)
    if (!Number.isNaN(candidate.getTime())) return candidate
  }
  return new Date()
}

function parseFrontmatter(raw: string): Record<string, unknown> | null {
  const m = raw.match(/^---\s*\n([\s\S]*?)\n---/)
  if (!m) return null
  try {
    const parsed = parseYaml(m[1]!)
    if (parsed === null || typeof parsed !== 'object') return null
    return parsed as Record<string, unknown>
  } catch {
    return null
  }
}

function extractValidUntil(fm: Record<string, unknown>): string | null {
  const v = fm['valid_until']
  if (typeof v === 'string') return v
  // YAML may parse bare ISO dates as Date objects.
  if (v instanceof Date && !Number.isNaN(v.getTime())) {
    return v.toISOString()
  }
  return null
}

const predicate: Predicate = async (
  ctx: PredicateContext,
): Promise<PredicateResult> => {
  const violations: PredicateViolation[] = []
  const now = resolveNow()

  for (const atom of ctx.atomicIndex) {
    if (atom.status === 'superseded') continue

    // `path` in atomic index entries is repo-relative under gks/.
    const fullPath = resolve(ctx.repoRoot, 'gks', atom.path)
    let raw: string
    try {
      raw = await readFile(fullPath, 'utf8')
    } catch {
      continue
    }

    const fm = parseFrontmatter(raw)
    if (!fm) continue

    const validUntilRaw = extractValidUntil(fm)
    if (validUntilRaw === null) continue

    const expiry = new Date(validUntilRaw)
    if (Number.isNaN(expiry.getTime())) continue

    const daysUntilExpiry = Math.floor(
      (expiry.getTime() - now.getTime()) / MS_PER_DAY,
    )

    if (daysUntilExpiry < 0) {
      violations.push({
        atomId: atom.id,
        message: `expired ${-daysUntilExpiry} days ago (valid_until ${validUntilRaw})`,
        severity: 'warning',
      })
    } else if (daysUntilExpiry < NEAR_EXPIRY_DAYS) {
      violations.push({
        atomId: atom.id,
        message: `expires in ${daysUntilExpiry} days (valid_until ${validUntilRaw})`,
        severity: 'info',
      })
    }
  }

  // No 'error' severity emitted; warnings/infos don't fail-exit.
  const hasErrors = violations.some((v) => v.severity === 'error')
  return { ok: !hasErrors, violations }
}

export default predicate
