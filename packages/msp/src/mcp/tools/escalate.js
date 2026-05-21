import { z } from 'zod';
import { createCognitiveLayer } from '../../cognitive/index.js';
import { jsonResult } from '../types.js';
export const name = 'msp_escalate';
export const description = 'Request a context scope expansion (UCF Phase 2). Use this when you need access to information or domains that are currently denied or missing.';
export const inputSchema = {
    request_scope_extension: z
        .array(z.string())
        .optional()
        .describe('List of domain names to request access to.'),
    reason: z.string().describe('Explanation of why this extension is required for the task.'),
};
export function handler(ctx) {
    return async (args) => {
        try {
            // Subject/context (UCF Phase 4) reserved; escalate does not currently
            // consume policy metadata.
            void ctx.subject;
            void ctx.policyContext;
            const layer = await createCognitiveLayer({ root: ctx.root });
            const result = await layer.escalate({
                request_scope_extension: args.request_scope_extension,
                reason: args.reason,
            });
            return jsonResult(result);
        }
        catch (err) {
            return {
                content: [{ type: 'text', text: `escalation failed: ${err.message}` }],
                isError: true,
            };
        }
    };
}
