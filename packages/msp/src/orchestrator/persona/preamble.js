import { loadIdentityBeliefs } from '../../memory/identity.js';
/**
 * Pillar 5: INFECT (Identity Evolution)
 * Generates a system preamble from distilled Tier 3 beliefs to inform
 * the agent's persona.
 */
export async function generateIdentityPreamble() {
    try {
        const beliefs = await loadIdentityBeliefs();
        if (beliefs.length === 0) {
            return ''; // No beliefs yet; standard generic persona applies
        }
        // Sort by confidence (highest first)
        const sorted = [...beliefs].sort((a, b) => b.confidence - a.confidence);
        const lines = [
            '### LONG-TERM IDENTITY BELIEFS',
            'The following beliefs have been distilled from your interaction history and should guide your behavior:',
            '',
            ...sorted.map(b => `- ${b.statement} (Confidence: ${Math.round(b.confidence * 100)}%)`),
            ''
        ];
        return lines.join('\n');
    }
    catch (err) {
        console.error(`[persona] failed to generate preamble: ${err.message}`);
        return '';
    }
}
