import { performance } from 'node:perf_hooks';
import { join } from 'node:path';
import { readFile, readdir } from 'node:fs/promises';
import { DEFAULT_PER_SOURCE_TIMEOUTS, } from '../types.js';
import { mspHome } from '../../../lib/msp-home.js';
/**
 * Retrieval source for Identity beliefs (Tier 3).
 * Reads from global MSP_HOME/memory/identity/ across all domains.
 */
export async function identitySource(opts) {
    const start = performance.now();
    const timeoutMs = opts.perSourceTimeouts?.identity ?? DEFAULT_PER_SOURCE_TIMEOUTS.identity;
    const home = mspHome();
    const identityDir = join(home, 'memory', 'identity');
    try {
        const files = await readdir(identityDir);
        const jsonFiles = files.filter(f => f.endsWith('.json'));
        const hits = [];
        const needle = opts.query.toLowerCase();
        for (const file of jsonFiles) {
            if (file === 'identity_audit.jsonl')
                continue;
            const filePath = join(identityDir, file);
            try {
                const raw = await readFile(filePath, 'utf8');
                const data = JSON.parse(raw);
                // Data is a domain-level belief object
                if (data.beliefs && Array.isArray(data.beliefs)) {
                    for (const belief of data.beliefs) {
                        if (belief.epistemic_state === 'deprecated')
                            continue;
                        const text = belief.statement;
                        if (text.toLowerCase().includes(needle)) {
                            hits.push({
                                atomId: belief.belief_id,
                                rank: 1,
                                snippet: text,
                                source: 'identity',
                                memoryTier: 'identity',
                                attributes: {
                                    domain: data.domain,
                                    epistemic_state: belief.epistemic_state,
                                    confidence: belief.confidence
                                }
                            });
                        }
                    }
                }
            }
            catch {
                continue;
            }
            if (performance.now() - start > timeoutMs)
                break;
        }
        const rankedHits = hits.map((h, i) => ({ ...h, rank: i + 1 }));
        return {
            source: 'identity',
            hits: rankedHits,
            latencyMs: Math.round(performance.now() - start)
        };
    }
    catch (err) {
        return {
            source: 'identity',
            hits: [],
            latencyMs: Math.round(performance.now() - start),
            error: err.message
        };
    }
}
