---
id: AUDIT--PHASE-F1-MASTER-GENESIS-WIRING
phase: 6
type: audit
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — Phase F1 Master ↔ Genesis runtime wiring (registry + apply +
  executor flag)
tags: &a1
  - msp
  - master
  - promotion
  - registry
  - runtime
  - audit
  - phase-f1
crosslinks: &a2
  references:
    - CONCEPT--PROMOTED-BLOCK-REGISTRY
    - BLUEPRINT--MASTER-RUNTIME-INTEGRATION
    - CONCEPT--MASTER-PROMOTION
    - CONCEPT--GENESIS-BLOCK-RUNTIME
    - BLUEPRINT--MASTER-PROMOTION-PIPELINE
    - BLUEPRINT--GENESIS-BLOCK-RUNTIME
    - SPEC--GENESIS-BLOCK-MANIFEST
    - ADR--MASTER-PROMOTION-DOC-TO-CODE
linked_symbols: &a3
  - file: packages/msp/src/master/registry.ts
  - file: packages/msp/src/master/promote-apply.ts
  - file: packages/msp/src/master/cli.ts
  - file: packages/msp/src/genesis/executor.ts
  - file: packages/msp/src/genesis/types.ts
  - file: packages/msp/test/master/registry.test.ts
  - file: packages/msp/test/master/promote-apply.test.ts
  - file: packages/msp/test/genesis/executor.test.ts
  - file: .gitignore
created_at: 2026-05-14T05:08:00.000+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--PHASE-F1-MASTER-GENESIS-WIRING
  phase: 6
  type: audit
  status: draft
  tier: process
  source_type: axiomatic
  vault_id: default
  title: AUDIT — Phase F1 Master ↔ Genesis runtime wiring (registry + apply +
    executor flag)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-14T05:08:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--PHASE-F1-MASTER-GENESIS-WIRING
    phase: 6
    type: audit
    status: draft
    tier: process
    source_type: axiomatic
    vault_id: default
    title: AUDIT — Phase F1 Master ↔ Genesis runtime wiring (registry + apply +
      executor flag)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-14T05:08:00.000+07:00
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

# AUDIT — Phase F1 Master ↔ Genesis runtime wiring

## Scope

Phase F1 of the agentic monorepo roadmap. Closes the gap between Phase
E4 (`msp-master-propose`) and Phase E5 (`msp-genesis-exec`): a Master
block that's been promoted is now *first-class* to the executor. The
runtime gains an O(1) lookup into a derived registry, and the result
carries a `from_master` flag so callers can branch behaviour.

## What shipped

### Atoms (3)
- `gks/concept/[[CONCEPT--PROMOTED-BLOCK-REGISTRY]].md` — what + why; positions
  the registry as derived state over the canonical `gks/master/*.md` atoms
  and explains why it's gitignored.
- `gks/blueprint/[[BLUEPRINT--MASTER-RUNTIME-INTEGRATION]].md` — implementation
  plan: registry API, `applyPromotion`, executor integration, CLI subcommand,
  test coverage targets.
- `gks/audit/[[AUDIT--PHASE-F1-MASTER-GENESIS-WIRING]].md` — this atom.

### Code (4 files — 2 new, 2 extended)
- **New** `packages/msp/src/master/registry.ts` — `MasterEntry`,
  `readRegistry`, `appendRegistry`, `findActiveMaster`. Pure I/O over
  `<root>/gks/master/registry.jsonl`. Tolerates missing file + malformed
  lines.
- **New** `packages/msp/src/master/promote-apply.ts` — `applyPromotion`,
  the human-triggered "yes, promote" action. Reads a proposal from
  `gks/inbound/`, stamps `tier: master` + fresh `promoted_at`, writes
  `gks/master/MASTER--<id>.md`, appends the registry entry, renames the
  consumed proposal to `<original>.applied`.
- **Extended** `packages/msp/src/master/cli.ts` — added the `apply <path>`
  subcommand. Dispatches on the first positional; the existing flag-based
  propose flow is unchanged.
