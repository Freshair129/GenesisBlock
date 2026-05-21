# Contributing to GKS

GKS uses **ADR-014 Doc-to-Code Enforcement** to ensure the integrity of the knowledge graph. All contributions that add new features or modify core logic must pass the integrity gates.

> **Atom prefix taxonomy (v2.3, 2026-05-13)**: this guide uses the v2.3
> vocabulary. New prefixes available: `FRAMEWORK--`, `STACK--`, `SPEC--`,
> `COGNITIVE--`, `SAFETY--`, `MOD--`. Renames: `GUARDRAIL--` → `GUARD--`;
> `FRAME--` has been **redefined** as Block Manifest (runtime entry-point
> of a Genesis Block, contract: `SPEC--GENESIS-BLOCK-MANIFEST`). The
> prior governance/architecture meaning moved to `FRAMEWORK--`. Full
> reference: [`docs/KNOWLEDGE-TYPES.md`](./docs/KNOWLEDGE-TYPES.md).

## The Doc-to-Code Loop

0. **Phase 0: Framework** (`FRAMEWORK--slug.md`) — *optional, foundational*
    Architectural framework / governance pattern, written only when introducing a
    new top-level rule (e.g. `FRAMEWORK--MSP-ARCHITECTURE-V2`). Most features skip P0.
1. **Phase 1: Concept** (`CONCEPT--slug.md`)
    Define the "What" and "Why". Status must be `stable` before moving to Phase 2.
    A companion `COGNITIVE--` atom (mental model / lens) may live alongside the CONCEPT for psychology-shaped work.
2. **Phase 2: ADR / FEAT / SPEC / STACK / MOD** (`ADR--slug.md`, etc.)
    Define the architectural decision, feature spec, data contract, tech stack, or module boundary. Status must be `stable` before moving to Phase 3.
3. **Phase 3: Blueprint** (`BLUEPRINT--slug.md`)
    Define the technical implementation plan. Status must be `stable` before Phase 4.
4. **Phase 4-6: Implementation** (`FEAT--slug.md`)
    Code changes. The `FEAT--` atom must cite the `BLUEPRINT` it implements.
    Block Manifest atoms (`FRAME--slug.md`) are authored at P0 / alongside their member atoms.

## CI Gates

Every Pull Request runs the following checks (see `.github/workflows/gks-gates.yml`):

1. **Index Integrity**: `npm run msp:index` must not produce any changes. Commit the updated `atomic_index.jsonl` whenever you add or move atoms.
2. **Link Validation**: Every `crosslinks.*` reference in the index must resolve to an existing atom.
3. **Flow Verification**: For every `FEAT--` atom, the walker asserts that the entire chain (`FEAT → BLUEPRINT → ADR → CONCEPT`) is `stable`.

## Local Enforcement

Install the example git hooks once so your machine catches drift before the
CI does:

```bash
cp examples/drift-detection/pre-push-hook.sh   .git/hooks/pre-push
cp examples/drift-detection/hotfix-gate.sh     .git/hooks/pre-commit
chmod +x .git/hooks/pre-push .git/hooks/pre-commit
```

Run the same gates locally any time:

```bash
npm run msp:index
git diff --exit-code -- gks/00_index/atomic_index.jsonl
npx tsx bin/gks.ts validate --links
npx tsx bin/gks.ts verify-flow FEAT--YOUR-FEATURE
```

> **Monorepo note (2026-05-11+)**: GKS now ships from `packages/gks/` of
> the `cognitive_system` monorepo. From the repo root, prefix per-package
> commands with the workspace selector — e.g.
> `npm run msp:index --workspace=packages/gks`. The CLI invocations
> (`bin/gks.ts ...`) work the same once you've `cd packages/gks` or used
> `npx tsx packages/gks/bin/gks.ts ...` from the root.

## Hotfixes (Escape Hatch)

If you need to land an urgent fix without writing the full chain of atoms immediately, open a **Hotfix Hatch**:

1. Tag the commit message with `HOTFIX` (or pass `--hotfix` to the gate hook), and open the atom:

    ```bash
    npx tsx bin/gks.ts hotfix open "$(git rev-parse HEAD)" \
      --title "Urgent fix" \
      --file=src/affected.ts
    ```

2. Within **48 hours**, backfill the missing atoms (`CONCEPT`, `ADR`, `BLUEPRINT`) referencing the hotfix in `crosslinks.resolves`, then close it:

    ```bash
    npx tsx bin/gks.ts hotfix close HOTFIX--<short-sha> \
      --resolved-by=ADR--BACKFILL --resolved-by=BLUEPRINT--BACKFILL
    ```

Failure to close the hotfix within 48 hours blocks any further commit on the
affected files (`gks hotfix check`) and the CI gate.
