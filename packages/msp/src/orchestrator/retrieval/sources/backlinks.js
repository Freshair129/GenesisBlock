import { createReadStream } from 'node:fs';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { createInterface } from 'node:readline';
function backlinksPath(root, namespace) {
    return resolve(root, '.brain/msp/projects', namespace, 'vector/backlinks.jsonl');
}
async function readEdges(path) {
    const out = [];
    let stream;
    try {
        stream = createReadStream(path, { encoding: 'utf8' });
    }
    catch (err) {
        if (err.code === 'ENOENT')
            return [];
        throw err;
    }
    try {
        const rl = createInterface({ input: stream, crlfDelay: Infinity });
        for await (const line of rl) {
            const trimmed = line.trim();
            if (!trimmed)
                continue;
            try {
                const obj = JSON.parse(trimmed);
                if (typeof obj.from === 'string' && typeof obj.to === 'string') {
                    out.push(obj);
                }
            }
            catch {
                /* skip malformed line */
            }
        }
    }
    catch (err) {
        if (err.code === 'ENOENT')
            return [];
        throw err;
    }
    return out;
}
/**
 * 1-hop graph expansion source. Loads `backlinks.jsonl`, builds an
 * undirected neighbour index, and for each phase-A candidate collects
 * 1-hop neighbours (both outbound `from→to` and inbound `to→from`).
 * Score = number of distinct candidates that point at the neighbour
 * (more votes = stronger signal). Excludes the candidates themselves.
 *
 * Missing file → empty (fresh project, no backlinks built).
 */
export async function backlinksSource(opts) {
    const start = performance.now();
    const path = backlinksPath(opts.root, opts.namespace);
    const candidateSet = new Set(opts.candidateAtomIds);
    if (candidateSet.size === 0) {
        return {
            source: 'backlinks',
            hits: [],
            latencyMs: Math.round(performance.now() - start),
        };
    }
    let edges;
    try {
        edges = await readEdges(path);
    }
    catch (err) {
        const msg = err instanceof Error ? err.message : String(err ?? 'unknown error');
        return {
            source: 'backlinks',
            hits: [],
            latencyMs: Math.round(performance.now() - start),
            error: msg,
        };
    }
    // For each candidate, collect the set of distinct neighbours via either
    // direction. Then score by the count of contributing candidates.
    const neighbourVoters = new Map();
    for (const edge of edges) {
        if (candidateSet.has(edge.from)) {
            // outbound: candidate.from points at edge.to
            if (!candidateSet.has(edge.to)) {
                let voters = neighbourVoters.get(edge.to);
                if (!voters) {
                    voters = new Set();
                    neighbourVoters.set(edge.to, voters);
                }
                voters.add(edge.from);
            }
        }
        if (candidateSet.has(edge.to)) {
            // inbound: edge.from points at candidate.to → expand to edge.from
            if (!candidateSet.has(edge.from)) {
                let voters = neighbourVoters.get(edge.from);
                if (!voters) {
                    voters = new Set();
                    neighbourVoters.set(edge.from, voters);
                }
                voters.add(edge.to);
            }
        }
    }
    const ranked = Array.from(neighbourVoters.entries())
        .map(([atomId, voters]) => ({ atomId, score: voters.size }))
        .sort((a, b) => {
        if (b.score !== a.score)
            return b.score - a.score;
        return a.atomId < b.atomId ? -1 : 1;
    });
    const hits = ranked.slice(0, opts.topK).map((r, i) => ({
        atomId: r.atomId,
        rank: i + 1,
        source: 'backlinks',
    }));
    return {
        source: 'backlinks',
        hits,
        latencyMs: Math.round(performance.now() - start),
    };
}
