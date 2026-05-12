---
id: AUDIT--GKS-3-6-0-PUBLISHED
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: GKS 3.6.0 published to npm — closes upstream proposal #5
tags:
  - msp
  - gks
  - audit
  - upstream
  - npm
  - migration
crosslinks: {"references":["AUDIT--GKS-UPSTREAM-PROPOSALS-FILED","AUDIT--TWO-REPO-VALIDATION","ADR--EMBEDDING-MODEL-PARITY"]}
linked_symbols: ["package.json","tsconfig.json","tsconfig.build.json","src/index.ts","src/memory.ts"]
created_at: 2026-05-07T04:55:00.000+07:00
---

# AUDIT — GKS 3.6.0 published to npm

## Scope

Records the 2026-05-07 self-publish of `@freshair129/gks@3.6.0` to npm registry, closing upstream proposal #5 / `Freshair129/GksV3#28`. Unblocks MSP's Knowledge Browser code path and removes the "aspirational" caveat from `ADR--EMBEDDING-MODEL-PARITY`.

## Pre-state (problem)

| Surface | Version |
|---|---|
| npm registry latest | 3.5.6 (no nomic embedder) |
| `Freshair129/GksV3` git HEAD | 3.6.0 (unpublished — has `createNomicEmbedder`) |
| MSP `package.json` pin | `^3.5.6` |
| MSP `node_modules` installed | 3.5.6 |
| MSP Knowledge Browser (`src/index.ts`, `src/memory.ts`) | excluded from `tsconfig*.json` build because `import { retain, recall }` could not resolve in 3.5.6 |
| `ADR--EMBEDDING-MODEL-PARITY` | had a "Status note" marking nomic claims as **aspirational pending GKS 3.6.0 publish** |

## What was done

### 1. Pre-publish verification (`G:\gks` repo, branch `claude/build-gks-v3-W8a7V`)

```bash
cd G:\gks
git status                                # clean
git log -1                                # 3ddff50 release: v3.6.0
npm run build                             # tsc clean
ls dist/src/memory/vector/embedder-nomic.js  # exists
grep createNomicEmbedder dist/src/memory/index.js  # line 589 (re-export)
grep OBSIDIAN_URL .env.example            # line 2
npm pack --dry-run                        # 196 entries
npm whoami                                # suanranger (owner)
```

### 2. Publish

```bash
cd G:\gks
npm publish --access public               # + @freshair129/gks@3.6.0
git push origin v3.6.0                    # tag pushed
```

Verification:
```bash
npm view @freshair129/gks dist-tags --no-cache
# → { latest: '3.6.0' }

git ls-remote --tags origin v3.6.0
# → 3ddff502c9f6a2f501ae70b158eb9f4addbdb018  refs/tags/v3.6.0
```

### 3. Pull into MSP (this repo, branch `claude/msp-pull-gks-3.6.0`)

```bash
cd G:\msp
# package.json: "@freshair129/gks": "^3.5.6" → "^3.6.0"
npm install
# node_modules/@freshair129/gks/package.json → version: "3.6.0"
```

### 4. Un-exclude Knowledge Browser source

Both `tsconfig.json` and `tsconfig.build.json` had:
```jsonc
"exclude": [..., "src/index.ts", "src/memory.ts"]
```

These two files import `retain`/`recall` from `@freshair129/gks/memory` and `MemoryStore`/`createRestObsidianAdapter`/`wrapObsidianWithCache` from `@freshair129/gks` — symbols that 3.5.6 didn't export but 3.6.0 does.

After 3.6.0 install + un-exclude:
- `npm run typecheck` → clean (0 errors)
- `npm run build` → succeeds (chmod +x runs on all 4 bins)
- `test/mcp/argv.test.ts` (8 tests) + `test/mcp/bin.test.ts` (2 tests) → 10/10 pass

### 5. Bookkeeping updates (this commit)

- `upstream/gks-proposals/05-publish-3.6.0.md`:
  - Title status 🔵 → 🟢
  - Added "Merged upstream" line referencing commit `3ddff50` and tag `v3.6.0`
  - **Moved file to** `upstream/gks-proposals/merged/05-publish-3.6.0.md` (per workflow in `upstream/gks-proposals/README.md`)
- `upstream/gks-proposals/README.md`:
  - Table row 05 — status badge 🔵 → 🟢, path updated to `merged/`, blocking note removed
- `gks/adr/ADR--EMBEDDING-MODEL-PARITY.md`:
  - "Status note (validated 2026-05-04)" → "Status (updated 2026-05-07): ✅ GKS 3.6.0 published"
  - The "Fallback while GKS 3.6.0 is unpublished" section is kept for traceability but is now historical
- GitHub issue `Freshair129/GksV3#28` — commented + closed with reference to this audit

## Counts

- Atoms in `gks/`: +1 (this audit)
- Upstream proposals: 5 filed → 4 in `🔵 awaiting upstream review` + **1 in `🟢 merged upstream`**
- npm package versions on registry: 3.5.6 → 3.5.6, 3.6.0 (latest)
- MSP files un-excluded from build: 2 (`src/index.ts`, `src/memory.ts`)
- LOC un-excluded: ~150 (Knowledge Browser backend)

## Implications going forward

### What unblocks

- **Knowledge Browser** can be developed/built/tested again — was paused since the merge that brought it in (`f33a3f6 feat(msp): knowledge browser web UI`)
- **MSP retrieval orchestration** (M7c) can use `createNomicEmbedder()` directly instead of the Ollama BGE-M3 fallback documented in the deleted Status note
- **EVA's possible migration** from in-tree `gks.ts` to `@freshair129/gks` package becomes feasible (separate work item; tracked under EVA roadmap)

### What's still aspirational

The 4 remaining upstream proposals (`#29`–`#32`) are unchanged — those are non-blocking improvements:
- `#32` phase 6 acceptance — workaround in `scripts/msp/propose.mjs` still active
- `#31` verify-flow flag — still hand-pointing supersede chains
- `#30` backlinks API — MSP still re-implements in `src/memory/backlinks/`
- `#29` SC parity doc — doc-only, no consumer impact

### Pattern observation

This is the first proposal that flowed end-to-end through the workflow defined in `upstream/gks-proposals/README.md`:
> 🟡 drafted → 🔵 awaiting upstream review → 🟢 merged upstream → moved to `merged/` + AUDIT atom

The other 4 proposals (`#29`–`#32`) require either upstream code review (different repo, different workflow) or substantial engineering. Proposal #5 was self-mergeable because:
1. Both repos owned by the same human (`Freshair129`)
2. The change was an npm publish (no code change in GksV3 — just a release action)
3. No reviewer gate was meaningful

## Source

User direction "3" (= "do both Phase A publish 3.6.0 AND Phase B setup workspace") on 2026-05-07. Phase A executed in this audit; Phase B (workspace) is separate work.
