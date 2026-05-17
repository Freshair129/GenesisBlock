---
id: BLUEPRINT--PROTO-LOADER
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — PROTO loader implementation plan
tags: &a1
  - msp
  - proto
  - loader
  - blueprint
  - implementation
  - m8a
crosslinks: &a2
  implements:
    - FEAT--PROTO-LOADER
  references:
    - ADR--PROTO-ATOM-TYPE
    - CONCEPT--PROTO-PATTERN
linked_symbols: &a3
  - file: packages/msp/src/validator/proto/loader.ts
  - file: packages/msp/src/validator/proto/types.ts
  - file: packages/msp/src/validator/proto/sample.ts
  - file: packages/msp/src/validator/contract.ts
  - file: packages/msp/src/validator/cli.ts
  - file: packages/msp/test/validator/proto/loader.test.ts
  - file: packages/msp/test/validator/proto/sample.test.ts
created_at: 2026-05-05T16:18:00.000+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--PROTO-LOADER
  phase: 3
  type: blueprint
  scale_level: L2
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: BLUEPRINT — PROTO loader implementation plan
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-05T16:18:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--PROTO-LOADER
    phase: 3
    type: blueprint
    scale_level: L2
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: BLUEPRINT — PROTO loader implementation plan
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-05T16:18:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      domain: blueprint
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: aws_secret
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: aws_secret
  leak_risk: high
  encryption_level: none
---

# BLUEPRINT — PROTO loader

