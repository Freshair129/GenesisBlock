import { z } from 'zod'

import { registerProject } from '../../projects/registry.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_project_register'

export const description =
  'Register a new project in `~/.msp/projects.yaml`. Errors if a project with the same name already exists (no silent overwrite). After registration, the project can be resolved via CLI flag, `MSP_PROJECT` env, `.mspconfig`, or as the default.'

export const inputSchema = {
  name: z.string().min(1).describe('Project short name (e.g. "eva", "clinic"). Must be unique.'),
  path: z.string().min(1).describe('Filesystem path to the project root.'),
  embedder: z
    .string()
    .optional()
    .describe('Embedder name (e.g. nomic-embed-text-v1.5). Optional; falls back to server default.'),
  description: z
    .string()
    .optional()
    .describe('Free-form human description of the project.'),
}

interface ProjectRegisterArgs {
  name: string
  path: string
  embedder?: string
  description?: string
}

export function handler(_ctx: ToolHandlerCtx) {
  return async (args: ProjectRegisterArgs): Promise<ToolTextResult> => {
    try {
      const entry: { path: string; embedder?: string; description?: string } = {
        path: args.path,
      }
      if (args.embedder) entry.embedder = args.embedder
      if (args.description) entry.description = args.description
      await registerProject(args.name, entry)
      return jsonResult({ ok: true, registered: { name: args.name, entry } })
    } catch (err) {
      return errorResult(`project_register failed: ${(err as Error).message}`)
    }
  }
}
