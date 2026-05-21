import { z } from 'zod';
import { distill } from '../../orchestrator/distiller/index.js';
export const name = 'msp_distill';
export const description = 'Triggers the asynchronous 8-8-8 memory distillation cycle for a namespace.';
export const inputSchema = z.object({
    namespace: z.string().describe('Target project namespace (e.g. evaAI)'),
    dryRun: z.boolean().optional().describe('If true, only reports what would be synthesised'),
    force: z.boolean().optional().describe('If true, bypasses the 8-session threshold check')
});
export function handler(ctx) {
    return async (args) => {
        try {
            const result = await distill({
                namespace: args.namespace,
                root: ctx.root,
                dryRun: args.dryRun,
                force: args.force
            });
            return {
                content: [
                    {
                        type: 'text',
                        text: `Distillation complete for '${args.namespace}'.
- Episodes processed: ${result.episodesProcessed}
- Narratives created: ${result.narrativesCreated}
- Identity beliefs revised: ${result.beliefsRevised}
${result.errors.length > 0 ? `\nErrors:\n${result.errors.join('\n')}` : ''}`
                    }
                ]
            };
        }
        catch (err) {
            return {
                isError: true,
                content: [{ type: 'text', text: `Distillation failed: ${err.message}` }]
            };
        }
    };
}
