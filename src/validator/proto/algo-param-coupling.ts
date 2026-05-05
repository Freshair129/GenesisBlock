/**
 * PROTO--ALGO-PARAM-COUPLING — bi-directional `tunes` ↔ `tunable_by` validator (M8d).
 *
 * Algorithms (`ALGO--*`) declare their tunable parameters via
 * `crosslinks.tunable_by: [PARAM--*, ...]`. Parameters (`PARAM--*`) declare
 * the algorithms they tune via `crosslinks.tunes: [ALGO--*, ...]`. This
 * predicate enforces two type/pairing invariants over the atomic index:
 *
 *   1. Type-pairing
 *      - Every value in `crosslinks.tunable_by` must be a `PARAM--*` id.
 *      - Every value in `crosslinks.tunes` must be an `ALGO--*` id.
 *
 *   2. Reciprocal coupling
 *      - If atom A declares `tunable_by: [B]` and atom B is present in the
 *        index, then B must declare `tunes: [..., A.id, ...]`.
 *      - Mirror: if A declares `tunes: [B]` and B is present, B must
 *        declare `tunable_by: [..., A.id, ...]`.
 *
 * Existence of referenced atoms (i.e. "does PARAM--X actually live in the
 * index?") is intentionally NOT checked here — that belongs to GKS's
 * `gks validate --links` per ADR--GRAPH-IS-GKS-DOMAIN. We only catch the
 * type/reciprocal mismatch when both ends are present in the index;
 * orphans are GKS's job.
 */

import type {
  Predicate,
  PredicateContext,
  PredicateResult,
  PredicateViolation,
} from './types.js'

const predicate: Predicate = (ctx: PredicateContext): PredicateResult => {
  const violations: PredicateViolation[] = []

  for (const atom of ctx.atomicIndex) {
    const tunableBy = atom.crosslinks?.tunable_by ?? []
    for (const target of tunableBy) {
      if (!target.startsWith('PARAM--')) {
        violations.push({
          atomId: atom.id,
          message: `${atom.id}: crosslinks.tunable_by references non-PARAM '${target}' (must be a PARAM--* id)`,
          severity: 'error',
        })
        continue
      }
      // Reciprocal check — only if the partner is in the index.
      const partner = ctx.atomicIndex.find((a) => a.id === target)
      if (partner) {
        const reciprocal = partner.crosslinks?.tunes ?? []
        if (!reciprocal.includes(atom.id)) {
          violations.push({
            atomId: atom.id,
            message: `${atom.id} declares tunable_by ${target}, but ${target} does not declare reciprocal tunes ${atom.id}`,
            severity: 'error',
          })
        }
      }
    }

    const tunes = atom.crosslinks?.tunes ?? []
    for (const target of tunes) {
      if (!target.startsWith('ALGO--')) {
        violations.push({
          atomId: atom.id,
          message: `${atom.id}: crosslinks.tunes references non-ALGO '${target}' (must be an ALGO--* id)`,
          severity: 'error',
        })
        continue
      }
      const partner = ctx.atomicIndex.find((a) => a.id === target)
      if (partner) {
        const reciprocal = partner.crosslinks?.tunable_by ?? []
        if (!reciprocal.includes(atom.id)) {
          violations.push({
            atomId: atom.id,
            message: `${atom.id} declares tunes ${target}, but ${target} does not declare reciprocal tunable_by ${atom.id}`,
            severity: 'error',
          })
        }
      }
    }
  }

  return { ok: violations.length === 0, violations }
}

export default predicate
