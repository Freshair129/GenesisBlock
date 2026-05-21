const PREDICATES = [
    'implements',
    'references',
    'used_by',
    'contradicts',
    'supersedes',
    'partially_supersedes',
    'superseded_by',
    'partially_superseded_by',
    'resolves',
];
export function edgesFromAtom(fm) {
    const id = fm.id;
    if (typeof id !== 'string' || id.length === 0)
        return [];
    const crosslinks = fm.crosslinks;
    if (!crosslinks || typeof crosslinks !== 'object' || Array.isArray(crosslinks)) {
        return [];
    }
    const out = [];
    for (const predicate of PREDICATES) {
        const list = crosslinks[predicate];
        if (!Array.isArray(list))
            continue;
        for (const target of list) {
            if (typeof target === 'string' && target.length > 0) {
                out.push({ from: id, to: target, type: predicate });
            }
        }
    }
    return out;
}
export function sortEdges(edges) {
    return [...edges].sort((a, b) => {
        if (a.from !== b.from)
            return a.from < b.from ? -1 : 1;
        if (a.to !== b.to)
            return a.to < b.to ? -1 : 1;
        if (a.type !== b.type)
            return a.type < b.type ? -1 : 1;
        return 0;
    });
}
