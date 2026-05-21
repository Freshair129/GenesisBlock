import { resolve } from 'node:path';
import { z } from 'zod';
import { CandidateWriter } from '../../memory/candidates/writer.js';
import { makeContext, makeSubject } from '../../policy/types.js';
import { jsonResult } from '../types.js';
export const name = 'msp_candidate';
export const description = 'Record a structurally-shaped candidate atom under .brain/msp/projects/<ns>/candidates/. Candidates are agent-runtime drafts; promotion to canon (gks/) is a human PR action. Returns the candidate file path.';
export const inputSchema = {
    proposed_id: z
        .string()
        .describe('Atomic id (E.g. CONCEPT-371--DATABASE-CHOICE--K2 or CONCEPT--FOO; type must be one of CONCEPT|ADR|FEAT|BLUEPRINT|GENESIS|FRAMEWORK|AUDIT|PROTO).'),
    type: z.string().describe('Atom type (concept, adr, feat, blueprint, genesis, framework, audit, proto).'),
    title: z.string().describe('Human-readable title (≤ 100 chars).'),
    body: z.string().describe('Body markdown. Composed under a "# {title}" heading if no leading heading present.'),
    rationale: z.string().optional().describe('Optional one-line note on why this candidate exists.'),
    confidence: z.number().min(0).max(1).optional().describe('Optional confidence 0..1.'),
    namespace: z.string().optional().describe('Project namespace (default: evaAI).'),
    root: z.string().optional().describe('Project root (default: server context root).'),
};
export function handler(ctx) {
    return async (args) => {
        const root = resolve(args.root ?? ctx.root);
        const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp');
        const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`);
        const writer = new CandidateWriter({
            root,
            namespace: args.namespace,
            proposedBy: 'agent',
            subject,
            context,
        });
        const result = await writer.write({
            type: args.type,
            proposed_id: args.proposed_id,
            title: args.title,
            body: args.body,
            rationale: args.rationale,
            confidence: args.confidence,
        });
        return jsonResult({
            proposed_id: args.proposed_id,
            candidate_path: result.path,
            overwritten: result.overwritten,
        });
    };
}
