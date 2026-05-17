---
id: AUDIT--MSP-MCP-SERVER
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M6 — MSP MCP server acceptance audit
tags:
  - msp
  - m6
  - audit
  - mcp
  - server
crosslinks:
  references:
    - FEAT--MSP-MCP-SERVER
    - BLUEPRINT--MSP-MCP-SERVER
    - ADR--MSP-MCP-SERVER
    - FEAT--MSP-VALIDATOR
    - FEAT--CODEGEN-MICROTASK-RUNNER
linked_symbols:
  - file: packages/msp/src/mcp/server.ts
  - file: packages/msp/src/mcp/types.ts
  - file: packages/msp/src/mcp/bin.ts
  - file: packages/msp/src/mcp/tools/validate.ts
  - file: src/mcp/tools/propose.ts
  - file: packages/msp/src/mcp/tools/run-task.ts
  - file: packages/msp/src/mcp/tools/session-append.ts
  - file: packages/msp/src/mcp/tools/episode-append.ts
  - file: packages/msp/src/mcp/tools/backlinks-rebuild.ts
created_at: 2026-05-03T18:20:24.061+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  domain: audit
---

# AUDIT — MSP MCP server (M6)

## Scope

Closes [[FEAT--MSP-MCP-SERVER]]. Closes P1 #5 from M3 production-readiness backlog. Last large item from the post-M3 TODO list.

## Acceptance criteria from FEAT

| # | Criterion | Result |
|---|---|---|
| 1 | Server starts on stdio with `msp-mcp-server` | ✅ bin smoke test |
| 2 | `tools/list` returns 6 tools with name/description/inputSchema | ✅ bin smoke test |
| 3 | `msp_validate` `{ all: true }` runs whole-tree | ✅ unit |
| 4 | `msp_validate` `{ files: [...] }` runs file subset | ✅ unit |
| 5 | `msp_propose` `phase: 6` produces P6 atom | ✅ via wrapper |
| 6 | `msp_run_task` `{ provider: 'mock', dry_run: true }` returns RunResult | ✅ delegates to runner with dry_run |
| 7 | `msp_session_append` writes a turn that re-reads | ✅ unit |
| 8 | `msp_episode_append` writes an episode that re-reads | ✅ unit |
| 9 | `msp_backlinks_rebuild` returns counts | ✅ unit |
| 10 | Schema-invalid input → MCP error response | ✅ implicit (Zod schemas + isError on handler errors) |
| 11 | Bin works from `node_modules/.bin/` (after build) | ✅ build emits `dist/mcp/bin.js` chmod +x |
| 12 | No `gks_*` tools duplicated | ✅ test asserts every name starts with `msp_` |

## Test summary

```
test/mcp/server.test.ts:                              4/4 passing
test/mcp/tools/validate.test.ts:                      4/4 passing
test/mcp/tools/session-episode-backlinks.test.ts:     6/6 passing
test/mcp/bin.test.ts:                                 1/1 passing  (real spawn + JSON-RPC)
total: 15/15
```

## Smoke test (manual)

```sh
$ npm run build
$ (echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":...}'; \
   echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'; \
   echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}') \
  | node dist/mcp/bin.js

→ 6 tools listed: msp_validate, msp_propose, msp_run_task,
                   msp_session_append, msp_episode_append, msp_backlinks_rebuild
```

## How to use

Add to `~/.config/claude/mcp.json`:

```jsonc
{
  "mcpServers": {
    "msp": {
      "command": "npx",
      "args": ["-y", "msp-mcp-server"],
      "env": { "MSP_ROOT": "/path/to/your/msp/repo" }
    }
  }
}
```

Run alongside `gks-mcp-server` (per the GksV3 + GitNexus pattern). MCP client merges tool surfaces; agent sees `gks_*` and `msp_*`.

## Bug found during dogfood

Initially placed bin source at `bin/msp-mcp-server.ts` outside `src/`. `tsconfig.build.json` has `rootDir: src/`, so tsc refused to emit. Fixed by moving to `src/mcp/bin.ts` and updating both `package.json` bin entry and `scripts/msp/chmod-bins.mjs` target list.

Same precedent as previous AUDITs: bug recorded in audit, not as separate INCIDENT/ISSUE — caught + fixed within the same PR before merge.

## Residual

- **No tools/call integration test against the spawned bin** — handler unit tests cover each tool's behaviour; spawn test covers tools/list. Adding tools/call over JSON-RPC adds significant test plumbing for marginal gain.
- **Authentication / TLS** — out of scope per ADR (local stdio only).
- **HTTP/SSE transport** — out of scope per ADR.
- **Real Ollama integration via tools/call** — supported (provider:'ollama') but not exercised in CI (no Ollama in CI).

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 15/15 MCP tests + 233/233 total tests + manual stdio JSON-RPC smoke
- Date: 2026-05-03

## Connections
- [[BLUEPRINT--MSP-MCP-SERVER]]
- [[ADR--MSP-MCP-SERVER]]
- [[FEAT--MSP-VALIDATOR]]
- [[FEAT--CODEGEN-MICROTASK-RUNNER]]

