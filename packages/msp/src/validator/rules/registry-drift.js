import { lookupType } from '../utils/registry.js';
export function registryDrift(atom, ctx) {
    const errors = [];
    const type = atom.fm['type'];
    if (typeof type !== 'string') {
        return errors; // caught by required-fields
    }
    const root = ctx.root ?? process.cwd();
    const typeDef = lookupType(type, root);
    if (!typeDef) {
        errors.push({
            rule: 'registry-drift',
            severity: 'error',
            message: `type '${type}' is not declared in atom_registry.yaml`,
            offending: type,
        });
        return errors;
    }
    const phase = atom.fm['phase'];
    if (typeof phase === 'number' && phase !== typeDef.phase) {
        errors.push({
            rule: 'registry-drift',
            severity: 'error',
            message: `atom phase (${phase}) drifts from registry phase (${typeDef.phase}) for type '${type}'`,
            offending: String(phase),
        });
    }
    const tier = atom.fm['tier'];
    if (typeof tier === 'string' && tier !== typeDef.tier) {
        errors.push({
            rule: 'registry-drift',
            severity: 'warning',
            message: `atom tier ('${tier}') drifts from registry tier ('${typeDef.tier}') for type '${type}'`,
            offending: tier,
        });
    }
    return errors;
}
