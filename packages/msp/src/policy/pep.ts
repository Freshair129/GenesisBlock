import { evaluatePolicy } from './pdp.js'
import { getPolicySet } from './loader.js'
import { logShadowDecision } from './shadow-log.js'
import {
  makeContext,
  makeResource,
  type Action,
  type RequestContext,
  type Resource,
  type Subject,
} from './types.js'
import { join } from 'node:path'

export interface PepOptions {
  root: string
  subject: Subject
  action: Action
  context: RequestContext
}

/**
 * Policy Enforcement Point (PEP).
 * Evaluates policy and either permits, denies, or logs (shadow).
 *
 * For UCF Phase 2:
 * - kind: 'subagent' -> Enforced (drop if denied)
 * - other kinds -> Shadow (log and permit)
 */
export async function enforcePolicy(
  resource: Resource,
  opts: PepOptions,
): Promise<{ permitted: boolean; decision: any }> {
  const policySet = getPolicySet()
  const decision = evaluatePolicy(opts.subject, resource, opts.action, opts.context, policySet)

  const logPath = join(opts.root, '.brain', 'msp', 'audit', 'policy-decisions.jsonl')
  await logShadowDecision(
    {
      trace_id: opts.context.trace_id,
      subject: opts.subject,
      resource,
      action: opts.action,
      context: opts.context,
      decision,
      policy_version: policySet.version,
    },
    logPath,
  )

  // UCF Phase 2 enforcement rule:
  // Subagents are enforced; everyone else is shadow-logged.
  const isSubagent = opts.subject.kind === 'subagent'
  const permitted = isSubagent ? decision.effect === 'permit' : true

  if (isSubagent && decision.effect === 'deny') {
    console.warn(
      `[ucf] PEP: Denied access to ${resource.id} for subagent ${opts.subject.id} (Action: ${opts.action})`,
    )
  }

  return { permitted, decision }
}
