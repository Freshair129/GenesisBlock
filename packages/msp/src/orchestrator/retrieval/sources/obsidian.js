import { performance } from 'node:perf_hooks';
import { basename, extname } from 'node:path';
function raceTimeout(p, timeoutMs) {
    return new Promise((resolve, reject) => {
        const t = setTimeout(() => reject(new Error('timeout')), timeoutMs);
        p.then((v) => {
            clearTimeout(t);
            resolve(v);
        }, (e) => {
            clearTimeout(t);
            reject(e);
        });
    });
}
const ATOM_ID_RE = /^([A-Z][A-Z0-9-]+--[A-Z0-9-]+)$/;
/**
 * Best-effort atom-id extraction from the search result path.
 * For atom files (`gks/<type>/ADR--FOO.md`), returns `ADR--FOO`. For
 * vault notes that don't match the atom-id shape, returns the basename
 * without extension as a stable identifier.
 */
function atomIdFromPath(relPath) {
    const base = basename(relPath, extname(relPath));
    if (ATOM_ID_RE.test(base))
        return base;
    return base;
}
/**
 * Obsidian text-search source. Caller MUST provide the client (M7c stays
 * orthogonal to M7a's lifecycle — see BLUEPRINT). Records the source name
 * based on the client's mode: `'obsidian-text'` for REST, `'grep'` for the
 * filesystem fallback.
 */
export async function obsidianSource(opts) {
    const start = performance.now();
    const sourceName = opts.obsidian?.mode === 'rest' ? 'obsidian-text' : 'grep';
    if (!opts.obsidian) {
        return {
            source: 'obsidian-text',
            hits: [],
            latencyMs: Math.round(performance.now() - start),
            skipped: 'no-client',
        };
    }
    const query = opts.query.trim();
    if (!query) {
        return {
            source: sourceName,
            hits: [],
            latencyMs: Math.round(performance.now() - start),
        };
    }
    try {
        const results = await raceTimeout(opts.obsidian.search(query, { limit: opts.topK }), opts.timeoutMs);
        const hits = results
            .slice(0, opts.topK)
            .map((r, i) => ({
            atomId: atomIdFromPath(r.path),
            rank: i + 1,
            snippet: r.snippet,
            source: sourceName,
        }));
        return {
            source: sourceName,
            hits,
            latencyMs: Math.round(performance.now() - start),
        };
    }
    catch (err) {
        const msg = err instanceof Error ? err.message : String(err ?? 'unknown error');
        return {
            source: sourceName,
            hits: [],
            latencyMs: Math.round(performance.now() - start),
            error: msg,
        };
    }
}
