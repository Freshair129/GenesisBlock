import { z } from 'zod'

import { readRegistry } from '../../projects/registry.js'
import { makeContext, makeSubject } from '../../policy/types.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_project_list'

export const description =
  'List projects registered in `~/.msp/projects.yaml`. Returns the registry contents (schemaVersion, projects map, default name). Empty registry → returns an empty `projects` object.'

export const inputSchema = {}

interface ProjectListArgs {
  // no args
}

export function handler(ctx: ToolHandlerCtx) {
  return async (_args: ProjectListArgs): Promise<ToolTextResult> => {
    try {
      const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp')
      const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`)

      const registry = await readRegistry({ subject, context })
      return jsonResult({ ok: true, registry })
    } catch (err) {
      return errorResult(`project_list failed: ${(err as Error).message}`)
    }
  }
}
