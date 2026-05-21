const T1_TASK_TYPES = new Set([
    'summarize',
    'classify',
    'format',
]);
const CONTEXT_SIZE_T2_THRESHOLD = 2_000_000;
export function pick(task) {
    if (task.budget_hint === 'T3') {
        return 'T3';
    }
    const contextSize = task.context_size_tokens ?? 0;
    if (contextSize > CONTEXT_SIZE_T2_THRESHOLD) {
        return 'T2';
    }
    if (task.severity === 'critical') {
        return 'T3';
    }
    if (T1_TASK_TYPES.has(task.type)) {
        return 'T1';
    }
    return 'T2';
}
