/**
 * PROTO--AUTHORITY-ENFORCEMENT — sanity-check `.brain/msp/authority.yaml`.
 *
 * `FRAME--AUTHORITY-MATRIX` defines tiers (T1/T2/T3) and the paths each tier
 * can write. The full enforcement story (match git author email/login to
 * tier, then check every touched path against tier's allowed_paths) is a
 * CI-workflow concern: it needs the PR's git author + diff, neither of
 * which a PROTO predicate can see — predicates only get an atomicIndex +
 * repoRoot.
 *
 * What this PROTO can do today, and what it does:
 *
 *   1. Read `<repoRoot>/.brain/msp/authority.yaml`.
 *      - ENOENT → vacuous pass (project hasn't opted into the tier model).
 *      - Other I/O error → emit error violation.
 *   2. Parse YAML; bad YAML → error violation.
 *   3. Validate the shape:
 *      - top-level must be an object with `tiers` and `allowed_paths` maps,
 *        each containing T1/T2/T3 keys whose values are string arrays.
 *      - no user appears in more than one tier (sets are disjoint).
 *      - every tier's `allowed_paths` includes at least one entry that
 *        matches the candidates queue (substring `candidates`) so T1 has
 *        somewhere legal to drop proposals (per ADR--AGENT-WRITE-BOUNDARIES).
 *      - every path entry is a non-empty string.
 *
 * The CI workflow that does the actual git-author + diff matching is
 * tracked in `AUDIT--AUTHORITY-ENFORCEMENT-PROTO.md` as future work.
 */

import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'

import type {
  Predicate,
  PredicateContext,
  PredicateResult,
  PredicateViolation,
  Severity,
} from './types.js'

const TIERS = ['T1', 'T2', 'T3'] as const
type Tier = (typeof TIERS)[number]

interface AuthorityConfig {
  tiers: Record<Tier, string[]>
  allowed_paths: Record<Tier, string[]>
}

function isStringArray(v: unknown): v is unknown[] {
  return Array.isArray(v)
}

function asRecord(v: unknown): Record<string, unknown> | null {
  if (typeof v !== 'object' || v === null || Array.isArray(v)) return null
  return v as Record<string, unknown>
}

function violation(
  message: string,
  severity: Severity = 'error',
): PredicateViolation {
  return { message, severity }
}

/**
 * Validate the parsed YAML shape. Returns a list of violations (empty when ok).
 */
function validateShape(parsed: unknown): PredicateViolation[] {
  const violations: PredicateViolation[] = []
  const root = asRecord(parsed)
  if (!root) {
    violations.push(
      violation('authority.yaml: top-level must be a mapping with `tiers` and `allowed_paths`'),
    )
    return violations
  }

  const tiersRaw = asRecord(root['tiers'])
  if (!tiersRaw) {
    violations.push(
      violation('authority.yaml: missing or invalid `tiers` mapping (expected T1/T2/T3 → string[])'),
    )
  }

  const pathsRaw = asRecord(root['allowed_paths'])
  if (!pathsRaw) {
    violations.push(
      violation(
        'authority.yaml: missing or invalid `allowed_paths` mapping (expected T1/T2/T3 → string[])',
      ),
    )
  }

  // If either top-level key is missing, no point in deeper checks.
  if (!tiersRaw || !pathsRaw) return violations

  // Per-tier validation.
  const tierMembers: Record<Tier, string[]> = { T1: [], T2: [], T3: [] }
  for (const t of TIERS) {
    const arr = tiersRaw[t]
    if (!isStringArray(arr)) {
      violations.push(
        violation(`authority.yaml: tiers.${t} must be an array of strings`),
      )
      continue
    }
    const members: string[] = []
    for (let i = 0; i < arr.length; i++) {
      const entry = arr[i]
      if (typeof entry !== 'string' || entry.length === 0) {
        violations.push(
          violation(`authority.yaml: tiers.${t}[${i}] must be a non-empty string`),
        )
        continue
      }
      members.push(entry)
    }
    tierMembers[t] = members
  }

  // Disjointness check — no user in two tiers at once.
  const seen = new Map<string, Tier>()
  for (const t of TIERS) {
    for (const user of tierMembers[t]) {
      const prior = seen.get(user)
      if (prior && prior !== t) {
        violations.push(
          violation(
            `authority.yaml: user "${user}" appears in both tiers.${prior} and tiers.${t} — tiers must be disjoint`,
          ),
        )
        continue
      }
      seen.set(user, t)
    }
  }

  // Per-tier allowed_paths validation.
  const tierPaths: Record<Tier, string[]> = { T1: [], T2: [], T3: [] }
  for (const t of TIERS) {
    const arr = pathsRaw[t]
    if (!isStringArray(arr)) {
      violations.push(
        violation(
          `authority.yaml: allowed_paths.${t} must be an array of strings`,
        ),
      )
      continue
    }
    const collected: string[] = []
    for (let i = 0; i < arr.length; i++) {
      const entry = arr[i]
      if (typeof entry !== 'string' || entry.length === 0) {
        violations.push(
          violation(
            `authority.yaml: allowed_paths.${t}[${i}] must be a non-empty string`,
          ),
        )
        continue
      }
      collected.push(entry)
    }
    if (collected.length === 0) {
      violations.push({
        message: `authority.yaml: allowed_paths.${t} is empty — tier has no writable paths`,
        severity: 'warning',
      })
    }
    tierPaths[t] = collected
  }

  // Every tier should be able to write to candidates (so T1 has somewhere legal).
  for (const t of TIERS) {
    const paths = tierPaths[t]
    if (paths.length === 0) continue // already warned
    const hasCandidates = paths.some((p) => p.includes('candidates'))
    if (!hasCandidates) {
      violations.push(
        violation(
          `authority.yaml: allowed_paths.${t} must include at least one candidates-queue path (T1 needs somewhere to drop proposals)`,
        ),
      )
    }
  }

  return violations
}

const predicate: Predicate = async (
  ctx: PredicateContext,
): Promise<PredicateResult> => {
  const path = resolve(ctx.repoRoot, '.brain/msp/authority.yaml')

  let raw: string
  try {
    raw = await readFile(path, 'utf8')
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      return { ok: true, violations: [] }
    }
    return {
      ok: false,
      violations: [
        violation(
          `authority.yaml: cannot read ${path}: ${(err as Error).message}`,
        ),
      ],
    }
  }

  let parsed: unknown
  try {
    parsed = parseYaml(raw)
  } catch (err) {
    return {
      ok: false,
      violations: [
        violation(`authority.yaml: invalid YAML: ${(err as Error).message}`),
      ],
    }
  }

  const violations = validateShape(parsed)
  // ok is true only when there are no error-severity violations. Warnings
  // pass (loader's shouldFailExit also gates on severity:error).
  const hasError = violations.some((v) => v.severity === 'error')
  return { ok: !hasError, violations }
}

export default predicate

// Exported for tests — internal helpers.
export { validateShape, type AuthorityConfig }
