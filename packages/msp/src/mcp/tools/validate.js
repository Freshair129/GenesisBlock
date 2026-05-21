import { resolve } from 'node:path';
import { z } from 'zod';
import { loadAtomicIndex, validate, validateAll } from '../../validator/index.js';
import { loadContract } from '../../validator/contract.js';
import { makeContext, makeSubject } from '../../policy/types.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_validate';
export const description = 'Run the MSP validator on one or more atom files (or the whole tree with --all). Returns per-file errors and warnings. Use this before committing or proposing atom changes.';
export const inputSchema = {
    files: z
        .array(z.string())
        .optional()
        .describe('Absolute or root-relative paths to atom .md files. Mutually exclusive with `all`.'),
    all: z.boolean().optional().describe('If true, walks gks/ recursively. Default false.'),
    root: z.string().optional().describe('Project root (default: server context root).'),
};
export function handler(ctx) {
    return async (args) => {
        const root = resolve(args.root ?? ctx.root);
        const indexPath = resolve(root, 'gks/00_index/atomic_index.jsonl');
        try {
            const subject = ctx.subject ?? makeSubject('user', 'anonymous');
            const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`);
            console.debug(`[ucf] 4-tuple: msp_validate | sub:${subject.id} | act:read | trace:${context.trace_id}`);
            let atomicIndex;
            try {
                atomicIndex = await loadAtomicIndex(indexPath);
            }
            catch (err) {
                return errorResult(`atomic index unreadable: ${err.message}`);
            }
            const contract = await loadContract(root);
            const validationCtx = {
                atomicIndex,
                forbiddenFields: contract.forbiddenFields,
                requiredFields: contract.requiredFields,
                root,
            };
            let results;
            if (args.all) {
                results = await validateAll([resolve(root, 'gks'), resolve(root, '.brain/msp/projects')], validationCtx);
            }
            else if (args.files && args.files.length > 0) {
                results = [];
                for (const f of args.files) {
                    results.push(await validate(resolve(root, f), validationCtx));
                }
            }
            else {
                return errorResult('must supply either `files` or `all: true`');
            }
            const ok = results.every((r) => r.errors.length === 0);
            return jsonResult({ ok, results });
        }
        catch (err) {
            return errorResult(`validate failed: ${err.message}`);
        }
    };
}
