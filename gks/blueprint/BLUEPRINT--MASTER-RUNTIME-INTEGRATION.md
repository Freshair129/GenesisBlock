---
id: BLUEPRINT--MASTER-RUNTIME-INTEGRATION
phase: 3
type: blueprint
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: BLUEPRINT — Master ↔ Genesis runtime wiring (Phase F1)
tags:
  - msp
  - master
  - promotion
  - runtime
  - registry
  - blueprint
  - phase-f1
crosslinks: {"references":["CONCEPT--PROMOTED-BLOCK-REGISTRY","CONCEPT--MASTER-PROMOTION","CONCEPT--GENESIS-BLOCK-RUNTIME","BLUEPRINT--MASTER-PROMOTION-PIPELINE","BLUEPRINT--GENESIS-BLOCK-RUNTIME","SPEC--GENESIS-BLOCK-MANIFEST","ADR--MASTER-PROMOTION-DOC-TO-CODE","ADR--HUMAN-REVIEW-GATES"]}
linked_symbols:
  - {"file":"packages/msp/src/master/registry.ts"}
  - {"file":"packages/msp/src/master/promote-apply.ts"}
  - {"file":"packages/msp/src/master/cli.ts"}
  - {"file":"packages/msp/src/genesis/executor.ts"}
  - {"file":"packages/msp/src/genesis/types.ts"}
  - {"file":"packages/msp/test/master/registry.test.ts"}
  - {"file":"packages/msp/test/master/promote-apply.test.ts"}
  - {"file":"packages/msp/test/genesis/executor.test.ts"}
  - {"file":".gitignore"}
created_at: 2026-05-14T05:05:00.000+07:00
---

# BLUEPRINT — Master ↔ Genesis runtime wiring (Phase F1)

## Goal

Wire `MASTER--*` promotion (Phase E4) and the Genesis Block runtime (Phase E5) together so a promoted Master block becomes first-class to the executor. After F1, an `executeBlock(id)` call against a block whose Master is registered:

- Reports `from_master: true` in the `ExecuteResult`.
- Biases the default tier upward (T2 minimum), reflecting "this block has been judged important enough to graduate from Genesis to Master".

Adds the human-triggered apply step (`msp-master-propose apply <proposalPath>`) that moves a proposal from `gks/inbound/` into `gks/master/` and writes the registry entry.

## Non-goals

- **Auto-applying proposals.** Per `ADR--MASTER-PROMOTION-DOC-TO-CODE` and `CONCEPT--MASTER-PROMOTION`, the human gate is preserved. `apply` is a CLI a human runs; `propose` never invokes it.
- **Public-API changes.** `ExecuteOptions`, `DispatchTask`, `DispatchResult`, `BrainQuery` and friends keep their existing shapes. Only `ExecuteResult` gains one optional field.
- **Cross-Master contradiction detection.** Out of scope (Layer 1+).
- **Token-cap validation on the applied Master.** The existing `PROTO--MASTER-TOKEN-CAP` rule already runs against `gks/master/` and will catch over-long bodies on the normal validator path; the apply step itself does not re-validate.
- **Authoring evidence ADRs.** `promotion_adr` is still a human deliverable; `apply` stamps the placeholder but does not create the ADR.

## Deliverables

### 1. `packages/msp/src/master/registry.ts`

Three functions, no class:

```ts
export interface MasterEntry {
  readonly block_id: string
  readonly promoted_at: string
  readonly promotion_pr?: string
  readonly status: 'active' | 'archived'
}

export async function readRegistry(root: string): Promise<MasterEntry[]>
export async function appendRegistry(root: string, entry: MasterEntry): Promise<void>
export async function findActiveMaster(
  root: string,
  blockId: string,
): Promise<MasterEntry | null>
```

