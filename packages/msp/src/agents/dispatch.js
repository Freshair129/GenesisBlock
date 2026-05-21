import { estimateCost, estimateTokens } from './cost-tracker.js';
import { canEscalate, enforceTierCap } from './cost-policy.js';
import { getAdapter } from './registry.js';
import { recordEpisode } from './result-recorder.js';
import { pick } from './routing.js';
import { recordUsage } from './usage-recorder.js';
/**
 * BLUEPRINT--AGENT-DISPATCHER §"Routing flow" — public entry point.
 *
 * 1. Pick initial tier (budget_hint > pick()).
 * 2. Enforce cost-policy cap: if denied and the caller forced via budget_hint,
 *    throw; otherwise downgrade to T2.
 * 3. Healthcheck the adapter; on failure (and not already T1), fall back to
 *    the next-stronger tier (one step only).
 * 4. Run the adapter. If !ok and the cost-policy permits escalation, escalate
 *    once and re-run.
 * 5. Best-effort: record an episode atom. Never fail dispatch on record errors.
 * 6. Return the DispatchResult.
 */
const DEFAULT_DEADLINE_MS = 60_000;
export async function dispatch(task) {
    const startedAt = Date.now();
    const contextSize = task.context_size_tokens ?? 0;
    const deadlineMs = task.deadline_ms ?? DEFAULT_DEADLINE_MS;
    // --- Step 1: choose initial tier.
    const requestedTier = task.budget_hint ?? pick(task);
    // --- Step 2: enforce tier cap.
    const cap = enforceTierCap(requestedTier, task.severity, contextSize);
    let tier;
    if (cap.allowed) {
        tier = requestedTier;
    }
    else if (task.budget_hint !== undefined) {
        // Caller explicitly asked for this tier and the policy refused — surface
        // the error rather than silently downgrading away from the caller's hint.
        throw new Error(`dispatch: budget_hint=${task.budget_hint} denied by cost policy: ${cap.reason ?? 'unspecified'}`);
    }
    else {
        // Routed automatically into a tier we can't use — downgrade to T2,
        // which the ADR designates as the safe always-allowed cloud tier.
        tier = 'T2';
    }
    // --- Step 3: healthcheck + fallback (single-step, stronger direction).
    let escalatedFrom;
    let adapter = getAdapter(tier);
    if (tier !== 'T1') {
        const healthy = await safeHealthcheck(adapter);
        if (!healthy) {
            const fallback = nextStrongerTier(tier);
            if (fallback !== null) {
                escalatedFrom = tier;
                tier = fallback;
                adapter = getAdapter(tier);
            }
        }
    }
    // --- Step 4: run, with one escalation retry on failure.
    let attempt = 1;
    let runResult = await adapter.run(task.prompt, {
        timeout_ms: deadlineMs,
        capture_stderr: true,
    });
    if (!runResult.ok) {
        const escalation = canEscalate(tier, task.severity, attempt);
        if (escalation !== null) {
            // Cost-policy allows another tier; check the cap for the escalation target.
            const escalateCap = enforceTierCap(escalation.to, task.severity, contextSize);
            if (escalateCap.allowed) {
                escalatedFrom = tier;
                tier = escalation.to;
                adapter = getAdapter(tier);
                attempt += 1;
                runResult = await adapter.run(task.prompt, {
                    timeout_ms: deadlineMs,
                    capture_stderr: true,
                });
            }
        }
    }
    // --- Step 5/6: build result (with cost estimate).
    const inputTokens = estimateTokens(task.prompt);
    const outputTokens = estimateTokens(runResult.output);
    const cost_usd = estimateCost(tier, inputTokens, outputTokens);
    const result = {
        tier_used: tier,
        output: runResult.output,
        duration_ms: Date.now() - startedAt,
        cost_usd,
        ...(escalatedFrom ? { escalated_from: escalatedFrom } : {}),
    };
    // --- Best-effort episode recording.
    try {
        await recordEpisode(task, result, process.cwd());
    }
    catch (err) {
        // Log but never fail dispatch on episode-write errors.
        const msg = err instanceof Error ? err.message : String(err);
        process.stderr.write(`[dispatch] recordEpisode failed: ${msg}\n`);
    }
    // --- Best-effort usage-bucket recording.
    try {
        await recordUsage({ tier, cost_usd }, process.cwd());
    }
    catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        process.stderr.write(`[dispatch] recordUsage failed: ${msg}\n`);
    }
    return result;
}
async function safeHealthcheck(adapter) {
    try {
        return await adapter.healthcheck();
    }
    catch {
        return false;
    }
}
function nextStrongerTier(tier) {
    if (tier === 'T1')
        return 'T2';
    if (tier === 'T2')
        return 'T3';
    return null;
}
