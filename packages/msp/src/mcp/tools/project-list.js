import { readRegistry } from '../../projects/registry.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_project_list';
export const description = 'List projects registered in `~/.msp/projects.yaml`. Returns the registry contents (schemaVersion, projects map, default name). Empty registry → returns an empty `projects` object.';
export const inputSchema = {};
export function handler(ctx) {
    return async (_args) => {
        try {
            // Subject/context (UCF Phase 4) reserved; readRegistry does not
            // currently consume policy metadata.
            void ctx.subject;
            void ctx.policyContext;
            const registry = await readRegistry();
            return jsonResult({ ok: true, registry });
        }
        catch (err) {
            return errorResult(`project_list failed: ${err.message}`);
        }
    };
}
