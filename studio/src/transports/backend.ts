import { invoke } from '@tauri-apps/api/core';
import type {
  EntityInspection,
  GraphScene,
  GraphSceneEdge,
  GraphSceneNode,
  RelationalSchemaSummary,
  NamedQueryRequest,
  SceneRequest,
  StudioCapabilities,
  StudioCollection,
  StudioEnvelope,
  StudioStatus,
  StudioTransport,
} from '../domain/contracts';

interface BackendCapabilities {
  protocol_version: string;
  engine_version: string;
  mode: 'local' | 'remote';
  read_features: StudioCapabilities['readFeatures'];
  write_features: string[];
  auth_features: string[];
  limits: {
    initial_scene_nodes: number;
    scene_node_ceiling: number;
    scene_edge_ceiling: number;
    expansion_nodes: number;
  };
  consistency: StudioCapabilities['consistency'];
}

interface BackendGraphNode {
  id: string;
  label: string;
  labels: string[];
  collection: string | null;
  valid_from: string;
  valid_to: string | null;
  caused_by: string | null;
  impact: number | null;
}

interface BackendGraphEdge {
  id: string;
  from: string;
  to: string;
  relation: string;
  valid_from: string;
  valid_to: string | null;
  caused_by: string | null;
}

interface BackendGraphScene {
  scene_id: string;
  frontier: number;
  nodes: BackendGraphNode[];
  edges: BackendGraphEdge[];
  groups: string[];
  truncated: boolean;
  continuation: string | null;
  warnings: string[];
}

interface BackendInspection {
  entity_id: string;
  frontier: number;
  node: BackendGraphNode;
  properties: Record<string, string | number | boolean>;
  incident_edges: BackendGraphEdge[];
  vector_collection: string | null;
  vector_present: boolean;
  index_lag: number;
  availability: EntityInspection['availability'];
}

interface BackendCollection {
  name: string;
  dim: number;
  metric: string;
  count: number;
  index_lag: number;
}

interface BackendSchema {
  namespace: string;
  schema_version: number;
  tables: unknown[];
  named_queries: Array<{
    name: string;
    parameters: Array<{ name: string }>;
    default_limit: number;
    max_limit: number;
  }>;
}

let requestSequence = 0;

function envelope<T>(data: T, frontier: number | null, warnings: string[] = []): StudioEnvelope<T> {
  requestSequence += 1;
  return {
    requestId: `studio-${requestSequence}`,
    frontier,
    generatedAt: new Date().toISOString(),
    truncated: false,
    warnings,
    data,
  };
}

function mapCapabilities(raw: BackendCapabilities): StudioCapabilities {
  return {
    protocolVersion: raw.protocol_version,
    engineVersion: raw.engine_version,
    mode: raw.mode,
    readFeatures: raw.read_features,
    writeFeatures: raw.write_features,
    authFeatures: raw.auth_features,
    limits: {
      initialSceneNodes: raw.limits.initial_scene_nodes,
      sceneNodeCeiling: raw.limits.scene_node_ceiling,
      sceneEdgeCeiling: raw.limits.scene_edge_ceiling,
      expansionNodes: raw.limits.expansion_nodes,
    },
    consistency: raw.consistency,
    unsupportedReasons: {},
  };
}

function nodePosition(index: number, count: number): { x: number; y: number } {
  const angle = index * 2.39996;
  const radius = 3 + 11 * Math.sqrt((index + 1) / Math.max(1, count));
  return { x: Math.cos(angle) * radius, y: Math.sin(angle) * radius };
}

function nodeColor(group: string): string {
  const palette = ['#0e7490', '#b45309', '#be123c', '#4d7c0f', '#6d28d9', '#475569'];
  let hash = 0;
  for (const character of group) hash = (hash * 31 + character.charCodeAt(0)) | 0;
  return palette[Math.abs(hash) % palette.length];
}

