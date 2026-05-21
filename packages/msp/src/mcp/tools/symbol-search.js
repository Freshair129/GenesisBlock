import { resolve } from 'node:path';
import { z } from 'zod';
import { SymbolStore } from '../../symbols/store/sqlite.js';
import { dbPath, graphExists } from '../../symbols/util.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_symbol_search';
export const description = 'Substring + token-fuzzy search across symbol `name + signature`. Returns ranked hits with a numeric score. Read-only.';
export const inputSchema = {
    query: z.string().min(1).describe('Search query (substring + token-fuzzy match).'),
    limit: z
        .number()
        .int()
        .positive()
        .optional()
        .describe('Max results to return (default 20).'),
    root: z.string().optional().describe('Project root (default: server context root).'),
    namespace: z.string().optional().describe('Project namespace (default: evaAI).'),
};
const DEFAULT_LIMIT = 20;
/**
 * Simple fuzzy score: 100 for exact name match, 80 for case-insensitive name
 * match, 60 for prefix, 40 for substring in name, 20 for substring in
 * signature, with bonuses for `exported` and small file depth.
 */
function score(symbol, query, queryLower) {
    const nameLower = symbol.name.toLowerCase();
    const sigLower = (symbol.signature ?? '').toLowerCase();
    let s = 0;
    if (symbol.name === query)
        s = 100;
    else if (nameLower === queryLower)
        s = 80;
    else if (nameLower.startsWith(queryLower))
        s = 60;
    else if (nameLower.includes(queryLower))
        s = 40;
    else if (sigLower.includes(queryLower))
        s = 20;
    else
        return 0;
    // Bonus: exported.
    if (symbol.exported)
        s += 5;
    // Penalty: deeper files.
    s -= Math.min(5, symbol.file.split('/').length);
    return s;
}
export function handler(ctx) {
    return async (args) => {
        const root = resolve(args.root ?? ctx.root);
        const namespace = args.namespace;
        if (!graphExists(root, namespace)) {
            return errorResult("graph not built — run 'npm run msp:graph build' first");
        }
        const store = new SymbolStore();
        try {
            store.open(dbPath(root, namespace));
            if (!store.getMeta('last_built_at')) {
                return errorResult("graph not built — run 'npm run msp:graph build' first");
            }
            const limit = args.limit ?? DEFAULT_LIMIT;
            const all = store.allSymbols();
            const queryLower = args.query.toLowerCase();
            const scored = [];
            for (const s of all) {
                const v = score(s, args.query, queryLower);
                if (v > 0)
                    scored.push({ ...s, score: v });
            }
            scored.sort((a, b) => {
                if (a.score !== b.score)
                    return b.score - a.score;
                return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
            });
            return jsonResult({ ok: true, hits: scored.slice(0, limit) });
        }
        catch (err) {
            return errorResult(`symbol_search failed: ${err.message}`);
        }
        finally {
            try {
                store.close();
            }
            catch {
                // ignore
            }
        }
    };
}
