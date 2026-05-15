import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'

import * as backlinksRebuild from './tools/backlinks-rebuild.js'
import * as brainResolve from './tools/brain-resolve.js'
import * as candidate from './tools/candidate.js'
import * as compressTool from './tools/compress.js'
import * as dispatchTool from './tools/dispatch.js'
import * as episodeAppend from './tools/episode-append.js'
import * as identityGet from './tools/identity-get.js'
import * as identitySet from './tools/identity-set.js'
import * as projectList from './tools/project-list.js'
import * as projectRegister from './tools/project-register.js'
import * as projectResolve from './tools/project-resolve.js'
import * as recallTool from './tools/recall.js'
import * as remember from './tools/remember.js'
import * as runTask from './tools/run-task.js'
import * as sessionAppend from './tools/session-append.js'
import * as symbolCommunity from './tools/symbol-community.js'
import * as symbolImpact from './tools/symbol-impact.js'
import * as symbolLookup from './tools/symbol-lookup.js'
import * as symbolNeighbors from './tools/symbol-neighbors.js'
import * as symbolSearch from './tools/symbol-search.js'
import * as symbolTrace from './tools/symbol-trace.js'
import * as validateTool from './tools/validate.js'
import * as escalateTool from './tools/escalate.js'
import type { ToolHandlerCtx } from './types.js'

export interface ServerOpts {
  /** Project root. Defaults to MSP_ROOT env var or process.cwd(). */
  root?: string
}

const TOOLS = [
  validateTool,
  brainResolve,
  dispatchTool,
  candidate,
  runTask,
  sessionAppend,
  episodeAppend,
  backlinksRebuild,
  recallTool,
  remember,
  compressTool,
  identityGet,
  identitySet,
  projectList,
  projectRegister,
  projectResolve,
  symbolLookup,
  symbolNeighbors,
  symbolImpact,
  symbolCommunity,
  symbolSearch,
  symbolTrace,
  escalateTool,
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
