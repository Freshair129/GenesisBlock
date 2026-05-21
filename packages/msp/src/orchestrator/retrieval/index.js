import { performance } from 'node:perf_hooks';
import { rrfFuse } from './fusion.js';
import { backlinksSource } from './sources/backlinks.js';
import { graphSource } from './sources/graph.js';
import { episodicSource } from './sources/episodic.js';
import { obsidianSource } from './sources/obsidian.js';
import { vectorSource } from './sources/vector.js';
import { grepSource } from './sources/grep.js';
import { narrativeSource } from './sources/narrative.js';
import { identitySource } from './sources/identity.js';
import { DEFAULT_NAMESPACE, DEFAULT_PER_SOURCE_TIMEOUTS, DEFAULT_RRF_K, DEFAULT_TOP_K, DEFAULT_TOTAL_TIMEOUT_MS, DEFAULT_WEIGHTS, } from './types.js';
import { enforcePolicy } from '../../policy/pep.js';
import { makeResource } from '../../policy/types.js';
/**
 * Multi-source retrieval orchestrator.
 *
 * Phases:
 * A. Resolve timeout/weights.
 * B. Run sources in parallel (vector, text, episodic, graph, narrative, identity).
 * C. RRF Fuse.
 * D. Re-rank (M10c).
 * E. Enforce UCF Policy (PEP).
 */
export async function recall(opts) {
    const overallStart = performance.now();
    const root = opts.root ?? process.cwd();
    const topK = opts.topK ?? DEFAULT_TOP_K;
    const totalTimeoutMs = opts.timeoutMs ?? DEFAULT_TOTAL_TIMEOUT_MS;
    const namespace = opts.namespace ?? DEFAULT_NAMESPACE;
    // UCF attributes
    const subject = opts.subject ?? {
        kind: 'user',
        id: 'default',
        attributes: {
            tier: 'T1',
            clearance: 'public',
            namespace
        }
    };
    const action = 'read';
    const context = opts.context ?? {
        time: new Date(),
        origin: 'internal',
        trace_id: `recall-${Date.now()}`
    };
    // Phase A: weights & timeouts.
    const rrfK = opts.rrfK ?? DEFAULT_RRF_K;
    const weights = { ...DEFAULT_WEIGHTS, ...opts.weights };
    const perSourceTimeouts = { ...DEFAULT_PER_SOURCE_TIMEOUTS, ...opts.perSourceTimeouts };
    // Scale per-source timeouts if total is constrained.
    if (opts.timeoutMs && opts.timeoutMs !== DEFAULT_TOTAL_TIMEOUT_MS) {
        const scale = opts.timeoutMs / DEFAULT_TOTAL_TIMEOUT_MS;
        for (const k of Object.keys(perSourceTimeouts)) {
            perSourceTimeouts[k] *= scale;
        }
    }
    // Source options normalized for type compatibility
    const sourceOpts = {
        ...opts,
        root,
        topK,
        namespace,
        timeoutMs: totalTimeoutMs,
        candidateAtomIds: [],
    };
    // Phase B: run sources.
    const tasks = [
        vectorSource(sourceOpts),
        obsidianSource(sourceOpts),
        grepSource(sourceOpts),
        episodicSource(sourceOpts),
        backlinksSource(sourceOpts),
        narrativeSource(sourceOpts),
        identitySource(sourceOpts),
    ];
    const results = await Promise.all(tasks);
    const graphRes = await graphSource(sourceOpts);
    const vectorRes = results.find((r) => r.source === 'gks-vector');
    const obsidianRes = results.find((r) => r.source === 'obsidian-text');
    const episodicRes = results.find((r) => r.source === 'episodic');
    // Phase C: fuse.
    const fuseStart = performance.now();
    const allResults = [...results, graphRes];
    const fusedHits = rrfFuse(allResults, { k: rrfK, weights, topK: opts.rerank ? (opts.rerankLimit ?? 30) : topK });
    const fusionMs = performance.now() - fuseStart;
    // Phase D: Re-rank (M10c)
    let rerankMs = 0;
    let finalHits = fusedHits;
    if (opts.rerank && opts.reranker && fusedHits.length > 0) {
        const rerankStart = performance.now();
        try {
            const itemsToRerank = fusedHits.map(h => ({
                id: h.atomId,
                text: h.snippet || '',
            }));
            const reranked = await opts.reranker.rerank(opts.query, itemsToRerank);
            const scoreMap = new Map(reranked.map(r => [r.id, r.score]));
            finalHits = fusedHits.map(h => ({
                ...h,
                score: scoreMap.get(h.atomId) ?? h.score,
            })).sort((a, b) => b.score - a.score)
                .slice(0, topK)
                .map((h, i) => ({ ...h, rank: i + 1 }));
        }
        catch (err) {
            console.warn(`[retrieval] rerank failed: ${err.message}`);
            allResults.push({ source: 'identity', hits: [], latencyMs: 0, error: `rerank: ${err.message}` });
            finalHits = fusedHits.slice(0, topK);
        }
        rerankMs = Math.round(performance.now() - rerankStart);
    }
    else {
        finalHits = fusedHits.slice(0, topK);
    }
    // Phase E: Enforce Policy (PEP)
    const filteredHits = [];
    const pepOpts = { root, subject, action, context };
    for (const hit of finalHits) {
        const attributes = {
            ...(hit.attributes ?? {}),
            body: hit.snippet ?? null,
        };
        const resource = makeResource('atom', hit.atomId, {}, attributes);
        const { permitted } = await enforcePolicy(resource, pepOpts);
        if (permitted) {
            filteredHits.push(hit);
        }
    }
    const semanticAvailable = !!opts.embedder && !!opts.vectorBackend && !vectorRes.error;
    const obsidianAvailable = opts.obsidian?.mode === 'rest';
    const rerankAvailable = !!opts.reranker;
    const result = {
        hits: filteredHits,
        semantic_available: semanticAvailable,
        obsidian_available: obsidianAvailable,
        rerank_available: rerankAvailable,
        fallback_reasons: collectFallbackReasons(allResults),
        timings: {
            vector: vectorRes.latencyMs,
            obsidian: obsidianRes.latencyMs,
            episodic: episodicRes.latencyMs,
            graph: graphRes.latencyMs,
            narrative: results.find(r => r.source === 'narrative')?.latencyMs,
            identity: results.find(r => r.source === 'identity')?.latencyMs,
            fusion: Math.round(fusionMs),
            rerank: rerankMs,
        },
    };
    return result;
}
function collectFallbackReasons(results) {
    const reasons = [];
    for (const r of results) {
        if (r.error)
            reasons.push(`${r.source}: error: ${r.error}`);
        if (r.skipped)
            reasons.push(`${r.source}: skipped: ${r.skipped}`);
    }
    return reasons;
}
