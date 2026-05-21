import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parse as parseYaml } from 'yaml';
import { atomicWrite } from './atomic-write.js';
import { edgesFromAtom, sortEdges } from './edges.js';
import { walkMarkdown } from './walk.js';
const DEFAULT_NAMESPACE = 'evaAI';
function backlinksPath(root, namespace) {
    return resolve(root, '.brain/msp/projects', namespace, 'vector/backlinks.jsonl');
}
function extractFrontmatter(text) {
    if (!text.startsWith('---'))
        return null;
    const end = text.indexOf('\n---', 3);
    if (end === -1)
        return null;
    const fmText = text.slice(3, end).trim();
    try {
        const parsed = parseYaml(fmText);
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
            return parsed;
        }
        return null;
    }
    catch {
        return null;
    }
}
function serialise(edges) {
    if (edges.length === 0)
        return '';
    return edges.map((e) => JSON.stringify(e)).join('\n') + '\n';
}
async function readExisting(path) {
    try {
        return await readFile(path, 'utf8');
    }
    catch (err) {
        if (err.code === 'ENOENT')
            return '';
        throw err;
    }
}
export async function rebuildBacklinks(opts) {
    const root = resolve(opts.root);
    const namespace = opts.namespace ?? DEFAULT_NAMESPACE;
    const outputPath = backlinksPath(root, namespace);
    const gksDir = resolve(root, 'gks');
    const edges = [];
    let atomCount = 0;
    for await (const file of walkMarkdown(gksDir)) {
        atomCount++;
        const text = await readFile(file, 'utf8').catch(() => null);
        if (text === null)
            continue;
        const fm = extractFrontmatter(text);
        if (!fm)
            continue;
        edges.push(...edgesFromAtom(fm));
    }
    const sorted = sortEdges(edges);
    const content = serialise(sorted);
    if (opts.check) {
        const existing = await readExisting(outputPath);
        return {
            atomCount,
            edgeCount: sorted.length,
            changed: existing !== content,
            outputPath,
        };
    }
    if (opts.dryRun) {
        return {
            atomCount,
            edgeCount: sorted.length,
            changed: false,
            outputPath,
        };
    }
    const existing = await readExisting(outputPath);
    const changed = existing !== content;
    if (changed) {
        await atomicWrite(outputPath, content);
    }
    return {
        atomCount,
        edgeCount: sorted.length,
        changed,
        outputPath,
    };
}
