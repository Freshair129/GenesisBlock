---
id: AUDIT--MCP-CWD-RESOLUTION-FIX
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: MSP MCP server cwd resolution bug fix (--root argv parsing)
tags: &a1
  - msp
  - mcp
  - bugfix
  - audit
  - claude-desktop
crosslinks: &a2
  references:
    - AUDIT--MSP-MCP-SERVER
    - AUDIT--MSP-MCP-TOOL-EXPANSION
    - AUDIT--GKS-UPSTREAM-PROPOSALS-FILED
linked_symbols: &a3
  - packages/msp/src/mcp/argv.ts
  - packages/msp/src/mcp/bin.ts
  - packages/msp/src/mcp/server.ts
  - packages/msp/test/mcp/argv.test.ts
  - packages/msp/test/mcp/bin.test.ts
created_at: 2026-05-07T03:18:00.000+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--MCP-CWD-RESOLUTION-FIX
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: MSP MCP server cwd resolution bug fix (--root argv parsing)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-07T03:18:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--MCP-CWD-RESOLUTION-FIX
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: MSP MCP server cwd resolution bug fix (--root argv parsing)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-07T03:18:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      id: AUDIT--MCP-CWD-RESOLUTION-FIX
      phase: 6
      type: audit
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: MSP MCP server cwd resolution bug fix (--root argv parsing)
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-07T03:18:00.000+07:00
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
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — MCP cwd resolution fix

## Problem (surfaced 2026-05-07 via `[[AUDIT--GKS-UPSTREAM-PROPOSALS-FILED]]`)

When Claude Desktop launches the MSP MCP server with the canonical config —

```json
"command": "node",
"args": ["G:\\msp\\dist\\mcp\\bin.js", "--root=G:\\msp"]
```

— `process.cwd()` is `C:\Windows\system32` (Claude Desktop's working dir), not `G:\msp`. Calls to `msp_propose` and `msp_validate` failed with paths resolved off the wrong root:

```
msp_propose:  Cannot find module 'C:\Windows\system32\scripts\msp\propose.mjs'
msp_validate: atomic index not found at C:\Windows\system32\gks\00_index\atomic_index.jsonl
```

## Root cause

`src/mcp/bin.ts` was calling `createMspMcpServer()` with no arguments. The factory in `src/mcp/server.ts` had this fallback chain:

```ts
const root = opts.root ?? process.env.MSP_ROOT ?? process.cwd()
```

Without `opts.root`, and Claude Desktop not setting `MSP_ROOT` in env, it landed on `cwd`.

The `--root=G:\msp` flag in `process.argv` was **never parsed**. Individual tool handlers (`propose.ts`, `validate.ts`, etc.) all already read `ctx.root` correctly — the bug was purely at the wiring layer between argv and the factory.

## Fix

Two-line conceptual fix, three files in practice:

1. **`src/mcp/argv.ts`** (new) — exports `parseRootFromArgv(argv)`:
   - Handles both `--root=<path>` and `--root <path>` forms
   - Empty values treated as missing (not as empty string)
   - First occurrence wins (CLI norm)
   - Strict prefix match (`--rootless` doesn't trigger)

2. **`src/mcp/bin.ts`** — parse argv before constructing the server:

   ```ts
   const root = parseRootFromArgv(process.argv.slice(2))
   const server = createMspMcpServer({ root })
   ```

3. **`src/mcp/server.ts`** — unchanged. The existing `opts.root ?? MSP_ROOT ?? cwd` chain still works; `bin.ts` now feeds it the parsed value.

## Tests added

- **`test/mcp/argv.test.ts`** (8 unit tests) — covers all forms, edge cases (empty, trailing, mixed args, prefix collisions)
- **`test/mcp/bin.test.ts`** — added regression test `uses --root=<path> argv flag when cwd is unrelated and MSP_ROOT is unset`. Spawns the bin from `os.tmpdir()` with a clean env (no `MSP_ROOT`), passes `--root=<repo>`, calls `msp_validate {all:true}`, asserts the response is well-formed JSON with `ok: boolean` and contains no `atomic index unreadable` / `C:\Windows\system32` strings.

The bin.test.ts spawn helper was also generalised — prefers `node dist/mcp/bin.js` when a build exists (works on Windows + Linux without depending on `npx` PATH resolution), falls back to `npx tsx src/mcp/bin.ts`.

## Verification

```bash
# Unit
npx vitest run test/mcp/argv.test.ts        # 8/8 pass

# Integration (post-build)
npm run build
npx vitest run test/mcp/bin.test.ts          # 2/2 pass
                                              # — initialize + tools/list (existing)
                                              # — --root from unrelated cwd (new regression)

# Live e2e from a non-repo cwd
cd $env:TEMP
echo '{...initialize...}\n{...tools/call msp_validate...}' | node G:\msp\dist\mcp\bin.js --root=G:\msp
# → {"ok":true, "results":[161 atoms, all empty errors]}
```

## Pre-existing flakes (NOT caused by this fix)

Full `npm test` shows 11 failures / 533+ passed. All are environmental Windows flakes documented in CLAUDE.md:

- `test/identity/store.test.ts` — Linux path strings hardcoded (`/home/user/...`); Windows resolves to `G:\home\user\...`
- `test/validator/cli.test.ts`, `test/scripts/propose.test.ts` — `spawn npx ENOENT` (Windows .cmd resolution; works in CI under `npm ci`)
- `test/hooks/*.test.ts` — `spawn chmod ENOENT` (no chmod on Windows)
- `test/codegen/acceptance/vitest.test.ts` — sandbox `EPERM` symlink to `node_modules`

Confirmed by stashing this branch's changes and re-running `bin.test.ts` on master — same `npx ENOENT`.

## Side benefits

The bin.test.ts refactor (prefer dist over npx) means the existing `tools/list` regression test now runs on Windows too, after a `npm run build`. Previously only CI could exercise it.

## Counts

- Atoms in `gks/`: +1 (this audit)
- Files changed: 4 (`argv.ts` new, `bin.ts` rewritten, `bin.test.ts` extended, `argv.test.ts` new)
- LOC added: ~85 (function + 8 unit tests + 1 integration test + spawn helper refactor)
- LOC removed: ~3
- Behaviour change: end-user — none beyond bug fix; API surface — none changed

## Why no separate CONCEPT/ADR/FEAT/BLUEPRINT chain

This is a P5 bug fix on existing implementation, not a new design decision. CLAUDE.md's doc-to-code workflow says "Phase 4 (TASK) is for orchestrator handoff — usually skipped for single-developer slices" — same logic applies for one-line bug fixes. The implemented behaviour was already specified in `[[BLUEPRINT--MSP-MCP-SERVER]]` (server reads root from configured options); we just made the wiring honour it.

## Source

User direction "Fix MSP MCP cwd bug" (2026-05-07) immediately after the bug was surfaced in `[[AUDIT--GKS-UPSTREAM-PROPOSALS-FILED]]`. Claude Code session on `claude/msp-fix-mcp-cwd-bug` branch.

## Connections

- [[AUDIT--MSP-MCP-SERVER]]
- [[AUDIT--MSP-MCP-TOOL-EXPANSION]]