function mapNode(node: BackendGraphNode, index: number, count: number): GraphSceneNode {
  const group = node.labels[0] ?? 'Unlabeled';
  const position = nodePosition(index, count);
  return {
    id: node.id,
    label: node.label,
    kind: 'knowledge',
    group,
    x: position.x,
    y: position.y,
    size: 4 + Math.min(5, Math.max(0, node.impact ?? 0)),
    color: nodeColor(group),
    vectorCollection: node.collection,
    validFrom: node.valid_from,
  };
}

function mapEdge(edge: BackendGraphEdge): GraphSceneEdge {
  return {
    id: edge.id,
    source: edge.from,
    target: edge.to,
    relation: edge.relation,
    color: '#a8a29e',
  };
}

function mapScene(raw: BackendGraphScene): StudioEnvelope<GraphScene> {
  const scene = {
    sceneId: raw.scene_id,
    nodes: raw.nodes.map((node, index) => mapNode(node, index, raw.nodes.length)),
    edges: raw.edges.map(mapEdge),
    groups: raw.groups,
    continuation: raw.continuation,
  };
  return {
    ...envelope(scene, raw.frontier, raw.warnings),
    truncated: raw.truncated,
  };
}

function mapInspection(raw: BackendInspection): StudioEnvelope<EntityInspection> {
  const incoming = raw.incident_edges.filter((edge) => edge.to === raw.entity_id).length;
  const outgoing = raw.incident_edges.filter((edge) => edge.from === raw.entity_id).length;
  return envelope(
    {
      entityId: raw.entity_id,
      label: raw.node.label,
      availability: raw.availability,
      relational: raw.properties,
      graph: { incoming, outgoing },
      vector: { collection: raw.vector_collection, nearestScore: null },
      temporal: { validFrom: raw.node.valid_from, causedBy: raw.node.caused_by },
    },
    raw.frontier,
    raw.index_lag > 0 ? [`Vector index lag: ${raw.index_lag}`] : [],
  );
}

function mapCollections(raw: BackendCollection[]): StudioCollection[] {
  return raw.map((collection) => ({
    name: collection.name,
    dimension: collection.dim,
    metric: collection.metric.toLowerCase() === 'cosine' ? 'cosine' : 'l2',
    vectorCount: collection.count,
    indexLag: collection.index_lag,
  }));
}

function mapSchemas(raw: BackendSchema[]): RelationalSchemaSummary[] {
  return raw.map((schema) => ({
    namespace: schema.namespace,
    version: schema.schema_version,
    tables: schema.tables.length,
    namedQueries: schema.named_queries.map((query) => ({
      name: query.name,
      parameters: query.parameters.map((parameter) => parameter.name),
      defaultLimit: query.default_limit,
      maxLimit: query.max_limit,
    })),
  }));
}

function namedQueryRequest(request: NamedQueryRequest): Record<string, unknown> {
  return {
    namespace: request.namespace,
    schema_version: request.schemaVersion,
    query_name: request.queryName,
    parameters: request.parameters,
    limit: request.limit,
  };
}

function graphRequest(request: SceneRequest): Record<string, unknown> {
  return { seed: request.seed, limit: request.limit, direction: 'both' };
}

export async function createLocalTransport(path: string): Promise<StudioTransport> {
  const initial = await invoke<BackendCapabilities>('studio_open_local', { path });
  const capabilities = mapCapabilities(initial);
  return {
    kind: 'local',
    async close() {
      await invoke('studio_close_local');
    },
    async getCapabilities() {
      return capabilities;
    },
    async getStatus() {
      const raw = await invoke<Record<string, number>>('studio_local_status');
      return envelope(
        {
          nodeCount: raw.node_count,
          edgeCount: raw.edge_count,
          collectionCount: raw.collection_count,
          indexLag: raw.index_lag,
          logicalClock: raw.logical_clock,
          memoryUsageMb: raw.memory_usage_mb ?? 0,
        },
        raw.frontier,
      );
    },
    async listCollections() {
      return envelope(mapCollections(await invoke<BackendCollection[]>('studio_local_collections')), null);
    },
    async listRelationalSchemas() {
      return envelope(mapSchemas(await invoke<BackendSchema[]>('studio_local_relational_schemas')), null);
    },
    async executeNamedQuery(request: NamedQueryRequest) {
      const data = await invoke<Record<string, unknown>[]>('studio_local_named_query', {
        request: namedQueryRequest(request),
      });
      return envelope(data, null);
    },
    async executeReadOnlyHql(query: string) {
      const data = await invoke<Record<string, unknown>[]>('studio_local_hql', { query });
      return envelope(data, null);
    },
    async loadGraphScene(request: SceneRequest) {
      return mapScene(await invoke<BackendGraphScene>('studio_local_graph', { request: graphRequest(request) }));
    },
    async expandGraphScene(nodeId: string) {
      return mapScene(await invoke<BackendGraphScene>('studio_local_graph', {
        request: graphRequest({ seed: nodeId, limit: capabilities.limits.expansionNodes }),
      }));
    },
    async inspectEntity(entityId: string) {
      return mapInspection(await invoke<BackendInspection>('studio_local_inspect', { entityId }));
    },
  };
}