- **Extended** `packages/msp/src/genesis/executor.ts` — calls
  `findActiveMaster` between `loadMembers` and `dispatch`. On hit, biases
  the default tier to `'T2'` (overridable via `opts.tier`) and surfaces
  `from_master: true` in the `ExecuteResult`.

### Type changes
- `ExecuteResult.from_master?: boolean` — new optional field. Set to
  `true` when the registry contains an active entry for the block at
  execution time. Absent / undefined when not.

### Infra
- `.gitignore` entry: `gks/master/registry.jsonl` (per
  `[[CONCEPT--PROMOTED-BLOCK-REGISTRY]]` — derived state).

### Tests
- **New** `packages/msp/test/master/registry.test.ts` — 13 cases covering
  read/append/find round-trips, malformed-line tolerance, archived-only
  blocks, last-write-wins on multi-active.
- **New** `packages/msp/test/master/promote-apply.test.ts` — 7 cases:
  full apply round-trip, tier stamping, promoted_at refresh, target-exists
  refusal, malformed-frontmatter refusal, missing-file refusal, body
  preservation.
- **Extended** `packages/msp/test/genesis/executor.test.ts` — added the
  registry mock; new cases for `from_master: true` + tier default + explicit
  override. Existing 6 cases still pass with the registry mock returning
  null.

## Behaviour after F1

| Scenario | Pre-F1 | Post-F1 |
|---|---|---|
| Run `executeBlock('FOO', opts)` with no registry entry | `from_master` absent; dispatcher routes via `pick()` | identical |
| Run `executeBlock('FOO', opts)` with active registry entry | n/a | `from_master: true`; `budget_hint: 'T2'` (unless `opts.tier` set) |
| Run `msp-master-propose --write` | writes `.proposal.md` to `gks/inbound/` | identical |
| Run `msp-master-propose apply <path>` | n/a | moves proposal → `gks/master/`, appends registry, renames proposal `.applied` |
| `gks/master/registry.jsonl` | n/a | gitignored derived state |

## Boundaries respected

- Stayed inside `packages/msp/src/master/` + `packages/msp/src/genesis/executor.ts`
  + the 3 listed test files + `.gitignore` + the 3 atoms.
- No changes to `DispatchTask`, `DispatchResult`, `BrainQuery`, or any
  other public-API surface beyond the one optional field on `ExecuteResult`.
- Did not touch `packages/msp/src/usage/` (Phase F2 territory) or episode
  GC code paths (Phase F4 territory).

## Follow-ups (not in this PR)

1. `archived` status transitions — when a Master is superseded, append an
   `archived` row instead of needing a manual edit. Needs a small CLI
   surface (`msp-master-propose archive <blockId>`).
2. Registry rebuild — walk `gks/master/*.md`, re-derive
   `registry.jsonl` from scratch. Useful after manual edits or for
   first-clone bootstrap (currently the user must `apply` each Master).
3. Cross-Master contradiction detection at apply time — currently
   apply only refuses to overwrite a file at the same path. Two
   Masters with overlapping scope under different ids still both get
   registered. Belongs in `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]` L1+.
4. CI automation: a GitHub Action that detects a freshly-merged
   `MASTER--*.md` and auto-appends the registry entry, removing the
   manual `apply` step on the maintainer's machine.

## References

- `[[CONCEPT--PROMOTED-BLOCK-REGISTRY]]` — what + why
- `[[BLUEPRINT--MASTER-RUNTIME-INTEGRATION]]` — plan
- `[[BLUEPRINT--MASTER-PROMOTION-PIPELINE]]` — Phase E4
- `[[BLUEPRINT--GENESIS-BLOCK-RUNTIME]]` — Phase E5
- `[[SPEC--GENESIS-BLOCK-MANIFEST]]` § 3.1 + § 5
- `[[ADR--MASTER-PROMOTION-DOC-TO-CODE]]` — human gate

## Connections
- [[CONCEPT--MASTER-PROMOTION]]
- [[CONCEPT--GENESIS-BLOCK-RUNTIME]]

