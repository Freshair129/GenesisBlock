import { evaluateCondition } from './operators.js';
import { makeDecision, } from './types.js';
/**
 * Policy Decision Point (PDP).
 * Pure function that evaluates a request against a set of policies.
 */
export function evaluatePolicy(subject, resource, action, context, policySet) {
    const hits = [];
    const trace = [];
    const data = { subject, resource, action, context };
    for (const policy of policySet.policies) {
        for (const rule of policy.rules) {
            const match = matchesRequest(rule, subject, action, context);
            if (!match) {
                trace.push({
                    rule_id: rule.id,
                    matched: false,
                    description: `Rule match failed (kind/action/origin).`,
                });
                continue;
            }
            if (rule.condition) {
                const conditionMet = evaluateCondition(rule.condition, data);
                if (!conditionMet) {
                    trace.push({
                        rule_id: rule.id,
                        matched: false,
                        description: `Condition not met.`,
                    });
                    continue;
                }
            }
            // Rule matched and condition passed!
            hits.push({ rule, policy_id: policy.id });
            trace.push({
                rule_id: rule.id,
                matched: true,
                description: `Matched with effect: ${rule.effect}`,
            });
        }
    }
    if (hits.length === 0) {
        return makeDecision('permit', [
            ...trace,
            { description: 'No rules matched. Default posture: permit.', matched: true },
        ]);
    }
    // Pick the strongest/highest priority rule.
    // For now: first match wins, but we can sort by priority if added to schema.
    hits.sort((a, b) => (b.rule.priority ?? 100) - (a.rule.priority ?? 100));
    const bestMatch = hits[0];
    const obligations = bestMatch.rule.on_deny?.obligation ? [bestMatch.rule.on_deny.obligation] : [];
    return {
        effect: bestMatch.rule.effect,
        obligations,
        advice: [],
        reasoning: trace,
    };
}
function matchesRequest(rule, subject, action, context) {
    if (!rule.match)
        return true;
    if (rule.match.subject_kind && !rule.match.subject_kind.includes(subject.kind)) {
        return false;
    }
    if (rule.match.action && !rule.match.action.includes(action)) {
        return false;
    }
    if (rule.match.origin && !rule.match.origin.includes(context.origin)) {
        return false;
    }
    return true;
}
