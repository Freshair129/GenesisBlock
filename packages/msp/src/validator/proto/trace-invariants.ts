import type {
  Predicate,
  PredicateContext,
  PredicateResult,
  PredicateViolation,
} from './types.js'
import type { AtomicIndexEntry } from '../types.js'

/**
 * PROTO--SYMBOLS-TRACE-INVARIANTS validator.
 *
 * Statically enforced sub-rules:
 *   Rule 2  — Acyclic Constraint on the atom graph
 *             (edges: supersedes, implements, parent_blueprint)
 *   Rule 4b — Atom Referential Integrity
 *   Rule 4a — Symbol Referential Integrity (requires symbol graph DB)
 *
 * Deferred to runtime (not enforced here):
 *   Rule 1  — Termination Guard
 *             By construction, the symbol-trace MCP tool uses a visited-set
 *             plus depth cap, so traces cannot diverge. There is no static
 *             violation case; this rule exists as a runtime invariant the
 *             tracer must uphold.
 *   Rule 3  — Entry Point Origin
 *             Applies when an architectural tool (e.g. `symbol_trace`) is
 *             invoked at runtime; it validates the trace's first edge src
 *             belongs to a framework-entry kind. The static graph contains
 *             no "trace" entity to inspect, so this is checked at the call
 *             site of the tracer, not here.
 */

const ATOM_GRAPH_EDGES = ['supersedes', 'implements', 'parent_blueprint'] as const
const ATOM_REF_INTEGRITY_HALT = 50
const SYMBOL_REF_INTEGRITY_WARN_THRESHOLD = 100

const RULE_ACYCLIC = 'acyclic-constraint'
const RULE_ATOM_REF_INTEGRITY = 'atom-ref-integrity'
const RULE_SYMBOL_REF_INTEGRITY = 'symbol-ref-integrity'

const predicate: Predicate = async (ctx: PredicateContext): Promise<PredicateResult> => {
  const violations: PredicateViolation[] = []

  violations.push(...checkAcyclicConstraint(ctx.atomicIndex))
  violations.push(...checkAtomReferentialIntegrity(ctx.atomicIndex))

  if (!ctx.symbolGraph) {
    violations.push({
      message: 'Symbol graph DB unavailable; Rule 4a (symbol referential integrity) skipped.',
      severity: 'info',
    })
  } else {
    violations.push(...checkSymbolReferentialIntegrity(ctx.symbolGraph))
  }

  const ok = !violations.some((v) => v.severity === 'error')
  return { ok, violations }
}

/**
 * Rule 2 — Acyclic Constraint on the atom graph.
 *
 * Iterative DFS with white/gray/black coloring. Reports the first cycle
 * found per connected component (not every cycle in dense SCCs). A
 * violation lists the cycle as `A → B → C → A` with the edge type that
 * closed it.
 */
function checkAcyclicConstraint(index: AtomicIndexEntry[]): PredicateViolation[] {
  const violations: PredicateViolation[] = []
  const idMap = new Map(index.map((a) => [a.id, a]))
  const colors = new Map<string, number>() // 0 white, 1 gray, 2 black
  const parent = new Map<string, { from: string; edge: string }>()

  const dfs = (start: string): { cycle: string[]; edge: string } | null => {
    const stack: Array<{ node: string; iter: Iterator<{ to: string; edge: string }> }> = []
    colors.set(start, 1)
    stack.push({ node: start, iter: outgoingEdges(idMap.get(start)) })

    while (stack.length > 0) {
      const top = stack[stack.length - 1]!
      const next = top.iter.next()
      if (next.done) {
        colors.set(top.node, 2)
        stack.pop()
        continue
      }
      const { to, edge } = next.value
      const color = colors.get(to) ?? 0
      if (color === 1) {
        const cycle: string[] = []
        let curr = top.node
        while (curr !== to) {
          cycle.push(curr)
          const p = parent.get(curr)
          if (!p) break
          curr = p.from
        }
        cycle.push(to)
        cycle.reverse()
        cycle.push(to)
        return { cycle, edge }
      }
      if (color === 0) {
        parent.set(to, { from: top.node, edge })
        colors.set(to, 1)
        stack.push({ node: to, iter: outgoingEdges(idMap.get(to)) })
      }
    }
    return null
  }

  for (const atom of index) {
    if ((colors.get(atom.id) ?? 0) !== 0) continue
    const found = dfs(atom.id)
    if (found) {
      violations.push({
        atomId: found.cycle[0],
        message: `Cycle in atom graph (${found.edge}): ${found.cycle.join(' → ')}`,
        severity: 'error',
        rule: RULE_ACYCLIC,
      })
    }
  }

  return violations
}

