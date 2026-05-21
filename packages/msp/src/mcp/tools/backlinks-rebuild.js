import { resolve } from 'node:path';
import { z } from 'zod';
import { rebuildBacklinks } from '../../memory/backlinks/indexer.js';
import { makeContext, makeSubject } from '../../policy/types.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_backlinks_rebuild';
export const description = 'Rebuild .brain/.../vector/backlinks.jsonl from atom crosslinks. Use --check to assert no drift (CI).';
export const inputSchema = {
    root: z.string().optional().describe('Project root (default: server context root).'),
    namespace: z.string().optional().describe('Default `evaAI` per ADR--PATH-ENCODING.'),
    dry_run: z.boolean().optional().describe('Preview counts without writing.'),
    check: z.boolean().optional().describe('Exit with error if file would change (drift check).'),
};
export function handler(ctx) {
    return async (args) => {
        const root = resolve(args.root ?? ctx.root);
        try {
            const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp');
            const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`);
            const result = await rebuildBacklinks({
                root,
                namespace: args.namespace,
                dryRun: args.dry_run === true,
                check: args.check === true,
                subject,
                context,
            });
            if (args.check && result.changed) {
                return errorResult(`backlinks drift: ${result.outputPath} differs (${result.edgeCount} edges from ${result.atomCount} atoms)`);
            }
            return jsonResult(result);
        }
        catch (err) {
            return errorResult(`backlinks rebuild failed: ${err.message}`);
        }
    };
}
