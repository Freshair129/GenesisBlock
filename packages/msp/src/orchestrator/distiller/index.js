import { cleanEpisodes } from './pillar-clean.js';
import { synthesizeNarrative, synthesizeIdentity } from './pillar-summary.js';
import { writeNarrativeAtom, writeBeliefAtom } from './pillar-relation.js';
import { readMemoryCounters, updateMemoryCounters } from '../../memory/counters.js';
import { initMemoryStore } from '../../memory/init.js';
import { loadUndistilledEpisodes, loadUndistilledNarratives } from './loader.js';
/**
 * Main orchestrator for memory distillation (Phase B & C).
 *
 * Implements the 8-8-8 protocol:
 * - 8 Sessions -> 1 Narrative (Tier 2)
 * - 8 Narratives -> 1 Identity Belief (Tier 3)
 */
export async function distill(opts) {
    const root = opts.root ?? process.cwd();
    const namespace = opts.namespace;
    const result = {
        episodesProcessed: 0,
        narrativesCreated: 0,
        beliefsRevised: 0,
        errors: []
    };
    try {
        await initMemoryStore(root, namespace);
        const counters = await readMemoryCounters(root, namespace);
        // --- Phase B: Session -> Narrative ---
        const sessionThreshold = 8;
        if (counters.session_seq >= sessionThreshold || opts.force) {
            const rawEpisodes = await loadUndistilledEpisodes({ root, namespace, limit: 50 });
            if (rawEpisodes.length > 0) {
                const cleaned = cleanEpisodes(rawEpisodes);
                result.episodesProcessed = cleaned.length;
                if (cleaned.length > 0 && opts.llm) {
                    console.log(`[distill] Tier 2: synthesizing narrative from ${cleaned.length} episodes...`);
                    const summary = await synthesizeNarrative(cleaned, opts.llm, opts.llmTimeoutMs);
                    const narrativeId = `NARRATIVE--${Date.now()}`;
                    const narrative = {
                        id: narrativeId,
                        namespace,
                        created_at: new Date().toISOString(),
                        domain: 'meta',
                        epistemic_state: 'confirmed',
                        confidence: 0.8,
                        encoding_level: 'L2',
                        source_episodes: cleaned.map(ep => ({
                            id: 'EPISODE--MOCK',
                            session_id: ep.sessionId,
                            encoding_level: ep.encoding_level || 'L2'
                        })),
                        content: summary
                    };
                    if (!opts.dryRun) {
                        await writeNarrativeAtom(root, narrative);
                        await updateMemoryCounters(root, namespace, 'core');
                        result.narrativesCreated = 1;
                    }
                }
            }
        }
        // --- Phase C: Narrative -> Identity ---
        // Re-read counters after Phase B update
        const updatedCounters = await readMemoryCounters(root, namespace);
        const coreThreshold = 8;
        if (updatedCounters.core_seq >= coreThreshold || opts.force) {
            const narratives = await loadUndistilledNarratives({ root, namespace, limit: 50 });
            if (narratives.length > 0 && opts.llm) {
                console.log(`[distill] Tier 3: synthesizing identity from ${narratives.length} narratives...`);
                const identityResult = await synthesizeIdentity(narratives, opts.llm, opts.llmTimeoutMs);
                if (!opts.dryRun) {
                    for (const b of identityResult.beliefs) {
                        const beliefId = `BELIEF--${Math.random().toString(36).substring(2, 9).toUpperCase()}`;
                        const belief = {
                            id: beliefId,
                            statement: b.belief,
                            confidence: b.confidence,
                            epistemic_state: 'confirmed',
                            source_narratives: b.evidence_cite,
                            first_observed_at: new Date().toISOString(),
                            times_confirmed: 1,
                            times_contested: 0
                        };
                        await writeBeliefAtom(root, belief);
                        result.beliefsRevised++;
                    }
                    await updateMemoryCounters(root, namespace, 'sphere');
                }
            }
        }
    }
    catch (err) {
        const msg = err.message;
        console.error(`[distill] error: ${msg}`);
        result.errors.push(msg);
    }
    return result;
}
