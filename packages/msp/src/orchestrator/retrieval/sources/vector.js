import { performance } from 'node:perf_hooks';
/**
 * Race a promise against a timer. On timeout, the original promise is left
 * dangling — acceptable for M7c per BLUEPRINT (no cleanup needed; vector +
 * obsidian have their own internal aborts).
 */
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
function snippetFromMetadata(doc) {
    if (typeof doc.text === 'string' && doc.text.length > 0) {
        return doc.text.slice(0, 240).replace(/\s+/g, ' ').trim();
    }
    const md = doc.metadata;
    if (md && typeof md.title === 'string')
        return md.title;
    return undefined;
}
/**
 * GKS vector semantic search source. Wraps an externally-supplied embedder
 * + vector backend (caller owns lifecycle). Returns hits with ranks 1..topK
 * derived from the backend's ranking.
 */
export async function vectorSource(opts) {
    const start = performance.now();
    if (!opts.embedder || !opts.vectorBackend) {
        return {
            source: 'gks-vector',
            hits: [],
            latencyMs: Math.round(performance.now() - start),
            error: 'no-embedder',
        };
    }
    const query = opts.query.trim();
    if (!query) {
        return {
            source: 'gks-vector',
            hits: [],
            latencyMs: Math.round(performance.now() - start),
        };
    }
    try {
        const vector = await raceTimeout(opts.embedder.embed(query), opts.timeoutMs);
        const remaining = Math.max(1, opts.timeoutMs - (performance.now() - start));
        const results = await raceTimeout(opts.vectorBackend.search(vector, { topK: opts.topK }), remaining);
        const hits = results.slice(0, opts.topK).map((r, i) => ({
            atomId: r.doc.metadata?.atom_id ?? r.doc.id,
            rank: i + 1,
            snippet: snippetFromMetadata(r.doc),
            source: 'gks-vector',
            attributes: r.doc.metadata?.attributes ?? {},
        }));
        return {
            source: 'gks-vector',
            hits,
            latencyMs: Math.round(performance.now() - start),
        };
    }
    catch (err) {
        const msg = err instanceof Error ? err.message : String(err ?? 'unknown error');
        return {
            source: 'gks-vector',
            hits: [],
            latencyMs: Math.round(performance.now() - start),
            error: msg,
        };
    }
}
