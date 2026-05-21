import { resolve } from 'node:path';
import { z } from 'zod';
import { createSlmClient } from '../../codegen/slm/factory.js';
import { appendEpisode } from '../../memory/episodic/writer.js';
import { consolidate } from '../../orchestrator/consolidator/index.js';
import { makeContext, makeSubject } from '../../policy/types.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_remember';
export const description = 'Consolidate a session.jsonl into Episode[] (hybrid tier-1 deterministic + tier-2 LLM borderline) and persist each emitted episode via the episodic writer. Returns the episodes plus a count of how many were written.';
export const inputSchema = {
    session_id: z.string().describe('Session id (file basename, without .jsonl).'),
    namespace: z
        .string()
        .optional()
        .describe('Project namespace (default `evaAI`).'),
    provider: z
        .enum(['mock', 'ollama'])
        .optional()
        .describe('SLM provider for tier-2 calls. Default `mock` (safe for any agent).'),
    root: z.string().optional().describe('Project root (default: server context root).'),
};
const DEFAULT_NAMESPACE = 'evaAI';
/**
 * Adapt a consolidator Episode into the on-disk MemoryEpisode shape so the
 * episodic writer (which expects `episodicId`, `range[]`, `content.summary`,
 * `importance_score`, `projectId`) can persist it.
 *
 * - `episodicId` is derived deterministically: `${sessionId}-${start}-${end}`
 * - `range` uses the existing `turnIdx-N..turnIdx-M` form (or single `turnIdx-N`)
 * - `projectId` reuses namespace (existing convention; see episode-append.ts test)
 */
function toMemoryEpisode(ep, namespace) {
    const [start, end] = ep.turnRange;
    const range = start === end
        ? [`turnIdx-${start}`]
        : [`turnIdx-${start}..turnIdx-${end}`];
    return {
        episodicId: `${ep.sessionId}-${start}-${end}`,
        sessionId: ep.sessionId,
        projectId: namespace,
        timestamp: ep.createdAt,
        importance_score: ep.score,
        range,
        content: { summary: ep.summary },
        tags: ep.tags,
    };
}
export function handler(ctx) {
    return async (args) => {
        const root = resolve(args.root ?? ctx.root);
        const namespace = args.namespace ?? DEFAULT_NAMESPACE;
        try {
            const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp');
            const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`);
            const llm = createSlmClient({ provider: args.provider ?? 'mock' });
            const episodes = await consolidate({
                sessionId: args.session_id,
                root,
                namespace,
                llm,
                subject,
                context,
            });
            const llmCalls = episodes.filter((e) => e.scoreSource === 'tier2').length;
            let persisted = 0;
            for (const ep of episodes) {
                await appendEpisode(toMemoryEpisode(ep, namespace), { root, namespace, subject, context });
                persisted += 1;
            }
            return jsonResult({
                ok: true,
                episodes_emitted: episodes,
                episodes_persisted: persisted,
                llm_calls: llmCalls,
            });
        }
        catch (err) {
            return errorResult(`remember failed: ${err.message}`);
        }
    };
}
