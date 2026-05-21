import { resolve } from 'node:path';
import { z } from 'zod';
import { SymbolStore } from '../../symbols/store/sqlite.js';
import { dbPath, graphExists } from '../../symbols/util.js';
import { errorResult, jsonResult } from '../types.js';
export const name = 'msp_symbol_lookup';
export const description = 'Look up Symbol Graph nodes by exact or prefix name match. Read-only. Returns hits ranked by exported-first then file depth. Requires `msp:graph build` to have run.';
export const inputSchema = {
    name: z.string().describe('Symbol name to look up (exact-match preferred, prefix fallback).'),
    kind: z
        .string()
        .optional()
        .describe('Optional filter: function | method | class | interface | type | enum | const | module.'),
    root: z.string().optional().describe('Project root (default: server context root).'),
    namespace: z.string().optional().describe('Project namespace (default: evaAI).'),
};
const KNOWN_KINDS = [
    'function',
    'method',
    'class',
    'interface',
    'type',
    'enum',
    'const',
    'module',
];
function fileDepth(file) {
    return file.split('/').length;
}
function rank(symbols) {
    return [...symbols].sort((a, b) => {
        if (a.exported !== b.exported)
            return a.exported ? -1 : 1;
        const da = fileDepth(a.file);
        const db = fileDepth(b.file);
        if (da !== db)
            return da - db;
        if (a.file !== b.file)
            return a.file < b.file ? -1 : 1;
        return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
    });
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
            // Subject/context (UCF Phase 4) reserved; SymbolStore.open does not
            // currently consume policy metadata.
            void ctx.subject;
            void ctx.policyContext;
            store.open(dbPath(root, namespace));
            if (!store.getMeta('last_built_at')) {
                return errorResult("graph not built — run 'npm run msp:graph build' first");
            }
            if (args.kind && !KNOWN_KINDS.includes(args.kind)) {
                return errorResult(`unknown kind "${args.kind}"`);
            }
            const all = store.allSymbols();
            let exact = all.filter((s) => s.name === args.name);
            let prefix = all.filter((s) => s.name !== args.name && s.name.startsWith(args.name));
            if (args.kind) {
                exact = exact.filter((s) => s.kind === args.kind);
                prefix = prefix.filter((s) => s.kind === args.kind);
            }
            const hits = [...rank(exact), ...rank(prefix)];
            return jsonResult({ ok: true, hits });
        }
        catch (err) {
            return errorResult(`symbol_lookup failed: ${err.message}`);
        }
        finally {
            try {
                store.close();
            }
            catch {
                // ignore close errors
            }
        }
    };
}
