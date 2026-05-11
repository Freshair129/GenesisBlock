/**
 * Adapter to wrap an existing core validator `Rule` (from `src/validator/rules/`)
 * as a PROTO `Predicate`.
 *
 * The core rules operate per-atom-file (`(ParsedAtom, ValidationContext) => ValidationError[]`).
 * PROTO predicates operate over the whole atomic index. This adapter bridges
 * the gap: walk the index, read+parse each atom file, run the rule, collect
 * violations.
 *
 * Used by M8f promotion of summary-min / adr-monotonic / evidence-for-decisions
 * — these PROTOs ship draft to validate the loader path; the original rules
 * remain authoritative until promoted to stable + cut over (M8f-2 follow-up).
 */

import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'

import type {
  ParsedAtom,
  Rule,
  Severity as RuleSeverity,
  ValidationContext,
  ValidationError,
  AtomicIndexEntry,
} from '../types.js'
import type {
  Predicate,
  PredicateContext,
  PredicateResult,
  PredicateViolation,
} from './types.js'

function severityToPredicateSeverity(
  s: RuleSeverity,
): PredicateViolation['severity'] {
  if (s === 'error') return 'error'
  if (s === 'warning') return 'warning'
  return 'info'
}

async function loadParsedAtom(
  filepath: string,
): Promise<ParsedAtom | null> {
  let raw: string
  try {
    raw = await readFile(filepath, 'utf8')
  } catch {
    return null
  }
  const m = raw.match(/^---\s*\n([\s\S]*?)\n---\s*(?:\n|$)/)
  if (!m) return null
  let fm: Record<string, unknown>
  try {
    fm = parseYaml(m[1]!) as Record<string, unknown>
  } catch {
    return null
  }
  const body = raw.slice(m[0].length)
  return { fm, body, source: raw, filepath }
}

/**
 * Turn a core `Rule` into a `Predicate`. The PROTO version walks the
 * full atomic index, reads each atom file, parses its frontmatter+body,
 * and runs `rule()` for each. ValidationError[] is mapped to
 * PredicateViolation[].
 */
export function ruleAdapter(
  rule: Rule,
  opts: { filter?: (entry: AtomicIndexEntry) => boolean } = {},
): Predicate {
  return async (ctx: PredicateContext): Promise<PredicateResult> => {
    // Reconstruct the Map<id, entry> shape that ValidationContext expects.
    const indexMap = new Map<string, AtomicIndexEntry>()
    for (const e of ctx.atomicIndex) indexMap.set(e.id, e)
    const validationCtx: ValidationContext = { atomicIndex: indexMap }

    const violations: PredicateViolation[] = []

    for (const entry of ctx.atomicIndex) {
      if (opts.filter && !opts.filter(entry)) continue
      const filepath = resolve(ctx.repoRoot, 'gks', entry.path)
      const atom = await loadParsedAtom(filepath)
      if (atom === null) continue

      const errs = rule(atom, validationCtx)
      for (const err of errs) {
        violations.push({
          atomId: entry.id,
          message: err.message,
          severity: severityToPredicateSeverity(err.severity),
        })
      }
    }

    const hasError = violations.some((v) => v.severity === 'error')
    return { ok: !hasError, violations }
  }
}
