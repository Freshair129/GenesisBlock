import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { mspHome } from '../lib/msp-home.js';
/**
 * Loads all confirmed identity beliefs across all domains.
 */
export async function loadIdentityBeliefs() {
    const home = mspHome();
    const identityDir = join(home, 'memory', 'identity');
    let files;
    try {
        files = await readdir(identityDir);
    }
    catch (err) {
        if (err.code === 'ENOENT')
            return [];
        throw err;
    }
    const jsonFiles = files.filter(f => f.endsWith('.json') && f !== 'identity_audit.jsonl');
    const allBeliefs = [];
    for (const file of jsonFiles) {
        const filePath = join(identityDir, file);
        try {
            const raw = await readFile(filePath, 'utf8');
            const data = JSON.parse(raw);
            if (data.beliefs && Array.isArray(data.beliefs)) {
                for (const b of data.beliefs) {
                    if (b.epistemic_state === 'confirmed') {
                        allBeliefs.push({
                            id: b.belief_id,
                            statement: b.statement,
                            confidence: b.confidence,
                            epistemic_state: b.epistemic_state,
                            source_narratives: b.source_narratives || [],
                            first_observed_at: b.first_observed_at,
                            times_confirmed: b.times_confirmed || 1,
                            times_contested: b.times_contested || 0
                        });
                    }
                }
            }
        }
        catch (err) {
            console.warn(`[identity] failed to load beliefs from ${file}: ${err.message}`);
        }
    }
    return allBeliefs;
}
