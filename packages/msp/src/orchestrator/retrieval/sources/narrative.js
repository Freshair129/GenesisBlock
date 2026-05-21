import { performance } from 'node:perf_hooks';
import { join } from 'node:path';
import { readdir } from 'node:fs/promises';
import { DEFAULT_PER_SOURCE_TIMEOUTS, } from '../types.js';
import { parseFile } from '../../../validator/parse.js';
/**
 * Retrieval source for Narrative atoms (Tier 2).
 * Reads from gks/narrative/ and filters by namespace if possible.
 */
export async function narrativeSource(opts) {
    const start = performance.now();
    const root = opts.root ?? process.cwd();
    const timeoutMs = opts.perSourceTimeouts?.narrative ?? DEFAULT_PER_SOURCE_TIMEOUTS.narrative;
    const narrativeDir = join(root, 'gks', 'narrative');
    try {
        const files = await readdir(narrativeDir);
        const mdFiles = files.filter(f => f.endsWith('.md'));
        const hits = [];
        // For Phase 1, we do a simple substring search on the query
        // In future, this should use the vector store 'memory/narrative' namespace
        const needle = opts.query.toLowerCase();
        for (const file of mdFiles) {
            const filePath = join(narrativeDir, file);
            try {
                const parsed = await parseFile(filePath);
                const fm = parsed.fm;
                const text = (fm['title'] || '') + ' ' + (fm['summary'] || '') + ' ' + parsed.body;
                if (text.toLowerCase().includes(needle)) {
                    hits.push({
                        atomId: fm['id'],
                        rank: 1, // Will be sorted later
                        snippet: fm['summary'] || parsed.body.slice(0, 200),
                        source: 'narrative',
                        memoryTier: 'narrative',
                        attributes: fm['attributes']
                    });
                }
            }
            catch {
                continue;
            }
            if (performance.now() - start > timeoutMs)
                break;
        }
        // Basic rank based on match quality or just order for now
        const rankedHits = hits.map((h, i) => ({ ...h, rank: i + 1 }));
        return {
            source: 'narrative',
            hits: rankedHits,
            latencyMs: Math.round(performance.now() - start)
        };
    }
    catch (err) {
        return {
            source: 'narrative',
            hits: [],
            latencyMs: Math.round(performance.now() - start),
            error: err.message
        };
    }
}
