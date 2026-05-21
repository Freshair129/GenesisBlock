export function merge(hits) {
    const byId = new Map();
    const order = [];
    for (const hit of hits) {
        const group = byId.get(hit.atom.id);
        if (group) {
            group.push(hit);
        }
        else {
            byId.set(hit.atom.id, [hit]);
            order.push(hit.atom.id);
        }
    }
    const kept = new Set();
    for (const id of order) {
        const group = byId.get(id);
        const hasProject = group.some((h) => h.source === 'project');
        for (const h of group) {
            if (hasProject && h.source === 'global')
                continue;
            kept.add(h);
        }
    }
    return hits.filter((h) => kept.has(h));
}
