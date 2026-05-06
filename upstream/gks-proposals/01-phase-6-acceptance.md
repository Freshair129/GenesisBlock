# 🔵 Proposal 01 — Accept `phase: 6` in `gks propose-inbound` CLI

**Filed upstream**: [Freshair129/GksV3#32](https://github.com/Freshair129/GksV3/issues/32) (2026-05-07)

## Why

MSP defines **Phase 6** = "post-implementation audit" (see `gks/audit/AUDIT--*` and `msp_spec.md` §3). It's the layer that records what actually happened after a FEAT shipped — distinct from Phase 5 (action / build).

Currently `gks propose-inbound` (in `Freshair129/GksV3`) rejects atoms with `phase: 6`:

```
src/memory/inbound.ts:
  throw new Error(`invalid phase ${phase}, must be integer 0..5`)
```

MSP works around this by submitting Phase 6 atoms via a custom script
(`scripts/msp/propose.mjs`) that bypasses GKS's CLI and writes inbound
files directly. Functional, but loses GKS's validation pipeline + audit log
entry on propose.

## What

Relax the phase range check from `0..5` to `0..6`.

### File: `src/memory/inbound.ts`

```diff
-    if (!Number.isInteger(phase) || phase < 0 || phase > 5) {
-      throw new Error(`invalid phase ${phase}, must be integer 0..5`)
+    if (!Number.isInteger(phase) || phase < 0 || phase > 6) {
+      throw new Error(`invalid phase ${phase}, must be integer 0..6`)
     }
```

Optionally update the JSDoc / typedef to reflect the new range.

### File: `src/types.ts` (if Phase enum exists)

```diff
-export type Phase = 0 | 1 | 2 | 3 | 4 | 5
+export type Phase = 0 | 1 | 2 | 3 | 4 | 5 | 6
```

### File: `docs/SCOPE.md` or equivalent

Optional one-line note that `phase: 6` is for post-implementation audit /
observability artefacts — but GKS doesn't enforce semantics, only stores.

## Compat

- **Existing atoms** (phase 0-5): unaffected.
- **Existing CLI users**: passing `--phase 6` previously errored; now succeeds. No downstream breakage.
- **Persistence schema**: phase is already stored as an integer; no migration.

## Test

Add to `tests/memory/inbound.test.ts`:

```ts
it('accepts phase: 6 (post-implementation audit)', async () => {
  const result = await store.proposeInbound({
    proposed_id: 'AUDIT--TEST',
    phase: 6,
    type: 'audit',
    content: 'test audit body',
  })
  expect(result.path).toMatch(/AUDIT--TEST/)
})

it('still rejects phase: 7 and phase: -1', async () => {
  await expect(store.proposeInbound({ ..., phase: 7 })).rejects.toThrow()
  await expect(store.proposeInbound({ ..., phase: -1 })).rejects.toThrow()
})
```

## Atom reference

- MSP: `gks/concept/CONCEPT--MSP-ROADMAP.md` §6 (upstream contributions table)
- MSP workaround removal target: `scripts/msp/propose.mjs` (the Phase 6 special case)

## Drafted

2026-05-04, M7-prep follow-up audit.
