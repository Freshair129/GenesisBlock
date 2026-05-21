import { SymbolStore } from '../../symbols/store/sqlite.js';
import { dbPath, graphExists } from '../../symbols/util.js';
/**
 * A read-only typed interface to the underlying symbol-graph database.
 * Used by PROTO validators to ensure they cannot mutate the graph,
 * and allows easy mocking in tests without bringing in better-sqlite3.
 */
export class SymbolGraphReader {
    store = null;
    /**
     * Opens the symbol graph database for the given repository root.
     * Returns true if successful, false if the graph does not exist or failed to open.
     */
    open(repoRoot) {
        if (!graphExists(repoRoot)) {
            return false;
        }
        this.store = new SymbolStore();
        try {
            this.store.open(dbPath(repoRoot));
            return true;
        }
        catch {
            this.store = null;
            return false;
        }
    }
    /**
     * Closes the underlying database connection gracefully.
     */
    close() {
        if (this.store) {
            try {
                this.store.close();
            }
            catch {
                // ignore
            }
            this.store = null;
        }
    }
    /**
     * Look up a single symbol by its ID.
     */
    getSymbol(id) {
        return this.store?.getSymbol(id) ?? null;
    }
    /**
     * Get all outgoing edges from a specific source symbol ID.
     */
    getOutgoingEdges(srcId, types) {
        return this.store?.getOutgoingEdges(srcId, types) ?? [];
    }
    /**
     * Perform a depth-limited BFS to find neighbors.
     */
    getNeighbors(id, depth, types) {
        return this.store?.getNeighbors(id, depth, types) ?? { nodes: [], edges: [] };
    }
    /**
     * Get all symbols in the graph.
     */
    allSymbols() {
        return this.store?.allSymbols() ?? [];
    }
    /**
     * Get all edges in the graph.
     */
    allEdges() {
        return this.store?.allEdges() ?? [];
    }
}
