import { z } from 'zod';
import { generateIdentityPreamble } from '../../orchestrator/persona/preamble.js';
export const name = 'msp_identity_beliefs';
export const description = "Returns the agent's long-term identity beliefs formatted as a system preamble.";
export const inputSchema = z.object({});
export function handler(_ctx) {
    return async () => {
        try {
            const preamble = await generateIdentityPreamble();
            return {
                content: [
                    {
                        type: 'text',
                        text: preamble || 'No durable identity beliefs have been established for this namespace yet.'
                    }
                ]
            };
        }
        catch (err) {
            return {
                isError: true,
                content: [{ type: 'text', text: `Failed to fetch identity beliefs: ${err.message}` }]
            };
        }
    };
}
