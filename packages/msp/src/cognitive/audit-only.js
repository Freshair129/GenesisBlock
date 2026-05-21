/**
 * §7.5 — Memory-for-Audit guardrail.
 *
 * Hits coming from episodic / session sources are marked `audit_only: true`
 * so callers know they exist for traceability and summarisation, not bulk
 * context replay. The hint is advisory — the facade does not enforce a
 * read-through ban; consumer agents are expected to honour the flag the
 * same way `epistemic.confidence` is honoured per §5.2.
 */
export function markAuditOnly(hit) {
    if (hit.source === 'episodic') {
        return { ...hit, audit_only: true };
    }
    return hit;
}
