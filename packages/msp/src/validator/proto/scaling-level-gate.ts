/**
 * PROTO--SCALING-LEVEL-GATE — structural sanity check for L1/L2/L3 chain
 * consistency on FEAT atoms (M8c).
 *
 * `FRAME--SCALING-LEVELS` defines escalating impact tiers, each with a
 * required atom set:
 *   - L1: no FEAT atom required
 *   - L2: CONCEPT + ADR + FEAT
 *   - L3: full chain incl. BLUEPRINT (and AUDIT, but AUDIT lands later)
 *
 * The full PR-time classifier (which inspects the git diff) is a CI workflow
 * concern and out of scope for the predicate domain. This PROTO instead
 * performs a *structural* check at the gks/ level: for each FEAT atom, walk
 * its `references` + `implements` crosslinks and confirm the expected paired
 * atoms exist.
 *
 * Heuristic for assessing implied level when no `level:` field is present:
 *   - Default expectation is L2 (CONCEPT + ADR linked).
 *   - If a BLUEPRINT atom links to the FEAT (or vice versa) the chain is
 *     treated as L3 and a BLUEPRINT linkage is required.
 *
 * Escape hatches:
 *   - `level_override: 'L1' | 'L2' | 'L3'` in the FEAT frontmatter forces
 *     the expectation to that level (no further checks if L1).
 *   - `level: 'L3'` (explicit tag) forces strict L3 chain.
 */

import type {
  Predicate,
  PredicateContext,
  PredicateResult,
  PredicateViolation,
} from './types.js'
import type { AtomicIndexEntry } from '../types.js'

type Level = 'L1' | 'L2' | 'L3'

function asLevel(v: unknown): Level | null {
  return v === 'L1' || v === 'L2' || v === 'L3' ? v : null
}

/** Read a possibly-extra string field off the index entry. */
function readField(entry: AtomicIndexEntry, key: string): unknown {
  return (entry as unknown as Record<string, unknown>)[key]
}

/** Collect `references` ∪ `implements` ids from a FEAT's crosslinks. */
function collectChain(entry: AtomicIndexEntry): Set<string> {
  const ids = new Set<string>()
  const xl = entry.crosslinks ?? {}
  for (const key of ['references', 'implements'] as const) {
    const arr = xl[key]
    if (Array.isArray(arr)) for (const id of arr) ids.add(id)
  }
  return ids
}

/** Does any BLUEPRINT atom in the index reference / implement this FEAT id? */
function blueprintBacklinkExists(
  index: AtomicIndexEntry[],
  featId: string,
): boolean {
  for (const a of index) {
    if (a.type !== 'blueprint') continue
    const xl = a.crosslinks ?? {}
    for (const key of ['references', 'implements'] as const) {
      const arr = xl[key]
      if (Array.isArray(arr) && arr.includes(featId)) return true
    }
  }
  return false
}

interface ChainHits {
  hasConcept: boolean
  hasAdr: boolean
  hasBlueprint: boolean
}

function classifyChain(
  index: AtomicIndexEntry[],
  feat: AtomicIndexEntry,
): ChainHits {
  const linked = collectChain(feat)
  const byId = new Map<string, AtomicIndexEntry>()
  for (const a of index) byId.set(a.id, a)

  let hasConcept = false
  let hasAdr = false
  let hasBlueprint = false

  for (const id of linked) {
    const target = byId.get(id)
    if (!target) {
      // Fallback — infer from the id prefix when the linked atom is not
      // present in the index (still useful for the structural check).
      if (id.startsWith('CONCEPT--')) hasConcept = true
      else if (id.startsWith('ADR--')) hasAdr = true
      else if (id.startsWith('BLUEPRINT--')) hasBlueprint = true
      continue
    }
    if (target.type === 'concept') hasConcept = true
    else if (target.type === 'adr') hasAdr = true
    else if (target.type === 'blueprint') hasBlueprint = true
  }

  // Also consider blueprint backlinks pointing at the FEAT.
  if (!hasBlueprint && blueprintBacklinkExists(index, feat.id)) {
    hasBlueprint = true
  }

  return { hasConcept, hasAdr, hasBlueprint }
}

function decideExpectedLevel(
  feat: AtomicIndexEntry,
  hits: ChainHits,
): Level {
  const override = asLevel(readField(feat, 'level_override'))
  if (override) return override
  const tagged = asLevel(readField(feat, 'level'))
  if (tagged) return tagged
  // No explicit level: if a BLUEPRINT is already in play, treat as L3,
  // otherwise default to L2.
  return hits.hasBlueprint ? 'L3' : 'L2'
}

const predicate: Predicate = (ctx: PredicateContext): PredicateResult => {
  const violations: PredicateViolation[] = []
  const feats = ctx.atomicIndex.filter((a) => a.type === 'feat')

  for (const feat of feats) {
    const hits = classifyChain(ctx.atomicIndex, feat)
    const expected = decideExpectedLevel(feat, hits)

    if (expected === 'L1') {
      // L1 has no chain expectation; respected.
      continue
    }

    const missing: string[] = []
    if (!hits.hasConcept) missing.push('CONCEPT')
    if (!hits.hasAdr) missing.push('ADR')
    if (expected === 'L3' && !hits.hasBlueprint) missing.push('BLUEPRINT')

    if (missing.length > 0) {
      violations.push({
        atomId: feat.id,
        message: `${feat.id} (expected ${expected}) is missing linked ${missing.join(' + ')} in crosslinks.references/implements`,
        severity: 'warning',
      })
    }
  }

  return { ok: violations.length === 0, violations }
}

export default predicate
