import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'

import * as backlinksRebuild from './tools/backlinks-rebuild.js'
import * as episodeAppend from './tools/episode-append.js'
import * as identityGet from './tools/identity-get.js'
import * as identitySet from './tools/identity-set.js'
import * as propose from './tools/propose.js'
import * as recallTool from './tools/recall.js'
import * as remember from './tools/remember.js'
import * as runTask from './tools/run-task.js'
import * as sessionAppend from './tools/session-append.js'
import * as validateTool from './tools/validate.js'
import type { ToolHandlerCtx } from './types.js'

export interface ServerOpts {
  /** Project root. Defaults to MSP_ROOT env var or process.cwd(). */
  root?: string
}

const TOOLS = [
  validateTool,
  propose,
  runTask,
  sessionAppend,
  episodeAppend,
  backlinksRebuild,
  recallTool,
  remember,
  identityGet,
  identitySet,
] as const

export function createMspMcpServer(opts: ServerOpts = {}): McpServer {
  const root = opts.root ?? process.env.MSP_ROOT ?? process.cwd()
  const ctx: ToolHandlerCtx = { root }

  const server = new McpServer({
    name: 'msp',
    version: '0.1.0',
  })

  for (const t of TOOLS) {
    server.registerTool(
      t.name,
      {
        description: t.description,
        inputSchema: t.inputSchema,
      },
      // SDK accepts our handler shape: (args) => Promise<{content,isError?}>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      t.handler(ctx) as any,
    )
  }

  return server
}

/** Names of all tools registered by createMspMcpServer (for tests). */
export const REGISTERED_TOOL_NAMES: readonly string[] = TOOLS.map((t) => t.name)
