import { readFile, readdir, stat } from 'node:fs/promises';
import { extname, join, relative, resolve } from 'node:path';
const SEARCH_DIRS = ['concept', 'adr', 'feat', 'blueprint', 'frame', 'framework', 'audit', 'task', 'issues'];
async function listMarkdown(root) {
    const out = [];
    const gks = resolve(root, 'gks');
    let entries;
    try {
        entries = await readdir(gks);
    }
    catch {
        return out;
    }
    for (const sub of entries) {
        if (!SEARCH_DIRS.includes(sub))
            continue;
        const subPath = join(gks, sub);
        let st;
        try {
            st = await stat(subPath);
        }
        catch {
            continue;
        }
        if (!st.isDirectory())
            continue;
        let files;
        try {
            files = await readdir(subPath);
        }
        catch {
            continue;
        }
        for (const f of files) {
            if (extname(f) === '.md')
                out.push(join(subPath, f));
        }
    }
    return out;
}
function titleFrom(body, fallback) {
    const m = body.match(/^title:\s*(.+)$/m);
    if (m)
        return m[1].trim().replace(/^['"]|['"]$/g, '');
    const h = body.match(/^#\s+(.+)$/m);
    if (h)
        return h[1].trim();
    return fallback;
}
function snippetFor(body, query, len = 160) {
    const lower = body.toLowerCase();
    const q = query.toLowerCase();
    const idx = lower.indexOf(q);
    if (idx < 0)
        return body.slice(0, len).replace(/\s+/g, ' ').trim();
    const start = Math.max(0, idx - Math.floor(len / 2));
    return body.slice(start, start + len).replace(/\s+/g, ' ').trim();
}
export function makeFilesystemClient(opts) {
    const root = resolve(opts.root);
    return {
        mode: 'filesystem',
        async search(query, { limit = 10 } = {}) {
            const q = query.toLowerCase().trim();
            if (!q)
                return [];
            const files = await listMarkdown(root);
            const hits = [];
            for (const abs of files) {
                let body;
                try {
                    body = await readFile(abs, 'utf8');
                }
                catch {
                    continue;
                }
                const lower = body.toLowerCase();
                if (!lower.includes(q))
                    continue;
                const title = titleFrom(body, abs.split('/').pop() ?? abs);
                const titleHit = title.toLowerCase().includes(q) ? 1 : 0;
                const occurrences = lower.split(q).length - 1;
                hits.push({
                    path: relative(root, abs),
                    title,
                    snippet: snippetFor(body, q),
                    score: titleHit * 2 + Math.min(occurrences, 10) / 10,
                });
            }
            return hits.sort((a, b) => b.score - a.score).slice(0, limit);
        },
        async readFile(relPath) {
            return readFile(resolve(root, relPath), 'utf8');
        },
    };
}
