import {
  VectorBackend,
  VectorBackendAddItem,
} from './backend.js'
import type {
  VectorDoc,
  VectorHit,
  VectorManifest,
  VectorMetadata,
  VectorSearchOptions,
} from '../types.js'
import type { Embedder } from './embedder.js'

export interface BridgeAdapterOptions {
  endpoint: string;
  name: string;
  embedder: Embedder;
}

/**
 * GKS VectorBackend that communicates with the msp-bridge Obsidian plugin.
 * Acts as a proxy for the pgvector store managed by the plugin.
 */
export class BridgeAdapter implements VectorBackend {
  readonly name: string;
  readonly embedder: Embedder;
  private readonly endpoint: string;

  constructor(opts: BridgeAdapterOptions) {
    this.name = opts.name;
    this.embedder = opts.embedder;
    this.endpoint = opts.endpoint;
  }

  async load(): Promise<void> {
    try {
      const res = await fetch(`${this.endpoint}/api/status`);
      if (!res.ok) throw new Error(`Bridge endpoint unreachable: ${res.statusText}`);
      const data = (await res.json()) as any;
      console.log(`[bridge-adapter] connected to ${data.name} v${data.version}`);
    } catch (err) {
      console.warn(`[bridge-adapter] failed to load status: ${(err as Error).message}`);
    }
  }

  size(): number {
    // Ideally this would be an async call to the API, but the interface is sync.
    // For now, we return 0 and rely on search results.
    return 0;
  }

  getManifest(): VectorManifest {
    return {
      embedder_model: this.embedder.model,
      dimension: this.embedder.dimension,
      doc_count: 0,
      last_updated: new Date().toISOString(),
      file_hashes: {},
    };
  }

  async add(text: string, metadata: VectorMetadata, opts?: { id?: string; chunkId?: string; source?: string }): Promise<VectorDoc> {
    const vector = await this.embedder.embed(text);
    return this.addWithVector(text, vector, metadata, opts);
  }

  async addWithVector(text: string, vector: number[], metadata: VectorMetadata, opts?: { id?: string; chunkId?: string; source?: string }): Promise<VectorDoc> {
    const res = await fetch(`${this.endpoint}/api/insert`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: opts?.id,
        text,
        vector,
        metadata: { ...metadata, source: opts?.source },
        store: this.name
      }),
    });

    if (!res.ok) {
      throw new Error(`Bridge insert failed: ${res.statusText}`);
    }

    const data = (await res.json()) as any;
    return data.doc;
  }

  async addBatch(items: VectorBackendAddItem[]): Promise<VectorDoc[]> {
    const texts = items.map(i => i.text);
    const vectors = await this.embedder.embedBatch(texts);
    
    const payload = items.map((item, i) => ({
      id: item.id,
      text: item.text,
      vector: vectors[i],
      metadata: { ...item.metadata, source: item.source },
      store: this.name
    }));

    const res = await fetch(`${this.endpoint}/api/insert-batch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ items: payload }),
    });

    if (!res.ok) {
      throw new Error(`Bridge batch insert failed: ${res.statusText}`);
    }

    const data = (await res.json()) as any;
    return data.docs;
  }

  async search(query: string | number[], opts?: VectorSearchOptions): Promise<VectorHit[]> {
    const vector = typeof query === 'string' ? await this.embedder.embed(query) : query;

    const res = await fetch(`${this.endpoint}/api/semantic-search`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ 
        vector, 
        limit: opts?.topK, 
        threshold: opts?.scoreThreshold,
        store: this.name
      }),
    });

    if (!res.ok) {
      throw new Error(`Bridge search failed: ${res.statusText}`);
    }

    const data = (await res.json()) as any;
    return data.hits;
  }

  async patchMetadata(id: string, patch: Partial<VectorMetadata>): Promise<VectorDoc | null> {
    const res = await fetch(`${this.endpoint}/api/patch-metadata`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id, patch, store: this.name }),
    });

    if (!res.ok) return null;
    const data = (await res.json()) as any;
    return data.doc;
  }

  async patchMetadataMany(patches: ReadonlyArray<{ id: string; patch: Partial<VectorMetadata> }>): Promise<Array<VectorDoc | null>> {
    const res = await fetch(`${this.endpoint}/api/patch-metadata-many`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ patches, store: this.name }),
    });

    if (!res.ok) return [];
    const data = (await res.json()) as any;
    return data.docs;
  }

  async get(id: string): Promise<VectorDoc | undefined> {
    const res = await fetch(`${this.endpoint}/api/get/${id}?store=${this.name}`);
    if (!res.ok) return undefined;
    const data = (await res.json()) as any;
    return data.doc;
  }

  async clear(): Promise<void> {
    await fetch(`${this.endpoint}/api/clear?store=${this.name}`, { method: 'DELETE' });
  }

  listDocs(): readonly VectorDoc[] {
    // Synchronous listing not supported via remote bridge
    return [];
  }
}
