import { z } from 'zod';
import { resolve as resolveQuery } from '../../brain/resolver.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_brain_resolve';
export const description = 'Resolve an atom query against the Two-Brain (project + global) vaults using the deterministic per-type routing table from BLUEPRINT--BRAIN-MERGE-STRATEGY. Returns deduped hits with each atom\'s id, type, source (project|global), and absolute path. Use this instead of hardcoding atom paths.';
const ATOM_TYPES = [
    'ADR',
    'FEAT',
    'BLUEPRINT',
    'AUDIT',
    'CONCEPT',
    'FRAMEWORK',
    'SPEC',
    'PROTOCOL',
    'SKILL',
    'ALGO',
    'PROTO',
    'PARAMS',
    'EPISODE',
    'IDENTITY',
    'REGISTRY',
    'MOD',
    'MASTER',
    'HOTFIX',
    'INC',
    'ISSUE',
];
const ATOM_TYPE_SET = new Set(ATOM_TYPES);
export const inputSchema = {
    id: z
        .string()
        .optional()
        .describe('Optional exact atom id to filter for (e.g. "ADR--MONOREPO-STRUCTURE").'),
    type: z
        .string()
        .optional()
        .describe('Optional AtomType prefix to scope the search (e.g. "ADR", "BLUEPRINT"). Must be one of the known taxonomy v2.3 types.'),
    vault_id: z
        .string()
        .optional()
        .describe('Optional vault identifier (reserved; passed through to the resolver).'),
};
export function handler(ctx) {
    return async (args) => {
        if (args.type !== undefined && !ATOM_TYPE_SET.has(args.type)) {
            return errorResult(`invalid AtomType: "${args.type}". Must be one of: ${ATOM_TYPES.join(', ')}`);
        }
        const query = {};
        if (args.id !== undefined)
            query.id = args.id;
        if (args.type !== undefined)
            query.type = args.type;
        if (args.vault_id !== undefined)
            query.vault_id = args.vault_id;
        // Subject/context (UCF Phase 4) intentionally unused — resolver does not
        // currently consume policy metadata; left in ctx for future wiring.
        void ctx;
        try {
            const hits = await resolveQuery(query);
            return jsonResult({
                ok: true,
                count: hits.length,
                hits: hits.map((h) => ({
                    id: h.atom.id,
                    type: h.atom.type,
                    source: h.source,
                    path: h.path,
                })),
            });
        }
        catch (err) {
            return errorResult(`brain_resolve failed: ${err.message}`);
        }
    };
}
