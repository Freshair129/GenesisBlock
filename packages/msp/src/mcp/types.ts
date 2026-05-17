import type { RequestContext, Subject } from '../policy/types.js'

export interface ToolHandlerCtx {
  /** Project root — defaults to process.env.MSP_ROOT ?? cwd. */
  root: string
  /** The authenticated subject (UCF Phase 4). */
  subject?: Subject
  /** The environmental request context (UCF Phase 4). */
  policyContext?: RequestContext
}

/** Shape of a single tool result message returned over MCP. */
export interface ToolTextResult {
  content: Array<{ type: 'text'; text: string }>
  isError?: boolean
}

/** Convenience: wrap a value as a single-text MCP tool result. */
export function jsonResult(value: unknown): ToolTextResult {
  return { content: [{ type: 'text', text: JSON.stringify(value, null, 2) }] }
}

export function errorResult(message: string): ToolTextResult {
  return { content: [{ type: 'text', text: message }], isError: true }
}
