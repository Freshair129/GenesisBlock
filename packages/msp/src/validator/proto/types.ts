/**
 * Types for the PROTO loader (M8a foundation).
 *
 * A PROTO-- atom (gks/proto/PROTO--*.md) declares a governance rule with:
 *   - `crosslinks.enforces: [FRAME--*]` — the FRAME this rule mechanises
 *   - `linked_symbols[0].file` — path to the predicate impl in src/validator/proto/
 *   - `severity: 'error' | 'warning' | 'info'` (optional; default 'warning')
 *   - `status: 'draft' | 'stable' | 'superseded'`
 *
 * The loader discovers PROTO atoms, dynamically imports their predicates,
 * runs them with a shared PredicateContext, and surfaces the results.
 *
 * Stable + severity:'error' violations cause `msp:validate --all` to exit 1.
 * Draft PROTOs run but never fail-exit (gradual rollout).
 */

import type { AtomicIndexEntry } from '../types.js'
import type { SymbolGraphReaderLike } from './symbol-graph-reader.js'

export type Severity = 'error' | 'warning' | 'info'

export type ProtoStatus = 'draft' | 'stable' | 'active' | 'superseded'

/** Discovered metadata about a PROTO atom. */
export interface ProtoMeta {
  /** Atom id, e.g. 'PROTO--SAMPLE-RULE'. */
  id: string
  /** Atom status. Loader skips 'superseded'; 'draft' runs but doesn't fail-exit. */
  status: ProtoStatus
  /** From frontmatter `severity:` field; defaults to 'warning'. */
  severity: Severity
  /** From `crosslinks.enforces` — the FRAME(s) this PROTO mechanises. */
  enforces: string[]
  /** Repo-relative path to the predicate impl (from `linked_symbols[0].file`). */
  implPath: string
  /** Atom file path on disk (for diagnostics). */
  filepath: string
}

/** Context passed to every predicate invocation. */
export interface PredicateContext {
  /** Loaded atomic index — predicates inspect atoms via this. */
  atomicIndex: AtomicIndexEntry[]
  /** Repo root, useful for predicates that need to read other files. */
  repoRoot: string
  /** Open handle to the project's symbol-graph DB, or null/undefined if
   *  unavailable in this validator run (e.g. fresh checkout without an
   *  indexed graph, or a unit test that doesn't exercise symbol rules).
   *  Predicates that need it must gracefully no-op when falsy. */
  symbolGraph?: SymbolGraphReaderLike | null
}

/** A single rule violation reported by a predicate. */
export interface PredicateViolation {
  /** Optional: the atom id that violated (when applicable). */
  atomId?: string
  /** Human-readable description. */
  message: string
  /** Severity of THIS violation; usually matches the PROTO's declared severity. */
  severity: Severity
  /** Optional stable tag identifying the sub-rule that produced the
   *  violation. Predicates enforcing multiple sub-rules mark each violation
   *  so consumers can filter without parsing `message`. */
  rule?: string
}

/** Return shape from any predicate. */
export interface PredicateResult {
  ok: boolean
  violations: PredicateViolation[]
}

/** Predicate function signature. Pure: same input → same output (mostly). */
export type Predicate = (
  ctx: PredicateContext,
) => PredicateResult | Promise<PredicateResult>

/** One row of the loader's output. */
export interface ProtoRunResult {
  meta: ProtoMeta
  result: PredicateResult
  /** Set when the predicate threw or could not be loaded. */
  loadError?: string
}

/** Aggregate summary across all PROTOs. */
export interface ProtoSummary {
  total: number
  passed: number
  failed: number
  byStatus: Record<ProtoStatus, number>
  results: ProtoRunResult[]
}