- Registry lives at `<root>/gks/master/registry.jsonl`. One JSON object per line, no trailing comma, terminated with `\n`.
- `readRegistry` returns `[]` if the file does not exist. Malformed lines are silently skipped (the registry is derived state — a partial corruption shouldn't blow up the executor).
- `appendRegistry` creates `<root>/gks/master/` (with parents) if missing, then appends `JSON.stringify(entry) + '\n'`.
- `findActiveMaster` does a linear scan over `readRegistry()` and returns the **last** entry for `blockId` whose `status === 'active'` (last-write-wins).

### 2. `packages/msp/src/master/promote-apply.ts`

```ts
export interface ApplyResult {
  readonly master_id: string
  readonly master_path: string
}

export async function applyPromotion(
  proposalPath: string,
  root: string,
): Promise<ApplyResult>
```

Behaviour:

1. Reads `proposalPath` (typically under `<root>/gks/inbound/`).
2. Parses frontmatter; requires `id: MASTER--<NAME>` and `promoted_from: GENESIS--<NAME>`. Otherwise throws.
3. Re-stamps the frontmatter:
   - `tier: master` (added if missing; left untouched if already present and equal).
   - `promoted_at:` overwritten with the current ISO UTC timestamp (the proposal's stamp was set when `propose` ran; `apply` is the canonical promotion event).
   - `promoted_from:` preserved.
4. Writes the result to `<root>/gks/master/MASTER--<NAME>.md`. If a file already exists at that path, throws (apply is not an overwrite tool — re-promotion should go through supersession).
5. Appends a `MasterEntry` to the registry: `{ block_id: '<NAME>', promoted_at, status: 'active' }`. `promotion_pr` is omitted (the apply step doesn't know which PR will land it; the human can edit the registry by hand if they care).
6. Consumes the proposal — renames it to `<original>.applied` (preserves the audit trail rather than deleting).

Returns `{ master_id, master_path }`.

### 3. `packages/msp/src/master/cli.ts` (EXTEND)

Add a subcommand without disrupting the existing flag-based `propose` flow. New invocation:

```
msp-master-propose apply <proposalPath> [--root=<dir>]
```

Existing invocations (`msp-master-propose [--root=…] [--write]`) keep working exactly as before. The CLI dispatches on the first positional: if it's `apply`, route to the apply handler; otherwise run the existing propose flow.

Apply handler:
- Resolves the proposal path against `--root` if relative.
- Calls `applyPromotion(proposalPath, root)`.
- On success, prints `✓ applied: <master_path>` and exits 0.
- On error, prints to stderr and exits 1.

### 4. `packages/msp/src/genesis/executor.ts` (EXTEND)

Internal changes only — `executeBlock`'s signature stays identical.

1. After `loadManifest`, call `findActiveMaster(opts.root, manifest.id)`.
2. If the lookup returns a non-null entry:
   - Treat the block as "from master".
   - Compute the effective tier: if `opts.tier` is set, honour it. Otherwise default to `'T2'` (instead of letting the dispatcher pick — Masters get the safe cloud baseline).
3. Build the `DispatchTask` as before, but use the effective tier (when set) as `budget_hint`.
4. Construct the result with the optional `from_master: true` field set iff the lookup hit.

A new exported field on `ExecuteResult` (in `genesis/types.ts`):

```ts
export interface ExecuteResult {
  block_id: string
  output: string
  members_loaded: number
  tier_used: 'T1' | 'T2' | 'T3'
  duration_ms: number
  from_master?: boolean   // ← new
}
```

The field is **optional** so existing callers/serialisers stay valid. When absent (or `undefined`), the block ran as a normal Genesis Block.

### 5. `.gitignore`

Add one entry:

```
gks/master/registry.jsonl
```

The Master atom files themselves (`gks/master/*.md`) remain tracked — only the registry is derived.

### 6. Tests — `packages/msp/test/`

- `test/master/registry.test.ts` — tmpdir fixture. Cases:
  - `readRegistry` on missing file → `[]`.
  - `appendRegistry` creates parent dirs + writes one valid JSONL row.
  - Round-trip: append two entries, read back, both present in order.
  - `findActiveMaster` returns null when block has no entry.
  - `findActiveMaster` returns the entry when present + active.
  - Malformed lines in the file are skipped (mixed with valid entries).
- `test/master/promote-apply.test.ts` — tmpdir fixture. Cases:
  - Apply a valid proposal → master file appears at `gks/master/`, proposal renamed to `.applied`, registry has one entry.
  - Re-apply against an existing master path → throws.
  - Proposal missing `promoted_from:` → throws.
  - Frontmatter contains `tier: master` after apply.
- `test/genesis/executor.test.ts` — extend with:
  - Case: registry contains an active entry for the block. Expect `result.from_master === true`. Expect the dispatch task's `budget_hint === 'T2'` (default tier for Masters when no explicit `opts.tier`).
  - Existing cases still pass (no registry → `from_master` falsy / undefined).

## Acceptance

- `npm test --workspace=packages/msp -- test/master/ test/genesis/` passes.
- `npm run typecheck` passes (strict, no `any`).
- `msp-master-propose apply <some.proposal.md>` on a freshly written proposal produces a master file + registry entry.
- `msp-master-propose --root=. --write` (existing flow) still works untouched.
- Validator passes on the three new atoms.

## Out of scope (deferred to later phases)

- Cross-Master contradiction detection.
- Registry rebuild command (walk `gks/master/` and rewrite registry from scratch).
- Server-side / remote registry sync.
- Status transitions beyond `active` (e.g. `archived` when a Master is superseded).
- CI automation that runs `apply` after PR merge.

## Reference

- `CONCEPT--PROMOTED-BLOCK-REGISTRY` — what + why
- `CONCEPT--MASTER-PROMOTION` — 4-of-5 rule + human gate
- `CONCEPT--GENESIS-BLOCK-RUNTIME` — executor contract
- `BLUEPRINT--MASTER-PROMOTION-PIPELINE` — Phase E4 (propose half)
- `BLUEPRINT--GENESIS-BLOCK-RUNTIME` — Phase E5 (executor half)
- `SPEC--GENESIS-BLOCK-MANIFEST` § 3.1 + § 5
