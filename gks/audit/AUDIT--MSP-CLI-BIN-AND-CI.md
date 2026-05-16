---
id: AUDIT--MSP-CLI-BIN-AND-CI
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M4a — bin entries + GitHub Actions CI
tags:
  - msp
  - m4
  - m4a
  - audit
  - infra
  - bin
  - ci
crosslinks: {"references":["FEAT--MSP-VALIDATOR","FEAT--MEMORY-BACKLINKS-INDEXER","FEAT--CODEGEN-MICROTASK-RUNNER"]}
linked_symbols:
  - {"file":"package.json"}
  - {"file":"tsconfig.build.json"}
  - {"file":"scripts/msp/chmod-bins.mjs"}
  - {"file":".github/workflows/test.yml"}
created_at: 2026-05-03T16:20:06.903+07:00
---

# AUDIT — M4a bin entries + CI

## Scope

L1 infrastructure work — no FEAT chain ceremony per `[[FRAMEWORK--SCALING-LEVELS]]`. Closes P0 items #3 and #4 from the production-readiness TODO.

## Changes

### Bin entries (`package.json`)

```json
"bin": {
  "msp-validate":  "./dist/validator/cli.js",
  "msp-backlinks": "./dist/memory/backlinks/cli.js",
  "msp-run-task":  "./dist/codegen/cli.js",
  "msp-propose":   "./scripts/msp/propose.mjs"
}
```

Users now invoke MSP tools as `npx msp-validate ...` instead of `npm run msp:validate -- ...`. The npm scripts remain for in-repo dev convenience.

### Build pipeline

- New `tsconfig.build.json` extending `tsconfig.json` with `outDir: dist`, `rootDir: src`, `noEmit: false`, `declaration: true`.
- New `scripts/msp/chmod-bins.mjs` sets `+x` on the three CLI outputs after `tsc` (which strips file mode).
- `npm run build` → `tsc -p tsconfig.build.json && node scripts/msp/chmod-bins.mjs`.
- `prepublishOnly` chains `typecheck && test && build` so a published package is verified.

### CI workflow

`.github/workflows/test.yml` runs on every push to main + every PR, on Node 20 and 22 (`fail-fast: false`):

1. `npm ci`
2. `npm run typecheck`
3. `npm run build`
4. `npm test`
5. `npm run msp:index`
6. `npm run msp:validate -- --all` (whole gks/ + inbound/)
7. `npm run msp:backlinks` then `--check` (rebuild + determinism assertion — see Bug #1 below)
8. `npm run msp:check-links` (gks crosslink integrity)
9. `gks verify-flow` per FEAT in a loop

Any failing step fails the PR.

## Verification

```sh
npm run build
# → emits dist/, chmod +x on bin entries
./dist/validator/cli.js --help
# → prints help; runs as a real bin
```

## Bugs found during the first CI run

### Bug #1 — `msp:backlinks --check` always fails on a fresh checkout

**Symptom**: First CI run on PR #5 — Node 20 job failed in 35 s, Node 22 cancelled in cascade because `fail-fast` was at its default `true`.

**Root cause**: `.brain/msp/projects/<ns>/vector/backlinks.jsonl` is gitignored (it's a derived artifact, per `[[ADR--MEMORY-BACKLINKS-INDEXER]]`). On a fresh checkout the file doesn't exist. My `--check` semantics treated "no existing file" as "drift from empty" → exit 1, every CI run forever.

**Fix** (commit `985684b`):

1. Workflow rebuilds the file first (`npm run msp:backlinks`), then re-runs with `--check` to assert the indexer is deterministic (second pass = no-op).
2. Added `fail-fast: false` so a failing Node version doesn't cancel the other before it produces a usable signal.

**Reproduction confirmed locally**:
```sh
rm -f .brain/msp/projects/evaAI/vector/backlinks.jsonl
npm run msp:backlinks -- --check    # exit 1
# After fix:
npm run msp:backlinks                # exit 0 (creates)
npm run msp:backlinks -- --check     # exit 0 (no-op)
```

**Classification (per discussion in chat)**: not an INCIDENT (no prod impact), not a HOTFIX (no chain bypass), not worth a standalone ISSUE-- (caught + fixed in the same PR before merge). Recorded here following the precedent in `[[AUDIT--MSP-VALIDATOR]]` "Bugs found during M2 dogfood".

**Re-verification after fix**: PR #5 second CI run — Node 20 ✅, Node 22 ✅.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: build runs clean; bin executes; PR #5 CI green on Node 20 + 22 after the bug-fix commit
- Date: 2026-05-03

## Connections
- [[FEAT--MSP-VALIDATOR]]
- [[FEAT--MEMORY-BACKLINKS-INDEXER]]
- [[FEAT--CODEGEN-MICROTASK-RUNNER]]

