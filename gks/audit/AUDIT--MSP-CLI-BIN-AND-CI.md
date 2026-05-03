---
id: AUDIT--MSP-CLI-BIN-AND-CI
phase: 6
type: audit
status: stable
vault_id: default
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
created_at: 2026-05-03T09:20:06.903Z
---

# AUDIT — M4a bin entries + CI

## Scope

L1 infrastructure work — no FEAT chain ceremony per `FRAME--SCALING-LEVELS`. Closes P0 items #3 and #4 from the production-readiness TODO.

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

`.github/workflows/test.yml` runs on every push to main + every PR, on Node 20 and 22:

1. `npm ci`
2. `npm run typecheck`
3. `npm run build`
4. `npm test`
5. `npm run msp:index`
6. `npm run msp:validate -- --all` (whole gks/ + inbound/)
7. `npm run msp:backlinks -- --check` (drift assertion)
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

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: build runs clean; bin executes; CI yaml syntactically valid (will run on push)
- Date: 2026-05-03
