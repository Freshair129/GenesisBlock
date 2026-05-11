import { z } from 'zod'

import { readRegistry } from '../../projects/registry.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_project_list'

export const description =
  'List projects registered in `~/.msp/projects.yaml`. Returns the registry contents (schemaVersion, projects map, default name). Empty registry → returns an empty `projects` object.'

export const inputSchema = {}

interface ProjectListArgs {
  // no args
}

export function handler(_ctx: ToolHandlerCtx) {
  return async (_args: ProjectListArgs): Promise<ToolTextResult> => {
    try {
      const registry = await readRegistry()
      return jsonResult({ ok: true, registry })
    } catch (err) {
      return errorResult(`project_list failed: ${(err as Error).message}`)
    }
  }
}
