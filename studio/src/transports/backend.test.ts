import { describe, expect, it } from 'vitest';
import { createRemoteTransport } from './backend';

const capabilities = {
  protocol_version: '1',
  engine_version: '0.2.0',
  mode: 'remote',
  read_features: ['status.read', 'graph.scene.read'],
  write_features: [],
  auth_features: ['api-key'],
  limits: {
    initial_scene_nodes: 500,
    scene_node_ceiling: 1_000,
    scene_edge_ceiling: 3_000,
    expansion_nodes: 100,
  },
  consistency: 'stable-frontier',
};

function json(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

describe('remote StudioTransport', () => {
  it('negotiates capabilities with bearer authentication before use', async () => {
    const calls: Array<{ url: string; authorization: string | null }> = [];
    const fetcher = (async (input: string | URL | Request, init?: RequestInit) => {
      calls.push({
        url: String(input),
        authorization: new Headers(init?.headers).get('authorization'),
      });
      return json(capabilities);
    }) as typeof fetch;

    const transport = await createRemoteTransport('https://db.example/', 'secret', fetcher);

    expect((await transport.getCapabilities()).protocolVersion).toBe('1');
    expect(calls).toEqual([
      { url: 'https://db.example/v1/studio/capabilities', authorization: 'Bearer secret' },
    ]);
  });

  it('normalizes bounded backend graph DTOs without embeddings', async () => {
    const fetcher = (async (input: string | URL | Request) => {
      const url = String(input);
      if (url.endsWith('/v1/studio/capabilities')) return json(capabilities);
      if (url.includes('/v1/studio/graph?')) {
        return json({
          scene_id: 'scene-1',
          frontier: 42,
          nodes: [{
            id: 'n1', label: 'Node one', labels: ['Memory'], collection: 'default',
            valid_from: '2026-07-01T00:00:00Z', valid_to: null, caused_by: null, impact: 2,
          }],
          edges: [],
          groups: ['Memory'],
          truncated: false,
          continuation: null,
          warnings: [],
        });
      }
      return json({}, 404);
    }) as typeof fetch;

    const transport = await createRemoteTransport('https://db.example', '', fetcher);
    const scene = await transport.loadGraphScene({ limit: 10 });

    expect(scene.frontier).toBe(42);
    expect(scene.data.nodes[0]).toMatchObject({
      id: 'n1',
      group: 'Memory',
      vectorCollection: 'default',
    });
    expect(JSON.stringify(scene)).not.toContain('embedding');
  });

  it('executes only a versioned named-query contract over the relational endpoint', async () => {
    let body = '';
    const fetcher = (async (input: string | URL | Request, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/v1/studio/capabilities')) return json(capabilities);
      if (url.endsWith('/v1/relational/query')) {
        body = String(init?.body);
        return json([{ id: 'row-1' }]);
      }
      return json({}, 404);
    }) as typeof fetch;

    const transport = await createRemoteTransport('https://db.example', '', fetcher);
    const result = await transport.executeNamedQuery({
      namespace: 'notes',
      schemaVersion: 3,
      queryName: 'note_by_id',
      parameters: { note_id: 'row-1' },
      limit: 1,
    });

    expect(result.data).toEqual([{ id: 'row-1' }]);
    expect(JSON.parse(body)).toEqual({
      namespace: 'notes',
      schema_version: 3,
      query_name: 'note_by_id',
      parameters: { note_id: 'row-1' },
      limit: 1,
    });
  });
});
