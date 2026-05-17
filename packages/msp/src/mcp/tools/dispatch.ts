import { z } from 'zod'

import { dispatch as dispatchTask } from '../../agents/dispatch.js'
import type { DispatchTask, Severity, TaskType, Tier } from '../../agents/types.js'
import { makeContext, makeSubject } from '../../policy/types.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_dispatch'

export const description =
  'Run a task through the T1/T2/T3 agent-tier dispatcher (BLUEPRINT--AGENT-DISPATCHER). The dispatcher picks a tier per CONCEPT--AGENT-TIER-ROUTING, enforces the cost policy from ADR--AGENT-TIER-COST-POLICY, escalates on failure, and writes an episode atom. Use this when an upstream agent has decided that another tier (cheaper or stronger) should handle a sub-task.'

const TASK_TYPES: readonly TaskType[] = [
  'summarize',
  'classify',
  'format',
  'codegen',
  'review',
  'other',
] as const
const SEVERITIES: readonly Severity[] = ['critical', 'regular', 'low'] as const
const TIERS: readonly Tier[] = ['T1', 'T2', 'T3'] as const

const TASK_TYPE_SET = new Set<string>(TASK_TYPES)
const SEVERITY_SET = new Set<string>(SEVERITIES)
const TIER_SET = new Set<string>(TIERS)

export const inputSchema = {
  prompt: z.string().describe('The task prompt to send to the chosen tier.'),
  type: z
    .string()
    .optional()
    .describe(`Task type — one of ${TASK_TYPES.join(', ')}. Default: "other".`),
  severity: z
    .string()
    .optional()
    .describe(`Task severity — one of ${SEVERITIES.join(', ')}. Default: "regular".`),
  context_size_tokens: z
    .number()
    .int()
    .nonnegative()
    .optional()
    .describe('Approximate prompt+context token count. Routes to T2 when > 2_000_000.'),
  budget_hint: z
    .string()
    .optional()
    .describe(`Force a specific tier — one of ${TIERS.join(', ')}. Throws if denied by cost policy.`),
  deadline_ms: z
    .number()
    .int()
    .positive()
    .optional()
    .describe('Per-tier run timeout in ms. Default 60_000.'),
}

interface DispatchArgs {
  prompt: string
  type?: string
  severity?: string
  context_size_tokens?: number
  budget_hint?: string
  deadline_ms?: number
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: DispatchArgs): Promise<ToolTextResult> => {
    if (args.type !== undefined && !TASK_TYPE_SET.has(args.type)) {
      return errorResult(
        `invalid task type: "${args.type}". Must be one of: ${TASK_TYPES.join(', ')}`,
      )
    }
    if (args.severity !== undefined && !SEVERITY_SET.has(args.severity)) {
      return errorResult(
        `invalid severity: "${args.severity}". Must be one of: ${SEVERITIES.join(', ')}`,
      )
    }
    if (args.budget_hint !== undefined && !TIER_SET.has(args.budget_hint)) {
      return errorResult(
        `invalid budget_hint: "${args.budget_hint}". Must be one of: ${TIERS.join(', ')}`,
      )
    }

    const task: DispatchTask = {
      type: (args.type as TaskType | undefined) ?? 'other',
      severity: (args.severity as Severity | undefined) ?? 'regular',
      prompt: args.prompt,
      ...(args.context_size_tokens !== undefined
        ? { context_size_tokens: args.context_size_tokens }
        : {}),
      ...(args.budget_hint !== undefined ? { budget_hint: args.budget_hint as Tier } : {}),
      ...(args.deadline_ms !== undefined ? { deadline_ms: args.deadline_ms } : {}),
    }

    try {
      const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp')
      const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`)

      const result = await dispatchTask(task)
      return jsonResult({
        ok: true,
        tier_used: result.tier_used,
        output: result.output,
        duration_ms: result.duration_ms,
        ...(result.escalated_from ? { escalated_from: result.escalated_from } : {}),
        ...(result.cost_usd !== undefined ? { cost_usd: result.cost_usd } : {}),
      })
    } catch (err) {
      return errorResult(`dispatch failed: ${(err as Error).message}`)
    }
  }
}