type Fetcher = typeof fetch;

export async function createRemoteTransport(
  baseUrl: string,
  token = '',
  fetcher: Fetcher = fetch,
): Promise<StudioTransport> {
  const normalizedBase = baseUrl.replace(/\/$/, '');
  const request = async <T>(path: string, init?: RequestInit): Promise<T> => {
    const headers = new Headers(init?.headers);
    headers.set('accept', 'application/json');
    if (init?.body) headers.set('content-type', 'application/json');
    if (token) headers.set('authorization', `Bearer ${token}`);
    const response = await fetcher(`${normalizedBase}${path}`, { ...init, headers });
    if (!response.ok) {
      const detail = await response.text();
      throw new Error(`Genesis server ${response.status}: ${detail || response.statusText}`);
    }
    return response.json() as Promise<T>;
  };
  const capabilities = mapCapabilities(await request<BackendCapabilities>('/v1/studio/capabilities'));
  return {
    kind: 'remote',
    async close() {},
    async getCapabilities() {
      return capabilities;
    },
    async getStatus() {
      const [status, frontier] = await Promise.all([
        request<Record<string, number>>('/v1/status'),
        request<number>('/v1/frontier'),
      ]);
      return envelope(
        {
          nodeCount: status.node_count,
          edgeCount: status.edge_count,
          collectionCount: Array.isArray(status.collections) ? status.collections.length : 0,
          indexLag: status.index_lag,
          logicalClock: 0,
          memoryUsageMb: status.memory_usage_mb,
        },
        frontier,
      );
    },
    async listCollections() {
      return envelope(mapCollections(await request<BackendCollection[]>('/v1/collections')), null);
    },
    async listRelationalSchemas() {
      return envelope(mapSchemas(await request<BackendSchema[]>('/v1/studio/relational/schemas')), null);
    },
    async executeNamedQuery(queryRequest: NamedQueryRequest) {
      const data = await request<Record<string, unknown>[]>('/v1/relational/query', {
        method: 'POST',
        body: JSON.stringify(namedQueryRequest(queryRequest)),
      });
      return envelope(data, null);
    },
    async executeReadOnlyHql(query: string) {
      const data = await request<Record<string, unknown>[]>('/v1/studio/query/read', {
        method: 'POST',
        body: JSON.stringify({ query }),
      });
      return envelope(data, null);
    },
    async loadGraphScene(sceneRequest: SceneRequest) {
      const search = new URLSearchParams({ limit: String(sceneRequest.limit) });
      if (sceneRequest.seed) search.set('seed', sceneRequest.seed);
      return mapScene(await request<BackendGraphScene>(`/v1/studio/graph?${search}`));
    },
    async expandGraphScene(nodeId: string) {
      const search = new URLSearchParams({
        seed: nodeId,
        limit: String(capabilities.limits.expansionNodes),
        direction: 'both',
      });
      return mapScene(await request<BackendGraphScene>(`/v1/studio/graph?${search}`));
    },
    async inspectEntity(entityId: string) {
      return mapInspection(
        await request<BackendInspection>(`/v1/studio/entity/${encodeURIComponent(entityId)}`),
      );
    },
  };
}
