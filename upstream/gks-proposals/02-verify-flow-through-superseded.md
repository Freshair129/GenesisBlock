# 🔵 Proposal 02 — `gks verify-flow --through-superseded` flag

**Filed upstream**: [Freshair129/GksV3#31](https://github.com/Freshair129/GksV3/issues/31) (2026-05-07)

## Why

`gks verify-flow <ID>` walks `crosslinks.references` from a starting atom to
verify the chain (FEAT → ADR → CONCEPT → FRAME). When a referenced atom has
`status: superseded`, the walker currently halts — even though the supersede
chain is intact via `crosslinks.superseded_by`.

Hit during MSP **PR #8** (M7-prep): `FRAME--MSP-ARCHITECTURE` (v1) was marked
`superseded` in favour of `FRAME--MSP-ARCHITECTURE-V2`. CI test 20 failed
because verify-flow refused to walk through the superseded v1 to reach the
shared ADRs. We worked around it by repointing 9 atoms' `crosslinks.references`
from v1 → v2, but the right fix is upstream.

## What

Add `--through-superseded` flag (default off — preserves current strict
behaviour) that follows `superseded_by` transparently when the walker hits a
superseded atom.

### File: `src/cli/verify-flow.ts` (or equivalent)

```diff
 export interface VerifyFlowOptions {
   id: string
   verbose?: boolean
+  throughSuperseded?: boolean   // follow `crosslinks.superseded_by` chain
 }

 // inside walker:
 if (atom.status === 'superseded') {
-  errors.push(`atom ${atom.id} is superseded; cannot continue chain`)
-  break
+  if (options.throughSuperseded && atom.crosslinks?.superseded_by?.length) {
+    const successor = atom.crosslinks.superseded_by[0]
+    if (visited.has(successor)) { /* cycle guard */ break }
+    visited.add(successor)
+    queue.push(successor)
+    if (options.verbose) console.log(`  → following supersede: ${atom.id} → ${successor}`)
+  } else {
+    errors.push(`atom ${atom.id} is superseded; cannot continue chain (use --through-superseded to follow)`)
+    break
+  }
 }
```

### File: `src/bin/gks.ts` argv parser

```diff
-  .option('verbose', { type: 'boolean', default: false })
+  .option('verbose', { type: 'boolean', default: false })
+  .option('through-superseded', { type: 'boolean', default: false, describe: 'follow superseded_by when walking the chain' })
```

## Compat

- **Default behaviour unchanged** — flag is opt-in. Existing CI workflows are not affected.
- **Output shape unchanged** — same `{ ok, errors, warnings }` envelope.
- **Verbose mode** logs the supersede hop so users can audit traversal.

## Test

```ts
describe('verify-flow with supersede chain', () => {
  beforeEach(async () => {
    await seed([
      { id: 'FRAME--A-V1', status: 'superseded', crosslinks: { superseded_by: ['FRAME--A-V2'] } },
      { id: 'FRAME--A-V2', status: 'stable', crosslinks: { references: ['CONCEPT--X'] } },
      { id: 'CONCEPT--X', status: 'stable' },
      { id: 'FEAT--Y', crosslinks: { references: ['FRAME--A-V1'] } },
    ])
  })

  it('halts at superseded by default', async () => {
    const r = await verifyFlow({ id: 'FEAT--Y' })
    expect(r.ok).toBe(false)
    expect(r.errors[0]).toMatch(/superseded/)
  })

  it('walks through with --through-superseded', async () => {
    const r = await verifyFlow({ id: 'FEAT--Y', throughSuperseded: true })
    expect(r.ok).toBe(true)
    expect(r.visited).toContain('CONCEPT--X')
  })

  it('detects supersede cycles without infinite loop', async () => {
    await seed([{ id: 'F1', status: 'superseded', crosslinks: { superseded_by: ['F1'] } }])
    const r = await verifyFlow({ id: 'F1', throughSuperseded: true })
    expect(r.ok).toBe(false)
    expect(r.errors.find(e => /cycle/.test(e))).toBeDefined()
  })
})
```

## Compat with FEAT--MSP-VALIDATOR

MSP's pre-commit hook calls `gks verify-flow` after promote. With the flag
opt-in, MSP can selectively enable it once a project starts using supersede
chains (e.g. via `.mspconfig.json` flag).

## Atom reference

- MSP: PR #8 CI failure analysis
- Affected MSP atoms: `FRAME--MSP-ARCHITECTURE` (superseded by V2), 9 atoms repointed in `b6714cf`
- MSP roadmap: `gks/concept/CONCEPT--MSP-ROADMAP.md` §6

## Drafted

2026-05-04, M7-prep follow-up audit.
