---
id: FEAT--MSP-MCP-SERVER
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: msp-mcp-server — 6 tools over stdio MCP
tags:
  - msp
  - mcp
  - server
  - user-facing
crosslinks: {"implements":["ADR--MSP-MCP-SERVER"],"references":["CONCEPT--MSP-MCP-SERVER","FEAT--MSP-VALIDATOR","FEAT--CODEGEN-MICROTASK-RUNNER","FEAT--MEMORY-SESSIONS-WRITER","FEAT--MEMORY-EPISODIC-WRITER","FEAT--MEMORY-BACKLINKS-INDEXER"]}
linked_symbols:
  - {"file":"packages/msp/src/mcp/server.ts"}
  - {"file":"packages/msp/src/mcp/tools/validate.ts"}
  - {"file":"src/mcp/tools/propose.ts"}
  - {"file":"packages/msp/src/mcp/tools/run-task.ts"}
  - {"file":"packages/msp/src/mcp/tools/session-append.ts"}
  - {"file":"packages/msp/src/mcp/tools/episode-append.ts"}
  - {"file":"packages/msp/src/mcp/tools/backlinks-rebuild.ts"}
created_at: 2026-05-03T18:13:54.737+07:00
---

# FEAT — msp-mcp-server

## User-facing behaviour

Add to MCP client config:

```jsonc
// ~/.config/claude/mcp.json
{
  "mcpServers": {
    "msp": {
      "command": "npx",
      "args": ["-y", "msp-mcp-server"]
    }
  }
}
```

Restart the MCP client. Six new tools become available alongside any others:

| Tool | Input | Output |
|---|---|---|
| `msp_validate` | `{ files?: string[]; all?: boolean; root?: string }` | `{ results: ValidationResult[]; ok: boolean }` |
| `msp_propose` | `{ id: string; title: string; body: string; phase: number; type: string; root?: string }` | `{ proposed_id: string; inbound_path: string }` |
| `msp_run_task` | `{ task_path: string; provider?: 'ollama'|'mock'; dry_run?: boolean }` | `RunResult` (typed per the runner) |
| `msp_session_append` | `{ episodic_id: string; turn: SessionTurn; root?: string }` | `{ ok: true }` |
| `msp_episode_append` | `{ episode: Episode; root?: string }` | `{ ok: true }` |
| `msp_backlinks_rebuild` | `{ root?: string; dry_run?: boolean; check?: boolean }` | `RebuildResult` |

## Acceptance criteria

- [ ] Server starts on stdio with `msp-mcp-server` (no args required)
- [ ] `tools/list` MCP request returns the 6 tools with name, description, inputSchema (Zod-derived JSON Schema)
- [ ] `tools/call` for `msp_validate` with `{ all: true }` runs the validator and returns results
- [ ] `tools/call` for `msp_validate` with `{ files: [...] }` validates only the listed files
- [ ] `tools/call` for `msp_propose` with `phase: 6` produces a P6 atom in inbound (uses propose wrapper)
- [ ] `tools/call` for `msp_run_task` with `{ provider: 'mock', dry_run: true }` returns a successful RunResult without calling SLM
- [ ] `tools/call` for `msp_session_append` writes a turn that re-reads correctly
- [ ] `tools/call` for `msp_episode_append` writes an episode that re-reads correctly
- [ ] `tools/call` for `msp_backlinks_rebuild` rebuilds + returns counts
- [ ] Schema-invalid input → MCP error response with field-level details (Zod messages)
- [ ] Bin entry `msp-mcp-server` works from `node_modules/.bin/`
- [ ] No `gks_*` tools are duplicated

## Surfaces

| Surface | Form |
|---|---|
| Bin | `msp-mcp-server` → `dist/mcp/server.js` |
| TS API | `createMspMcpServer()` returns a configured `Server` instance (testable) |
| Config | optional env: `MSP_ROOT` (default cwd), `MSP_SLM_PROVIDER` (default `mock` for safety) |

## Out of scope

- Re-exporting `gks_*` tools — users mount both servers.
- HTTP/SSE transport — stdio only.
- Authentication — local stdio.
- Session lifetime in the server — every call is stateless.

## Connections
- [[ADR--MSP-MCP-SERVER]]
- [[CONCEPT--MSP-MCP-SERVER]]
- [[FEAT--MSP-VALIDATOR]]
- [[FEAT--CODEGEN-MICROTASK-RUNNER]]
- [[FEAT--MEMORY-SESSIONS-WRITER]]
- [[FEAT--MEMORY-EPISODIC-WRITER]]
- [[FEAT--MEMORY-BACKLINKS-INDEXER]]

