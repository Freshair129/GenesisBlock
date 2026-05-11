/**
 * PROTO--PHASE-GATES — enforce P0..P6 phase ordering at PR-time (M8b).
 *
 * Enforces `FRAME--PHASE-GOVERNANCE`. Two checks:
 *
 *   1. **Hard error** — every phase-5 or phase-6 atom (FEAT/AUDIT) that writes
 *      code via `linked_symbols` MUST be preceded by a phase-3 BLUEPRINT atom
 *      whose own `linked_symbols` covers at least one of the same files.
 *      Escape hatch: `phase_override: { skip_blueprint: true, reason: ... }`
 *      in the atom's frontmatter (read from disk, not the atomic index).
 *
 *   2. **Soft warning** — a phase-2 ADR atom that doesn't reference any
 *      phase-1 CONCEPT via `crosslinks.references` is suspicious (an ADR
 *      should usually cite the CONCEPT it decides on).
 *
 * Ships as `status: draft` in `gks/proto/PROTO--PHASE-GATES.md` so even when
 * the predicate flags real atoms, CI doesn't fail-exit (gradual rollout).
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
import type { AtomicIndexEntry } from '../types.js'

/** Resolve a string from a `linked_symbols` array element. */
function extractFile(symbol: unknown): string | null {
  if (typeof symbol !== 'object' || symbol === null) return null
  const file = (symbol as { file?: unknown }).file
  return typeof file === 'string' && file.length > 0 ? file : null
}

/** Collect every `linked_symbols[*].file` (string) from an atom entry. */
function collectFiles(entry: AtomicIndexEntry): string[] {
  if (!Array.isArray(entry.linked_symbols)) return []
  const out: string[] = []
  for (const s of entry.linked_symbols) {
    const f = extractFile(s)
    if (f !== null) out.push(f)
  }
  return out
}

/** Parse YAML frontmatter from raw markdown; returns `null` if absent. */
function parseFrontmatter(raw: string): Record<string, unknown> | null {
  const m = raw.match(/^---\s*\n([\s\S]*?)\n---\s*(?:\n|$)/)
  if (!m) return null
  try {
    const fm = parseYaml(m[1]!)
    return fm && typeof fm === 'object' && !Array.isArray(fm)
      ? (fm as Record<string, unknown>)
      : null
  } catch {
    return null
  }
}

/**
 * Read an atom's frontmatter and return `true` iff it sets
 * `phase_override.skip_blueprint: true`.
 *
 * Path resolution: `entry.path` is repo-relative under `gks/`
 * (e.g. `audit/AUDIT--FOO.md`). We resolve `<repoRoot>/gks/<path>`.
 *
 * On any read / parse failure, returns `false` (fail-closed: the gate fires).
 */
async function hasSkipBlueprint(
  entry: AtomicIndexEntry,
  repoRoot: string,
): Promise<boolean> {
  if (typeof entry.path !== 'string' || entry.path.length === 0) return false
  const abs = resolve(repoRoot, 'gks', entry.path)
  let raw: string
  try {
    raw = await readFile(abs, 'utf8')
  } catch {
    return false
  }
  const fm = parseFrontmatter(raw)
  if (fm === null) return false
  const override = fm['phase_override']
  if (typeof override !== 'object' || override === null) return false
  const skip = (override as { skip_blueprint?: unknown }).skip_blueprint
  return skip === true
}

/**
 * The predicate.
 *
 * Pure-ish: reads atom files from disk to inspect `phase_override`. No
 * mutation, no network, no LLM.
 */
const predicate: Predicate = async (
  ctx: PredicateContext,
): Promise<PredicateResult> => {
  const violations: PredicateViolation[] = []

  // Pre-compute: every phase-3 BLUEPRINT's linked_symbols files (the union).
  const blueprintFiles = new Set<string>()
  for (const a of ctx.atomicIndex) {
    if (a.phase !== 3 || a.type !== 'blueprint') continue
    for (const f of collectFiles(a)) blueprintFiles.add(f)
  }

  // Phase-5/6 atoms (FEAT or AUDIT writing code) need a backing phase-3 BLUEPRINT.
  const codeWriters = ctx.atomicIndex.filter((a) => {
    if (a.phase !== 5 && a.phase !== 6) return false
    if (a.type !== 'feat' && a.type !== 'audit') return false
    return collectFiles(a).length > 0
  })

  for (const a of codeWriters) {
    const files = collectFiles(a)
    const covered = files.some((f) => blueprintFiles.has(f))
    if (covered) continue

    // Check escape hatch before reporting.
    const skip = await hasSkipBlueprint(a, ctx.repoRoot)
    if (skip) continue

    violations.push({
      atomId: a.id,
      message: `phase-${a.phase} ${a.type} writes code via linked_symbols (${files
        .slice(0, 3)
        .join(
          ', ',
        )}${files.length > 3 ? ', …' : ''}) but no phase-3 BLUEPRINT covers any of those files; add a BLUEPRINT or set phase_override.skip_blueprint: true`,
      severity: 'error',
    })
  }

  // Soft warning: ADR (phase 2) with no CONCEPT (phase 1) referenced.
  for (const a of ctx.atomicIndex) {
    if (a.phase !== 2 || a.type !== 'adr') continue
    const refs = a.crosslinks?.references
    const hasConcept =
      Array.isArray(refs) && refs.some((id) => typeof id === 'string' && id.startsWith('CONCEPT--'))
    if (hasConcept) continue
    violations.push({
      atomId: a.id,
      message: `ADR has no CONCEPT-- referenced via crosslinks.references; an ADR should usually decide on a CONCEPT`,
      severity: 'warning',
    })
  }

  return { ok: violations.length === 0, violations }
}

export default predicate
