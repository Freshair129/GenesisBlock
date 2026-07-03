import GenesisDbNative from './NativeGenesisDb';
import {
  ContextPackage,
  GenesisDBError,
  HybridSearchInput,
  NeighborOutput,
  NodeInput,
  NodeOutput,
} from './types';

export * from './types';

/**
 * Embedded GenesisBlockDB for React Native (MARK XVI Phase B-3). One
 * instance owns one native handle. See docs/SPEC--MOBILE-SDK.md §B-3.
 *
 * ```ts
 * const db = await GenesisDB.open(`${RNFS.DocumentDirectoryPath}/genesisdb`);
 * const node = await db.addNode({ labels: ['Person'] });
 * const ctx = await db.retrieveContext(node.id, 'H1');
 * await db.close();
 * ```
 */
export class GenesisDB {
  private dbId: number | null;

  private constructor(dbId: number) {
    this.dbId = dbId;
  }

  static async open(path: string): Promise<GenesisDB> {
    const dbId = await GenesisDbNative.open(path);
    return new GenesisDB(dbId);
  }

  async addNode(input: NodeInput): Promise<NodeOutput> {
    const result = await GenesisDbNative.addNode(this.id(), JSON.stringify(input));
    return JSON.parse(result) as NodeOutput;
  }

  async search(input: HybridSearchInput): Promise<NeighborOutput[]> {
    const result = await GenesisDbNative.search(this.id(), JSON.stringify(input));
    return JSON.parse(result) as NeighborOutput[];
  }

  /** Execute a raw HQL query string. The result shape varies by command
   * (SEARCH/TRAVERSE/MATCH/CONTEXT), so it is returned untyped. */
  async executeHql(query: string): Promise<unknown> {
    const result = await GenesisDbNative.executeHql(this.id(), query);
    return JSON.parse(result);
  }

  async retrieveContext(
    targetId: string,
    tier: string = 'H1',
    budget?: number,
    fuzzy: boolean = false
  ): Promise<ContextPackage> {
    const payload = JSON.stringify({ target_id: targetId, tier, budget, fuzzy });
    const result = await GenesisDbNative.retrieveContext(this.id(), payload);
    return JSON.parse(result) as ContextPackage;
  }

  async flushIndex(): Promise<void> {
    await GenesisDbNative.flushIndex(this.id());
  }

  async close(): Promise<void> {
    if (this.dbId !== null) {
      const dbId = this.dbId;
      this.dbId = null;
      await GenesisDbNative.close(dbId);
    }
  }

  private id(): number {
    if (this.dbId === null) {
      throw new GenesisDBError('GenesisDB handle is closed');
    }
    return this.dbId;
  }
}

export default GenesisDB;
