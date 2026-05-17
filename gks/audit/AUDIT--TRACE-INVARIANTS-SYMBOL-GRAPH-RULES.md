---
id: AUDIT--TRACE-INVARIANTS-SYMBOL-GRAPH-RULES
phase: 6
type: audit
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — Symbol-graph trace invariants + PROTO promotion to stable
tags: &a1
  - validator
  - proto
  - trace-invariants
  - symbol-graph
  - audit
crosslinks: &a2
  references:
    - PROTO--SYMBOLS-TRACE-INVARIANTS
    - BLUEPRINT--PROTO-LOADER
    - BLUEPRINT--TRACE-INVARIANTS-PREDICATE-CONTEXT
    - AUDIT--TRACE-INVARIANTS-ATOM-GRAPH-RULES
    - AUDIT--WIRE-TRACE-INVARIANTS-PROTO
phase_override: &a3
  skip_blueprint: true
  reason: Implementation surface is already covered by
    BLUEPRINT--TRACE-INVARIANTS-PREDICATE-CONTEXT (reader injection) +
    BLUEPRINT--PROTO-LOADER (predicate scaffold). Audit closes them out.
created_at: 2026-05-16T01:00:00.000+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--TRACE-INVARIANTS-SYMBOL-GRAPH-RULES
  phase: 6
  type: audit
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: AUDIT — Symbol-graph trace invariants + PROTO promotion to stable
  tags: *a1
  crosslinks: *a2
  phase_override: *a3
  created_at: 2026-05-16T01:00:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--TRACE-INVARIANTS-SYMBOL-GRAPH-RULES
    phase: 6
    type: audit
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: AUDIT — Symbol-graph trace invariants + PROTO promotion to stable
    tags: *a1
    crosslinks: *a2
    phase_override: *a3
    created_at: 2026-05-16T01:00:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      domain: audit
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — Symbol-graph trace invariants

## Scope

Closes out the trace-invariants predicate by:
1. Wiring Rule 4a (Symbol Referential Integrity) through the new `SymbolGraphReader` injected into `PredicateContext`.
2. Removing the misleading "Rule 1 / Rule 3" stub code from the predicate. Those two rules are runtime invariants the symbol-trace tooling enforces; they have no static violation case to test, and the previous stub was an unenforced no-op masquerading as enforcement.
3. Promoting `[[PROTO--SYMBOLS-TRACE-INVARIANTS]]` from `status: active` → `status: stable` so its error-severity violations gate `msp:validate --all`.

## What shipped

| File | Change |
|---|---|
| `packages/msp/src/validator/proto/types.ts` | `PredicateContext.symbolGraph` typed as the new `SymbolGraphReaderLike` interface (was the concrete class) so tests can mock without `as any` casts. `PredicateViolation.rule?: string` added so consumers filter by rule tag instead of parsing `message`. |
| `packages/msp/src/validator/proto/symbol-graph-reader.ts` | Read-only `SymbolGraphReaderLike` interface extracted; class declared to implement it. No behaviour change. |
| `packages/msp/src/validator/proto/trace-invariants.ts` | Full rewrite. Rule 2, 4a, 4b enforced with stable rule tags (`acyclic-constraint`, `symbol-ref-integrity`, `atom-ref-integrity`). Iterative DFS for cycle detection (no recursion-depth risk). Rule 1 / Rule 3 stub code removed — header comments now document them as runtime invariants. The Rule 4a downgrade filter uses the `rule` tag instead of string-prefix matching on `message`. |
| `packages/msp/test/validator/proto/trace-invariants.test.ts` | Rewritten against `SymbolGraphReaderLike` mocks with no casts. Coverage: Rule 2 (acyclic, self-loop, simple, mixed-edge, disjoint, non-tracked-edges-ignored), Rule 4b (pass, missing-target, all-keys-scanned, halt-at-50), Rule 4a (null-graph info, pass, error, unresolved-skip, >100-downgrade). |
| `gks/proto/[[PROTO--SYMBOLS-TRACE-INVARIANTS]].md` | `status: active → stable`, `severity: error` declared, body rewritten to label Rule 1 / Rule 3 as runtime invariants and document rule tags for the enforced rules. |

## Rules — what is and is not enforced

| Rule | Static | Tag | Notes |
|---|---|---|---|
| 1. Termination Guard | no | — | Symbol-trace tool maintains visited-set + depth cap; no static violation case exists. |
| 2. Acyclic Constraint | yes (error) | `acyclic-constraint` | First cycle per component. |
| 3. Entry Point Origin | no | — | Runtime check at the trace tool's call site. |
| 4a. Symbol Ref Integrity | yes (error → warning if >100) | `symbol-ref-integrity` | Skipped when DB absent. |
| 4b. Atom Ref Integrity | yes (error) | `atom-ref-integrity` | Halts after 50 violations. |

## Verification

- `npm run msp:validate` — 325 atoms passed; `[[PROTO--SYMBOLS-TRACE-INVARIANTS]]` loads and produces no error violations against the current vault. Symbol-graph DB absent locally → Rule 4a returns an info note.
- `npm run typecheck --workspace=packages/msp` — clean.
- `npm run test --workspace=packages/msp` — `trace-invariants.test.ts` covers all enforced rules and the graceful no-op path.

## Out of scope

- **Indexer `external: true` handling.** The indexer strips it today, so all unresolved crosslink targets surface as Rule 4b violations. If external references become a real use case, upgrade the indexer to preserve the marker and update Rule 4b to honour it.
- **Multiple cycles per SCC.** Rule 2 reports one cycle per connected component. Switching to Tarjan SCC enumeration is a follow-up if dense cycle clusters appear in practice.

## Connections
- [[BLUEPRINT--PROTO-LOADER]]
- [[BLUEPRINT--TRACE-INVARIANTS-PREDICATE-CONTEXT]]
- [[AUDIT--TRACE-INVARIANTS-ATOM-GRAPH-RULES]]
- [[AUDIT--WIRE-TRACE-INVARIANTS-PROTO]]

