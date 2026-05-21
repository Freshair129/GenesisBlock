import { resolve } from 'node:path';
import { z } from 'zod';
import { createSlmClient } from '../../codegen/slm/factory.js';
import { compress } from '../../orchestrator/compressor/index.js';
import { makeContext, makeSubject } from '../../policy/types.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_compress';
export const description = 'Compress importance-scored episodes to fit a token budget via three-tier strategy (keep / trim / resummarise / truncate). Pure read + transform — does NOT persist anything. Returns the compressed episodes with provenance and per-tier counts.';
export const inputSchema = {
    episodes: z
        .array(z.object({
        sessionId: z.string(),
        turnRange: z.tuple([z.number(), z.number()]),
        summary: z.string(),
        score: z.number(),
        turns: z.array(z.unknown()),
        atomId: z.string().optional(),
    }))
        .describe('CompressorEpisode[] — episodes with their source turns + score.'),
    budget_tokens: z.number().int().positive().describe('Total token budget for the compressed output.'),
    preserve_order: z
        .boolean()
        .optional()
        .describe('If true, output is reordered chronologically by turnRange[0] after selection. Default false (importance-desc).'),
    llm_timeout_ms: z
        .number()
        .int()
        .positive()
        .optional()
        .describe('Per-LLM-call timeout in ms. Default 8000.'),
    provider: z
        .enum(['mock', 'ollama'])
        .optional()
        .describe('SLM provider for tier-3 resummarise calls. Default `mock` (safe; truncate fallback always works).'),
    root: z.string().optional().describe('Project root (default: server context root).'),
};
export function handler(ctx) {
    return async (args) => {
        void resolve(args.root ?? ctx.root); // root accepted for parity; compress doesn't read fs
        try {
            const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp');
            const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`);
            const llm = createSlmClient({ provider: args.provider ?? 'mock' });
            const result = await compress({
                episodes: args.episodes,
                budgetTokens: args.budget_tokens,
                llm,
                llmTimeoutMs: args.llm_timeout_ms,
                preserveOrder: args.preserve_order,
                subject,
                context,
            });
            return jsonResult({
                ok: true,
                compressed: result.compressed,
                total_tokens_used: result.totalTokensUsed,
                tier_counts: result.tierCounts,
            });
        }
        catch (err) {
            return errorResult(`compress failed: ${err.message}`);
        }
    };
}
