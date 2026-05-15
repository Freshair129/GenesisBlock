import { type PolicySet, type Rule } from './loader.js'
import { evaluateCondition } from './operators.js'
import {
  makeDecision,
  type Action,
  type Decision,
  type RequestContext,
  type Resource,
  type Subject,
} from './types.js'

/**
 * Policy Decision Point (PDP).
 * Pure function that evaluates a request against a set of policies.
 */
export function evaluatePolicy(
  subject: Subject,
  resource: Resource,
  action: Action,
  context: RequestContext,
  policySet: PolicySet,
): Decision {
  const hits: Array<{ rule: Rule; policy_id: string }> = []
  const trace: Array<{ rule_id: string; matched: boolean; description: string }> = []

  const data = { subject, resource, action, context }

  for (const policy of policySet.policies) {
    for (const rule of policy.rules) {
      const match = matchesRequest(rule, subject, action, context)
      if (!match) {
        trace.push({
          rule_id: rule.id,
          matched: false,
          description: `Rule match failed (kind/action/origin).`,
        })
        continue
      }

      if (rule.condition) {
        const conditionMet = evaluateCondition(rule.condition, data)
        if (!conditionMet) {
          trace.push({
            rule_id: rule.id,
            matched: false,
            description: `Condition not met.`,
          })
          continue
        }
      }

      // Rule matched and condition passed!
      hits.push({ rule, policy_id: policy.id })
      trace.push({
        rule_id: rule.id,
        matched: true,
        description: `Matched with effect: ${rule.effect}`,
      })
    }
  }

  if (hits.length === 0) {
    return makeDecision('permit', [
      ...trace,
      { description: 'No rules matched. Default posture: permit.', matched: true },
    ])
  }

  // Pick the strongest/highest priority rule.
  // For now: first match wins, but we can sort by priority if added to schema.
  hits.sort((a, b) => (b.rule.priority ?? 100) - (a.rule.priority ?? 100))

  const bestMatch = hits[0]!
  return {
    effect: bestMatch.rule.effect,
    obligations: [],
    advice: [],
    reasoning: trace,
  }
}

function matchesRequest(
  rule: Rule,
  subject: Subject,
  action: Action,
  context: RequestContext,
): boolean {
  if (!rule.match) return true

  if (rule.match.subject_kind && !rule.match.subject_kind.includes(subject.kind)) {
    return false
  }

  if (rule.match.action && !rule.match.action.includes(action)) {
    return false
  }

  if (rule.match.origin && !rule.match.origin.includes(context.origin)) {
    return false
  }

  return true
}