function* outgoingEdges(
  atom: AtomicIndexEntry | undefined,
): Generator<{ to: string; edge: string }> {
  if (!atom?.crosslinks) return
  for (const edge of ATOM_GRAPH_EDGES) {
    const targets = atom.crosslinks[edge]
    if (!Array.isArray(targets)) continue
    for (const to of targets) yield { to, edge }
  }
}

/**
 * Rule 4b — Atom Referential Integrity.
 *
 * Every string ID in any `crosslinks.*` array must resolve to an atom in
 * the index. The indexer strips `external: true` markers today (see
 * AUDIT--TRACE-INVARIANTS-ATOM-GRAPH-RULES), so missing targets are
 * unconditionally flagged. We halt after `ATOM_REF_INTEGRITY_HALT`
 * violations to avoid log floods during vault drift.
 */
function checkAtomReferentialIntegrity(index: AtomicIndexEntry[]): PredicateViolation[] {
  const violations: PredicateViolation[] = []
  const ids = new Set(index.map((a) => a.id))

  for (const atom of index) {
    if (!atom.crosslinks) continue
    for (const [field, targets] of Object.entries(atom.crosslinks)) {
      if (!Array.isArray(targets)) continue
      for (const target of targets) {
        if (typeof target !== 'string') continue
        if (ids.has(target)) continue
        violations.push({
          atomId: atom.id,
          message: `Atom ${atom.id} references missing target ${target} via crosslinks.${field}`,
          severity: 'error',
          rule: RULE_ATOM_REF_INTEGRITY,
        })
        if (violations.length >= ATOM_REF_INTEGRITY_HALT) {
          violations.push({
            message: `Atom referential integrity halted after ${ATOM_REF_INTEGRITY_HALT} violations (likely widespread drift; fix the first batch then re-run).`,
            severity: 'error',
            rule: RULE_ATOM_REF_INTEGRITY,
          })
          return violations
        }
      }
    }
  }

  return violations
}

/**
 * Rule 4a — Symbol Referential Integrity.
 *
 * Every `resolved: true` edge in the symbol graph must point to a symbol
 * that exists in the symbols table. If the violation count exceeds
 * `SYMBOL_REF_INTEGRITY_WARN_THRESHOLD`, all 4a violations are downgraded
 * to `warning` so we surface the drift without blocking CI on graph
 * extraction lag.
 */
function checkSymbolReferentialIntegrity(
  graph: NonNullable<PredicateContext['symbolGraph']>,
): PredicateViolation[] {
  const violations: PredicateViolation[] = []
  for (const edge of graph.allEdges()) {
    if (!edge.resolved) continue
    if (graph.getSymbol(edge.dst_id)) continue
    violations.push({
      message: `Symbol edge ${edge.src_id} → ${edge.dst_id} (${edge.type}) points to a missing symbol.`,
      severity: 'error',
      rule: RULE_SYMBOL_REF_INTEGRITY,
    })
  }

  if (violations.length > SYMBOL_REF_INTEGRITY_WARN_THRESHOLD) {
    for (const v of violations) {
      if (v.rule === RULE_SYMBOL_REF_INTEGRITY) v.severity = 'warning'
    }
    violations.push({
      message: `Symbol referential integrity found ${violations.length} violations (> ${SYMBOL_REF_INTEGRITY_WARN_THRESHOLD}); severities downgraded to warning. Likely a graph extraction lag — investigate the extractor before promoting back to error.`,
      severity: 'warning',
      rule: RULE_SYMBOL_REF_INTEGRITY,
    })
  }

  return violations
}

export default predicate
