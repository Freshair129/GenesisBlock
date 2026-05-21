import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
/**
 * Pillar 4: RELATION (Identity Variant)
 * Generates the GKS atom for a distilled Identity belief.
 */
export async function writeBeliefAtom(root, belief) {
    const content = `---
id: ${belief.id}
phase: 1
type: belief
status: stable
vault_id: default
tier: process
source_type: learned
epistemic_state: ${belief.epistemic_state}
confidence: ${belief.confidence}
title: BELIEF — ${belief.id}
tags: [msp, memory, identity, belief, distillation]
aliases: [Identity Memory, BELIEF]
cluster: memory
role: Distilled long-term belief
crosslinks:
  references:
${belief.source_narratives.map(id => `    - ${id}`).join('\n')}
created_at: ${new Date().toISOString()}
---

# BELIEF — ${belief.id}

## Belief

${belief.statement}

## Evidence

- Supported by ${belief.source_narratives.length} narratives.
- First observed at: ${belief.first_observed_at}
- Times confirmed: ${belief.times_confirmed}
- Times contested: ${belief.times_contested}

## Source

- Distilled from hierarchical 8-8-8 memory synthesis.
`;
    const filePath = join(root, 'gks', 'identity', `${belief.id}.md`);
    // Note: Directory is guaranteed to exist by initMemoryStore.
    await writeFile(filePath, content, 'utf8');
    return filePath;
}
/**
 * Pillar 4: RELATION
 * Generates the GKS atom for the narrative unit and establishes cross-links.
 */
export async function writeNarrativeAtom(root, narrative) {
    const content = `---
id: ${narrative.id}
phase: 1
type: narrative
status: stable
vault_id: default
tier: process
source_type: learned
title: NARRATIVE — ${narrative.id} — ${narrative.domain} synthesis
tags: [msp, memory, narrative, distillation, ${narrative.domain}]
aliases: [Core Memory, NARRATIVE]
cluster: memory
role: Cross-session pattern / arc
crosslinks:
  references:
${narrative.source_episodes.map(ep => `    - ${ep.id}`).join('\n')}
created_at: ${narrative.created_at}
---

# NARRATIVE — ${narrative.id}

## Summary

${narrative.content.summary}

## Patterns

${narrative.content.patterns_observed.map(p => `- ${p}`).join('\n')}

## Source

- Distilled from ${narrative.source_episodes.length} episodes.
`;
    const filePath = join(root, 'gks', 'narrative', `${narrative.id}.md`);
    // Note: Directory is guaranteed to exist by initMemoryStore.
    await writeFile(filePath, content, 'utf8');
    return filePath;
}
