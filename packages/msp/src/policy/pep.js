import { evaluatePolicy } from './pdp.js';
import { getPolicySet } from './loader.js';
import { logShadowDecision } from './shadow-log.js';
import { join } from 'node:path';
/**
 * Policy Enforcement Point (PEP).
 * Evaluates policy and either permits, denies, or logs (shadow).
 *
 * For UCF Phase 2/3/4:
 * - kind: 'subagent' -> Enforced (drop if denied)
 * - action: 'delete' or 'restricted' -> Enforced for all
 * - other kinds -> Shadow (log and permit)
 */
export async function enforcePolicy(resource, opts) {
    const policySet = getPolicySet();
    const decision = evaluatePolicy(opts.subject, resource, opts.action, opts.context, policySet);
    const logPath = join(opts.root, '.brain', 'msp', 'audit', 'policy-decisions.jsonl');
    await logShadowDecision({
        trace_id: opts.context.trace_id,
        subject: opts.subject,
        resource,
        action: opts.action,
        context: opts.context,
        decision,
        policy_version: policySet.version,
    }, logPath);
    // Step-up Auth check (Phase 5)
    const stepUpObligation = decision.obligations.find((o) => o.kind === 'request-step-up-auth') ||
        decision.advice.find((a) => a.kind === 'request-step-up-auth');
    const isEnforced = opts.subject.kind === 'subagent' ||
        opts.subject.kind === 'user' ||
        opts.action === 'delete' ||
        resource.attributes.classification === 'restricted';
    const permitted = isEnforced ? decision.effect === 'permit' : true;
    if (isEnforced && decision.effect === 'deny') {
        console.warn(`[ucf] PEP: Denied access to ${resource.id} for ${opts.subject.kind} ${opts.subject.id} (Action: ${opts.action})`);
    }
    return {
        permitted,
        decision,
        requiresStepUp: !!stepUpObligation && decision.effect === 'deny',
        stepUpParams: stepUpObligation?.params
    };
}
