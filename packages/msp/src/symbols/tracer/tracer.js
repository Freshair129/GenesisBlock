export class SymbolTracer {
    store;
    constructor(store) {
        this.store = store;
    }
    async trace(seedId, opts) {
        const maxDepth = opts.maxDepth ?? 8;
        const direction = opts.direction;
        const edgeTypes = ['calls', 'handles'];
        const paths = [];
        const visited = new Set();
        const seed = this.store.getSymbol(seedId);
        if (!seed)
            return [];
        const walk = (current, depth, currentPathNodes, currentPathEdges) => {
            const isCycle = currentPathNodes.some(n => n.id === current.id);
            const node = { ...current, depth, isCycle };
            const newPathNodes = [...currentPathNodes, node];
            if (isCycle || depth >= maxDepth) {
                paths.push({ nodes: newPathNodes, edges: currentPathEdges });
                return;
            }
            const edges = direction === 'down'
                ? this.store.getOutgoingEdges(current.id, edgeTypes)
                : this.store.getIncomingEdges(current.id, edgeTypes);
            if (edges.length === 0) {
                paths.push({ nodes: newPathNodes, edges: currentPathEdges });
                return;
            }
            for (const edge of edges) {
                const nextId = direction === 'down' ? edge.dst_id : edge.src_id;
                const nextSymbol = this.store.getSymbol(nextId);
                const traceEdge = { ...edge, depth: depth + 1 };
                const newPathEdges = [...currentPathEdges, traceEdge];
                if (!nextSymbol) {
                    // Unresolved or missing symbol
                    paths.push({
                        nodes: [...newPathNodes, { id: nextId, name: 'unknown', kind: 'function', file: 'unknown', start_line: 0, end_line: 0, exported: false, parent_id: null, signature: null, community_id: null, created_at: '', depth: depth + 1, isCycle: false }],
                        edges: newPathEdges
                    });
                    continue;
                }
                walk(nextSymbol, depth + 1, newPathNodes, newPathEdges);
            }
        };
        walk(seed, 0, [], []);
        return paths;
    }
}
