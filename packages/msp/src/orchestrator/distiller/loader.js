import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { parseFile } from '../../validator/parse.js';
/**
 * Loads all Narrative atoms from GKS that have not yet been distilled.
 */
export async function loadUndistilledNarratives(opts) {
    const narrativeDir = join(opts.root, 'gks', 'narrative');
    let files;
    try {
        files = await readdir(narrativeDir);
    }
    catch (err) {
        if (err.code === 'ENOENT')
            return [];
        throw err;
    }
    const narratives = [];
    for (const file of files) {
        if (!file.endsWith('.md'))
            continue;
        const filePath = join(narrativeDir, file);
        try {
            const parsed = await parseFile(filePath);
            const fm = parsed.fm;
            narratives.push({
                id: fm['id'],
                namespace: opts.namespace,
                created_at: fm['created_at'],
                domain: fm['domain'] || 'meta',
                epistemic_state: fm['epistemic_state'] || 'confirmed',
                confidence: fm['confidence'] || 0.8,
                encoding_level: fm['encoding_level'] || 'L2',
                source_episodes: [],
                content: {
                    summary: fm['summary'] || parsed.body.slice(0, 500),
                    key_decisions: [],
                    unresolved_questions: [],
                    patterns_observed: []
                }
            });
        }
        catch (err) {
            console.warn(`[distiller] failed to load narrative ${file}: ${err.message}`);
        }
        if (opts.limit && narratives.length >= opts.limit)
            break;
    }
    return narratives;
}
/**
 * Pillar 0: LOADER
 * Finds and reads consolidated episodes that have not been distilled into
 * a narrative yet.
 */
export async function loadUndistilledEpisodes(opts) {
    const memoryDir = join(opts.root, '.brain', 'msp', 'projects', opts.namespace, 'memory', 'sessions');
    let files;
    try {
        files = await readdir(memoryDir);
    }
    catch (err) {
        if (err.code === 'ENOENT')
            return [];
        throw err;
    }
    const episodes = [];
    const sortedFiles = files.filter(f => f.endsWith('.json')).sort();
    for (const file of sortedFiles) {
        const filePath = join(memoryDir, file);
        try {
            const raw = await readFile(filePath, 'utf8');
            const data = JSON.parse(raw);
            if (Array.isArray(data)) {
                for (const ep of data) {
                    if (!ep.distilled) {
                        episodes.push(ep);
                    }
                }
            }
        }
        catch (err) {
            console.warn(`[distiller] failed to read episode file ${file}: ${err.message}`);
        }
        if (opts.limit && episodes.length >= opts.limit)
            break;
    }
    return episodes;
}
