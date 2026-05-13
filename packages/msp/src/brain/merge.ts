import type { BrainHit } from './types.js';

export function merge(hits: BrainHit[]): BrainHit[] {
  const byId = new Map<string, BrainHit[]>();
  const order: string[] = [];
  for (const hit of hits) {
    const group = byId.get(hit.atom.id);
    if (group) {
      group.push(hit);
    } else {
      byId.set(hit.atom.id, [hit]);
      order.push(hit.atom.id);
    }
  }

  const kept = new Set<BrainHit>();
  for (const id of order) {
    const group = byId.get(id)!;
    const hasProject = group.some((h) => h.source === 'project');
    for (const h of group) {
      if (hasProject && h.source === 'global') continue;
      kept.add(h);
    }
  }

  return hits.filter((h) => kept.has(h));
}
