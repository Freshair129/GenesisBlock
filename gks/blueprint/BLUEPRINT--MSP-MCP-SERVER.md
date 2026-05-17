---
id: BLUEPRINT--MSP-MCP-SERVER
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — MSP MCP server implementation plan
tags:
  - msp
  - mcp
  - server
  - blueprint
  - implementation
crosslinks:
  implements:
    - FEAT--MSP-MCP-SERVER
  references:
    - ADR--MSP-MCP-SERVER
linked_symbols:
  - file: packages/msp/src/mcp/server.ts
  - file: packages/msp/src/mcp/types.ts
  - file: packages/msp/src/mcp/tools/validate.ts
  - file: src/mcp/tools/propose.ts
  - file: packages/msp/src/mcp/tools/run-task.ts
  - file: packages/msp/src/mcp/tools/session-append.ts
  - file: packages/msp/src/mcp/tools/episode-append.ts
  - file: packages/msp/src/mcp/tools/backlinks-rebuild.ts
created_at: 2026-05-03T18:13:55.210+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  domain: blueprint
---

# BLUEPRINT — MSP MCP server

```yaml
metadata:
  title: "MSP MCP server"
  parent_feat: FEAT--MSP-MCP-SERVER

architectural_pattern: |
  One thin server module + N tool handler modules. Each handler:
    - exports a `name`, `description`, `inputSchema` (Zod)
    - exports a `handler(args, ctx)` that returns the tool result
  Server registers each handler with `server.tool()` and connects stdio.

  Per-call handlers delegate to existing TS modules. No new business
  logic in src/mcp/.

data_logic: |
  packages/msp/src/mcp/bin.ts:
    1. import { createMspMcpServer } from './server.js'
    2. const server = createMspMcpServer()
    3. const transport = new StdioServerTransport()
    4. await server.connect(transport)

  packages/msp/src/mcp/server.ts:
    function createMspMcpServer({ root? } = {}): Server
      - new Server({ name: 'msp', version: '0.1.0' }, { capabilities: { tools: {} } })
      - for each tool in [validate, propose, runTask, sessionAppend,
        episodeAppend, backlinksRebuild]:
          server.tool(tool.name, tool.description, tool.inputSchema, tool.handler(ctx))
      - return server

  Per-tool module shape (e.g. validate.ts):
    export const name = 'msp_validate'
    export const description = '...'
    export const inputSchema = z.object({...})
    export function handler(ctx) { return async (args) => {...} }

geography:
  - "packages/msp/src/mcp/types.ts"          # ToolDef, ToolHandlerCtx
  - "packages/msp/src/mcp/server.ts"         # createMspMcpServer
  - "packages/msp/src/mcp/tools/validate.ts"
  - "src/mcp/tools/propose.ts"
  - "packages/msp/src/mcp/tools/run-task.ts"
  - "packages/msp/src/mcp/tools/session-append.ts"
  - "packages/msp/src/mcp/tools/episode-append.ts"
  - "packages/msp/src/mcp/tools/backlinks-rebuild.ts"
  - "packages/msp/src/mcp/bin.ts"
  - "packages/msp/test/mcp/server.test.ts"
  - "test/mcp/tools/*.test.ts"

api_contracts:
  - name: createMspMcpServer
    signature: |
      function createMspMcpServer(opts?: { root?: string }): Server
    types: |
      // Server is from @modelcontextprotocol/sdk
      // opts.root defaults to process.env.MSP_ROOT ?? process.cwd()

  - name: ToolHandlerCtx
    types: |
      interface ToolHandlerCtx {
        root: string
      }

verification_plan:
  - vitest unit per tool: import handler, call with mock args + ctx,
    assert returned shape (no MCP plumbing required)
  - vitest server.test.ts: import createMspMcpServer, list registered
    tools via internal API, assert all 6 present with expected names
  - smoke: spawn `node dist/mcp/server.js` over stdio, send
    JSON-RPC `tools/list` + `tools/call`, assert responses
  - dogfood: msp-mcp-server validates msp's own atoms via the tool

  CI: run all of the above; no real Ollama needed (mock provider)
```

## Implementation order

T1 SERVER-SETUP   (server.ts + types.ts + bin entry)
T2 TOOL-VALIDATE  (delegates to existing validator)
T3 TOOL-PROPOSE   (shells the propose wrapper)
T4 TOOL-RUN-TASK  (delegates to existing runner with mock SLM default)
T5 TOOL-SESSION   (delegates to sessions writer)
T6 TOOL-EPISODE   (delegates to episodic writer)
T7 TOOL-BACKLINKS (delegates to backlinks indexer)
+ smoke test (spawn the bin + send JSON-RPC) + AUDIT

## Connections
- [[FEAT--MSP-MCP-SERVER]]
- [[ADR--MSP-MCP-SERVER]]

