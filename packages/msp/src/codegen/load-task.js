import { readFile } from 'node:fs/promises';
import { parse as parseYaml } from 'yaml';
import { CodegenError } from './types.js';
import { parseBudget, parseScope } from '../policy/task-scope.js';
export async function loadTask(path) {
    let raw;
    try {
        raw = await readFile(path, 'utf8');
    }
    catch (err) {
        throw new CodegenError(`cannot read task ${path}`, err);
    }
    // Strip leading comment lines so YAML parser sees a pure object.
    const cleaned = raw
        .split('\n')
        .filter((l) => !l.trim().startsWith('#'))
        .join('\n');
    let parsed;
    try {
        parsed = parseYaml(cleaned);
    }
    catch (err) {
        throw new CodegenError(`task ${path}: invalid YAML`, err);
    }
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        throw new CodegenError(`task ${path}: top-level must be a YAML object`);
    }
    const obj = parsed;
    for (const f of ['id', 'parent_blueprint', 'prompt']) {
        if (typeof obj[f] !== 'string' || !obj[f].trim()) {
            throw new CodegenError(`task ${path}: missing required field '${f}'`);
        }
    }
    if (!Array.isArray(obj.acceptance) || obj.acceptance.length < 1) {
        throw new CodegenError(`task ${path}: 'acceptance' must be a non-empty array`);
    }
    if (!Array.isArray(obj.geography) || obj.geography.length < 1) {
        throw new CodegenError(`task ${path}: 'geography' must be a non-empty array`);
    }
    return {
        id: obj.id,
        parent_blueprint: obj.parent_blueprint,
        status: typeof obj.status === 'string' ? obj.status : undefined,
        prompt: obj.prompt,
        acceptance: obj.acceptance.filter((x) => typeof x === 'string'),
        geography: obj.geography.filter((x) => typeof x === 'string'),
        assignee: typeof obj.assignee === 'string' ? obj.assignee : undefined,
        created_at: typeof obj.created_at === 'string' ? obj.created_at : undefined,
        scope: parseScope(obj.scope),
        budget: parseBudget(obj.budget),
    };
}
