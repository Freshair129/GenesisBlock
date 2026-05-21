/**
 * Pure cost-policy primitives implementing ADR--AGENT-TIER-COST-POLICY.
 *
 * - Escalation rules (canEscalate): when an adapter call fails, decide if and
 *   to which next-stronger tier we should retry.
 * - Tier caps (enforceTierCap): for an initial routing decision, check that
 *   the chosen tier is permitted given task severity and context size.
 *
 * Both functions are pure and synchronous so dispatch.ts can call them inline
 * without async overhead.
 */
const CONTEXT_SIZE_T2_THRESHOLD = 2_000_000;
const MAX_ATTEMPT_BEFORE_ESCALATE = 2;
/**
 * Per ADR §"Automatic Escalation":
 *
 *  - T1 → T2 when severity ≥ regular and we have attempts left (`attempt < 2`).
 *  - T2 → T3 only when severity === 'critical' and attempts left.
 *  - T3 never escalates further (we're already at the top tier).
 *  - `low` severity never escalates — fail fast for trivial work.
 */
export function canEscalate(from, severity, attempt) {
    if (attempt >= MAX_ATTEMPT_BEFORE_ESCALATE)
        return null;
    if (severity === 'low')
        return null;
    if (from === 'T1') {
        // 'regular' or 'critical' both escalate from T1 → T2
        return { to: 'T2' };
    }
    if (from === 'T2') {
        if (severity === 'critical')
            return { to: 'T3' };
        return null;
    }
    // from === 'T3'
    return null;
}
/**
 * Per ADR §"Tier Cap" and §"Token Budget":
 *
 *  - Context > 2M tokens: only T2 is allowed (per CONCEPT--AGENT-TIER-ROUTING
 *    long-context routing rule, and ADR token-budget clause).
 *  - T3 requires severity === 'critical'. We do NOT model "explicitly approved
 *    sessions" here — dispatch.ts is responsible for force-overrides when the
 *    caller sets `budget_hint: 'T3'`.
 *  - T2 / T1 are otherwise unrestricted.
 */
export function enforceTierCap(tier, severity, contextSize) {
    if (contextSize > CONTEXT_SIZE_T2_THRESHOLD && tier !== 'T2') {
        return {
            allowed: false,
            reason: `context_size_tokens=${contextSize} > ${CONTEXT_SIZE_T2_THRESHOLD}; only T2 permitted`,
        };
    }
    if (tier === 'T3' && severity !== 'critical') {
        return {
            allowed: false,
            reason: `T3 restricted to critical severity (got: ${severity})`,
        };
    }
    return { allowed: true };
}
