import { resolve as resolvePath } from 'node:path';
import { z } from 'zod';
import { resolveProject } from '../../projects/resolve.js';
import { makeContext, makeSubject } from '../../policy/types.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_project_resolve';
export const description = 'Return the resolved active project for the current invocation. Useful for debugging "which project loaded" questions. Resolution chain: CLI flag → MSP_PROJECT env → `.mspconfig` walk → registry default → literal `default`. Errors loudly if the resolved name is not in the registry.';
export const inputSchema = {
    cli_flag: z
        .string()
        .optional()
        .describe('Simulate a CLI flag value (e.g. argv `--project=eva`).'),
    env: z
        .string()
        .optional()
        .describe('Simulate an env var value (default: process.env.MSP_PROJECT).'),
    cwd: z
        .string()
        .optional()
        .describe('Starting directory for the `.mspconfig` walk (default: server context root).'),
};
export function handler(ctx) {
    return async (args) => {
        try {
            const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp');
            const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`);
            const cwd = resolvePath(args.cwd ?? ctx.root);
            const opts = {
                cwd,
                subject,
                context,
            };
            if (typeof args.cli_flag === 'string')
                opts.cliFlag = args.cli_flag;
            if (typeof args.env === 'string')
                opts.env = args.env;
            else if (typeof process.env['MSP_PROJECT'] === 'string') {
                opts.env = process.env['MSP_PROJECT'];
            }
            const resolved = await resolveProject(opts);
            return jsonResult({ ok: true, resolved });
        }
        catch (err) {
            return errorResult(`project_resolve failed: ${err.message}`);
        }
    };
}
