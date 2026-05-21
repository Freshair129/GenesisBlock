# GKS MCP Server — CLAUDE.md

# Created At: 2026-05-14 14:00:00 +07:00 (v0.1.0-skeleton)

## What This Is

`@freshair129/gks-mcp` — Model Context Protocol server for Genesis Knowledge System.

**Consumer: AI agents only** — Claude Code, Claude Desktop, Gemini, Antigravity, and any
MCP-compatible agent runtime. Humans do not interact with this directly.

## Status

**SKELETON** — entry point exists, no tools implemented yet.
Next step: implement `gks_search` tool using `@modelcontextprotocol/sdk`.

## MCP Tools (planned)

| Tool | Description | Input |
|---|---|---|
| `gks_search` | Semantic search across atoms | `query: string, limit?: number, type?: string` |
| `gks_get` | Get atom by ID | `id: string` |
| `gks_list` | List atoms with filter | `type?: string, phase?: string` |
| `gks_validate` | Run validator, return errors | — |
| `gks_graph` | Get neighbors of atom | `id: string, depth?: number` |
| `msp_propose` | Draft a new atom candidate | `type: string, title: string, body: string` |

## Transport

Default: **stdio** (works with Claude Desktop and Claude Code `mcpServers` config)
Optional: HTTP/SSE for remote agents

## Claude Desktop Config (when ready)

```json
{
  "mcpServers": {
    "gks": {
      "command": "node",
      "args": ["C:/Users/freshair/cognitive_system/apps/mcp/dist/index.js"]
    }
  }
}
```

## Stack

- TypeScript + Node.js (ESM)
- `@modelcontextprotocol/sdk` — MCP server SDK
- Imports `@freshair129/gks` and `@freshair129/msp` directly
- No HTTP server for basic usage (stdio transport)

## Dev

```bash
npm run dev       # run via tsx
npm run build     # tsc → dist/
npm start         # run built server (for Claude Desktop testing)
```

## Rules

- All tool responses must be JSON-serializable
- Errors must return MCP error format (not throw)
- Tools must be idempotent reads — no writes except `msp_propose`
- `msp_propose` writes to `.brain/msp/candidates/` only (never directly to gks/)
