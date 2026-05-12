---
id: ADR--MSP-MCP-SERVER
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Stdio MCP server, MSP-only tools, run side-by-side with gks-mcp-server
tags:
  - msp
  - mcp
  - decision
  - server
crosslinks: {"references":["CONCEPT--MSP-MCP-SERVER","FEAT--MSP-VALIDATOR","FEAT--CODEGEN-MICROTASK-RUNNER"]}
created_at: 2026-05-03T19:13:54.288+07:00
---

# ADR — MSP MCP server shape

## Context

Three design decisions to lock down:

1. **Transport**: stdio vs HTTP/SSE.
2. **Tool surface**: MSP-only vs include passthroughs to GKS.
3. **State**: stateful (long-running session) vs stateless (one tool call = one shot).

## Decision

### (1) Stdio only

`@modelcontextprotocol/sdk` ships `StdioServerTransport`. Same shape as `gks-mcp-server`. Clients (Claude Code, Cursor) launch the server via `command + args` in their MCP config. Zero network exposure, zero auth needed. HTTP/SSE is out of scope — different deployment model.

### (2) MSP-only tools (6); never duplicate GKS

Run both servers side-by-side. The MCP client merges tool surfaces. From the agent's perspective, `gks_recall` and `msp_validate` are both available; from the operator's perspective, each server is a single grep-able file with one responsibility.

```jsonc
// ~/.config/claude/mcp.json
{
  "mcpServers": {
    "gks": {
      "command": "npx",
      "args": ["-y", "gks-mcp-server"]
    },
    "msp": {
      "command": "npx",
      "args": ["-y", "msp-mcp-server"]
    }
  }
}
```

### Tool catalog (6 tools)

| Tool | Delegates to |
|---|---|
| `msp_validate` | `validate(filepath, ctx)` from `src/validator/index.ts` |
| `msp_propose` | shells `node scripts/msp/propose.mjs` (handles phase 6) |
| `msp_run_task` | `runTask(path, opts)` from `src/codegen/runner.ts`; SLM provider via env or input |
| `msp_session_append` | `openSession(...).appendTurn(row)` from `src/memory/sessions/writer.ts` |
| `msp_episode_append` | `appendEpisode(episode, opts)` from `src/memory/episodic/writer.ts` |
| `msp_backlinks_rebuild` | `rebuildBacklinks(opts)` from `src/memory/backlinks/indexer.ts` |

Input schemas use the SDK's `z.object({...})` style with `.describe(...)` per field for tool discoverability.

### (3) Stateless

Each tool call is one shot. The codegen runner spins up + tears down per call. Session writer opens + closes per `appendTurn` — the lock is held only during the write, not for a session lifetime.

This makes the server crash-safe and trivially parallelisable. Trade-off: per-call overhead is higher than a long-lived session writer; acceptable since these tools are interactive (~once per agent turn), not hot-path.

## Consequences

**Positive**
- Server is ~200 LOC of glue + 6 small handlers.
- Same install pattern as `gks-mcp-server` — discovery is identical.
- Pluggable SLM provider via env (`MSP_SLM_PROVIDER`) — agents can override per call too.
- Stateless = no leaked file locks across crashes.

**Negative**
- Per-call session-writer open/close cost: ~5 ms file lock + write + release. Negligible at agent cadence.
- No batch tool calls — agents must loop. SDK supports parallel calls though, so 10 concurrent appends → 10 stat/lock/append cycles in parallel. Lock acquisition would serialise per-episodic-id.

## Alternatives considered

1. **Single mega-server exposing both `gks_*` + `msp_*`.** Rejected — duplicates GKS surface; couples MSP to GKS internals.
2. **HTTP/SSE transport.** Rejected for v1 — adds auth surface; not how Claude Code mounts MCP servers.
3. **Stateful session in the server (one episodicId per server lifetime).** Rejected — restrictive; multi-session agents would need multiple servers.

## What this ADR does NOT change

- GKS-MCP-server stays unchanged.
- The TS modules (`validator/`, `codegen/`, `memory/`) stay unchanged. This is wrapping, not rewriting.
- npm scripts stay (`msp:validate`, `msp:run-task`, etc.) — different surface for different consumers.

## Source

`CONCEPT--MSP-MCP-SERVER` + GksV3 README's MCP-server pattern + `@modelcontextprotocol/sdk` 1.x docs.
