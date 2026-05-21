const PROJECT_ONLY = {
    read_order: ['project'],
    write_target: 'project',
};
const GLOBAL_FIRST_PROJECT_FALLBACK = {
    read_order: ['global', 'project'],
    write_target: 'conflict',
};
const GLOBAL_ONLY = {
    read_order: ['global'],
    write_target: 'global',
};
const TABLE = {
    ADR: PROJECT_ONLY,
    FEAT: PROJECT_ONLY,
    BLUEPRINT: PROJECT_ONLY,
    AUDIT: PROJECT_ONLY,
    CONCEPT: PROJECT_ONLY,
    FRAMEWORK: PROJECT_ONLY,
    SPEC: PROJECT_ONLY,
    PROTOCOL: PROJECT_ONLY,
    SKILL: GLOBAL_FIRST_PROJECT_FALLBACK,
    ALGO: GLOBAL_FIRST_PROJECT_FALLBACK,
    PROTO: GLOBAL_FIRST_PROJECT_FALLBACK,
    PARAMS: GLOBAL_FIRST_PROJECT_FALLBACK,
    EPISODE: PROJECT_ONLY,
    IDENTITY: GLOBAL_ONLY,
    REGISTRY: GLOBAL_ONLY,
    MOD: PROJECT_ONLY,
    MASTER: PROJECT_ONLY,
    HOTFIX: PROJECT_ONLY,
    INC: PROJECT_ONLY,
    ISSUE: PROJECT_ONLY,
};
export function routingFor(type) {
    const rule = TABLE[type];
    if (!rule) {
        throw new Error(`routingFor: unknown atom type ${String(type)}`);
    }
    return rule;
}
export function writeTargetFor(type, vault_id) {
    const rule = routingFor(type);
    if (rule.write_target !== 'conflict') {
        return rule.write_target;
    }
    return vault_id ? 'project' : 'global';
}
