/**
 * §13 Hybrid Retrieval — layer 2 (FTS).
 *
 * Pure-Node full-text search over `gks/<type>/*.md`. Used by
 * `createCognitiveLayer({ ... }).recall()` between the atomic-id short
 * circuit (layer 1) and the vector / graph parallel dispatch (layers 3/4).
 *
 * Implementation choices:
 *   - No ripgrep dependency → works in CI / read-only env without a binary.
 *   - Case-insensitive substring match on title + body.
 *   - Token-overlap score (Jaccard-ish) to break ties when many atoms hit.
 *   - Returns at most `limit` hits, default 10.
 *
 * This is intentionally simple; the cheap-cascade design (§13.2 step 1)
 * relies on the more expensive vector + graph layers to handle semantic
 * recall — FTS only needs to catch keyword hits the index missed.
 */
import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
export async function ftsSearch(gksRoot, query, opts = {}) {
    const limit = opts.limit ?? 10;
    const q = query.trim().toLowerCase();
    if (!q)
        return [];
    const tokens = q.split(/\s+/).filter(Boolean);
    if (tokens.length === 0)
        return [];
    const out = [];
    let types = [];
    try {
        types = await readdir(gksRoot);
    }
    catch {
        return [];
    }
    for (const type of types) {
        if (type === '00_index')
            continue;
        const typeDir = join(gksRoot, type);
        let files = [];
        try {
            files = await readdir(typeDir);
        }
        catch {
            continue;
        }
        for (const file of files) {
            if (!file.endsWith('.md'))
                continue;
            let body;
            try {
                body = await readFile(join(typeDir, file), 'utf8');
            }
            catch {
                continue;
            }
            const lower = body.toLowerCase();
            let matches = 0;
            for (const t of tokens) {
                if (lower.includes(t))
                    matches++;
            }
            if (matches === 0)
                continue;
            const id = file.replace(/\.md$/, '');
            const title = extractTitle(body) ?? id;
            const score = matches / tokens.length;
            out.push({
                hit: {
                    id,
                    source: 'vector',
                    score,
                    path: join('gks', type, file),
                    title,
                    snippet: snippet(body, tokens),
                    metadata: { type, matchedBy: 'fts' },
                },
                score,
            });
        }
    }
    out.sort((a, b) => b.score - a.score);
    return out.slice(0, limit).map((r) => r.hit);
}
function extractTitle(text) {
    const fm = /^---\n([\s\S]*?)\n---/.exec(text);
    if (!fm)
        return null;
    const m = /^title:\s*(.+)$/m.exec(fm[1]);
    return m ? m[1].trim() : null;
}
function snippet(text, tokens) {
    // Strip frontmatter so the snippet shows real body content.
    const body = text.replace(/^---\n[\s\S]*?\n---\n/, '');
    const lower = body.toLowerCase();
    for (const t of tokens) {
        const i = lower.indexOf(t);
        if (i >= 0) {
            const start = Math.max(0, i - 60);
            const end = Math.min(body.length, i + 180);
            const fragment = body.slice(start, end).replace(/\s+/g, ' ').trim();
            return (start > 0 ? '…' : '') + fragment + (end < body.length ? '…' : '');
        }
    }
    return body.slice(0, 200).replace(/\s+/g, ' ').trim();
}
