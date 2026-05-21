// Constructors
export function makeSubject(kind, id, attributes = {}) {
    return { kind, id, attributes };
}
export function makeResource(kind, id, namespace = {}, attributes = {}) {
    return { kind, id, namespace, attributes };
}
export function makeContext(origin, trace_id, overrides = {}) {
    return {
        time: overrides.time ?? new Date(),
        origin,
        trace_id,
        ...overrides,
    };
}
export function makeDecision(effect, reasoning) {
    return {
        effect,
        obligations: [],
        advice: [],
        reasoning: typeof reasoning === 'string' ? [{ description: reasoning, matched: true }] : reasoning,
    };
}
