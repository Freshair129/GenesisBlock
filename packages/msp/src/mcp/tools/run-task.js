import { resolve } from 'node:path';
import { z } from 'zod';
import { runTask } from '../../codegen/runner.js';
import { createSlmClient } from '../../codegen/slm/factory.js';
import { makeContext, makeSubject } from '../../policy/types.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_run_task';
export const description = 'Execute one T*.task.yaml under the codegen contract. Default SLM provider is `mock` (safe for any agent); set provider:"ollama" for real codegen against a local Ollama.';
export const inputSchema = {
    task_path: z.string().describe('Path to T*.task.yaml (absolute or root-relative).'),
    provider: z.enum(['mock', 'ollama']).optional().describe('SLM provider. Default `mock`.'),
    dry_run: z.boolean().optional().describe('If true, render prompt without calling SLM.'),
    root: z.string().optional().describe('Project root (default: server context root).'),
};
export function handler(ctx) {
    return async (args) => {
        const root = resolve(args.root ?? ctx.root);
        const taskPath = resolve(root, args.task_path);
        const originalCwd = process.cwd();
        process.chdir(root);
        try {
            const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp');
            const action = 'expose-to-llm';
            const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`);
            console.debug(`[ucf] 4-tuple: msp_run_task | sub:${subject.id} | act:${action} | trace:${context.trace_id}`);
            const result = await runTask(taskPath, {
                slmClient: createSlmClient({ provider: args.provider ?? 'mock' }),
                dryRun: args.dry_run === true,
                // Phase 0: we don't pass the tuple yet as runTask doesn't accept it,
                // but we've logged it and identified the action.
            });
            return jsonResult(result);
        }
        catch (err) {
            return errorResult(`runTask failed: ${err.message}`);
        }
        finally {
            process.chdir(originalCwd);
        }
    };
}