```yaml
metadata:
  title: "PROTO loader — pluggable governance rule scaffold"
  parent_feat: FEAT--PROTO-LOADER

architectural_pattern: |
  Three small modules + minimal contract-loader change + CLI extension.

    src/validator/proto/types.ts     Predicate, PredicateResult, Severity, ProtoMeta
    src/validator/proto/loader.ts    discoverProtos(rootDir) + runProtos(protos, ctx)
    src/validator/proto/sample.ts    trivial demo predicate
    src/validator/contract.ts        register 'proto' as a valid type
    src/validator/cli.ts             call runProtos after existing rules; print summary
    gks/proto/PROTO--SAMPLE-RULE.md  status: draft demo atom

  No new deps. Uses dynamic import() to load predicate modules.

data_logic: |
  src/validator/proto/types.ts
    type Severity = 'error' | 'warning' | 'info'
    interface ProtoMeta {
      id: string                       // 'PROTO--SAMPLE-RULE'
      status: 'draft' | 'stable' | 'superseded'
      severity: Severity               // from atom frontmatter `severity:` field
      enforces: string[]               // from crosslinks.enforces
      implPath: string                 // from linked_symbols[0].file (relative to repo root)
    }
    interface PredicateContext {
      atomicIndex: AtomicIndex          // re-use existing index loader
      repoRoot: string
    }
    interface PredicateResult {
      ok: boolean
      violations: Array<{
        atomId?: string                 // optional: which atom violated
        message: string
        severity: Severity
      }>
    }
    type Predicate = (ctx: PredicateContext) => PredicateResult | Promise<PredicateResult>

  src/validator/proto/loader.ts
    async discoverProtos(repoRoot: string): Promise<ProtoMeta[]>
      list gks/proto/*.md
      parse frontmatter
      for each:
        validate id matches PROTO-- pattern
        validate crosslinks.enforces non-empty
        validate linked_symbols[0].file exists in src/
        push ProtoMeta
      return sorted by id

    async runProtos(metas: ProtoMeta[], ctx: PredicateContext)
      results: Array<{ meta: ProtoMeta; result: PredicateResult }>
      for each meta:
        if meta.status !== 'stable' AND meta.status !== 'draft':
          skip (superseded)
        try:
          mod = await import(absolutePath(meta.implPath))
          predicate = mod.default ?? mod.predicate
          if typeof predicate !== 'function':
            push violation 'no default export' as severity:error
            continue
          result = await predicate(ctx)
          push { meta, result }
        catch err:
          push { meta, result: { ok: false, violations: [{ message: err.message, severity: 'error' }] } }
      return results

    summarise(results) → { passed, failed, byStatus, byAtom }

    shouldFailExit(results) → boolean
      = any meta.status === 'stable' && any violation severity === 'error'

  src/validator/proto/sample.ts
    export default function predicate(ctx: PredicateContext): PredicateResult {
      // Trivial demo: just verifies that there's at least one FRAME atom.
      // Real PROTOs would do something meaningful.
      const hasFrame = ctx.atomicIndex.some(a => a.type === 'frame')
      if (hasFrame) return { ok: true, violations: [] }
      return { ok: false, violations: [{ message: 'no FRAME atom found', severity: 'warning' }] }
    }

  src/validator/contract.ts (extend)
    add 'proto' to the validTypes list (or equivalent)
    add id pattern `^PROTO--[A-Z][A-Z0-9-]*$`
    add required-fields entry for type=proto: crosslinks.enforces, linked_symbols
    add optional `severity` frontmatter for type=proto

  src/validator/cli.ts (extend)
    after existing rule loop:
      const protos = await discoverProtos(repoRoot)
      const ctx = { atomicIndex: ..., repoRoot }
      const results = await runProtos(protos, ctx)
      print summary line: `PROTOs: P passed, Q failed`
      if shouldFailExit(results): process.exit(1)

  gks/proto/PROTO--SAMPLE-RULE.md
    frontmatter:
      id: PROTO--SAMPLE-RULE
      phase: 2
      type: proto
      status: draft                  ← important: draft so it doesn't fail CI
      severity: warning
      crosslinks: { enforces: [FRAMEWORK--MSP-ARCHITECTURE-V2] }
      linked_symbols: [{ file: 'src/validator/proto/sample.ts' }]
    body: brief explanation of what the rule does

geography:
  - "packages/msp/src/validator/proto/types.ts"
  - "packages/msp/src/validator/proto/loader.ts"
  - "packages/msp/src/validator/proto/sample.ts"
  - "packages/msp/src/validator/contract.ts"                  # ← MODIFIED
  - "packages/msp/src/validator/cli.ts"                       # ← MODIFIED
  - "gks/proto/PROTO--SAMPLE-RULE.md"            # NEW directory + atom
  - "packages/msp/test/validator/proto/loader.test.ts"        # ~9 tests
  - "packages/msp/test/validator/proto/sample.test.ts"        # ~3 tests
  - "packages/msp/test/validator/cli.test.ts"                 # ← MODIFIED for new summary line + 3 new tests

verification_plan:
  - vitest types: trivial (no logic; just type imports compile)
  - vitest loader: 9 tests
      - empty gks/proto/ → []
      - parse single PROTO atom
      - skip atoms without crosslinks.enforces
      - skip atoms without linked_symbols
      - rejects bad id pattern
      - runProtos calls predicate
      - draft status → run but don't fail-exit
      - stable status with error → fail-exit
      - missing impl file → violation
  - vitest sample: 3 tests — predicate runs, returns ok with FRAME present, returns violation without
  - vitest cli (extended): 3 tests
      - --all output now includes 'PROTOs: P passed, Q failed' summary
      - draft PROTO violation → still exits 0
      - stable PROTO with severity:error violation → exits 1

  Test count: ~470 → ~485 (+15)

implementation_order:
  T1 TYPES        src/validator/proto/types.ts
  T2 LOADER       src/validator/proto/loader.ts + 9 tests
  T3 SAMPLE       src/validator/proto/sample.ts + 3 tests + sample atom
  T4 CONTRACT     update src/validator/contract.ts (register 'proto' type + required-fields)
  T5 CLI          update src/validator/cli.ts (call loader, summary, exit code) + 3 tests
  T6 AUDIT        AUDIT--PROTO-LOADER atom

dependency_for: M8b PROTO--PHASE-GATES, M8c PROTO--SCALING-LEVEL-GATE, M8d PROTO--ALGO-PARAM-COUPLING, M8e PROTO--AUTHORITY-ENFORCEMENT, M8f audit-existing-rules-as-PROTOs
```

## Implementation notes

- **Run `npm ci`** in worktree (CLAUDE.md).
- **Dynamic import** — use `await import(absoluteUrl)` (not require) since the project is ESM.
- **Predicate path resolution** — `linked_symbols[0].file` is relative to repo root; convert to file URL for dynamic import.
- **Re-use atomicIndex loader** — `src/validator/atomic-index.ts` already loads `gks/00_index/atomic_index.jsonl`. Pass to the predicate context.
- **No new deps**.
- **[[PROTO--SAMPLE-RULE]]** ships at `status: draft` so it does NOT cause exit 1 even though it's loaded.

## Implementer: do NOT do

- Implement specific governance PROTOs (M8b–f scope)
- Add async predicate scheduling / concurrency
- Add sandboxing for predicate exec
- Modify the existing validator rules
- Add PROTO discovery via Obsidian

## Connections
- [[FEAT--PROTO-LOADER]]
- [[ADR--PROTO-ATOM-TYPE]]
- [[CONCEPT--PROTO-PATTERN]]

