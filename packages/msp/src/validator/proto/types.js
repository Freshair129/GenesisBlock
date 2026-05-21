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
export {};
