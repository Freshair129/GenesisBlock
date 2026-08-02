export type StudioMode = 'local' | 'remote';

export type StudioFeature =
  | 'status.read'
  | 'relational.read'
  | 'graph.scene.read'
  | 'graph.scene.expand'
  | 'vector.read'
  | 'hql.read'
  | 'entity.inspect';

export interface StudioCapabilities {
  protocolVersion: string;
  engineVersion: string;
  mode: StudioMode;
  readFeatures: StudioFeature[];
  writeFeatures: string[];
  authFeatures: string[];
  limits: {
    initialSceneNodes: number;
    sceneNodeCeiling: number;
    sceneEdgeCeiling: number;
    expansionNodes: number;
  };
  consistency: 'mock-fixture' | 'read-committed' | 'stable-frontier';
  unsupportedReasons: Record<string, string>;
}

export interface StudioEnvelope<T> {
  requestId: string;
  frontier: number | null;
  generatedAt: string;
  truncated: boolean;
  warnings: string[];
  data: T;
}

export interface StudioStatus {
  nodeCount: number;
  edgeCount: number;
  collectionCount: number;
  indexLag: number;
  logicalClock: number;
  memoryUsageMb: number;
}

export interface StudioCollection {
  name: string;
  dimension: number;
  metric: 'cosine' | 'l2';
  vectorCount: number;
  indexLag: number;
}

export interface RelationalSchemaSummary {
  namespace: string;
  version: number;
  tables: number;
  namedQueries: RelationalNamedQuery[];
}

export interface RelationalNamedQuery {
  name: string;
  parameters: string[];
  defaultLimit: number;
  maxLimit: number;
}

export interface NamedQueryRequest {
  namespace: string;
  schemaVersion: number;
  queryName: string;
  parameters: Record<string, unknown>;
  limit?: number;
}

export interface GraphSceneNode {
  id: string;
  label: string;
  kind: 'knowledge' | 'agent' | 'event' | 'artifact';
  group: string;
  x: number;
  y: number;
  size: number;
  color: string;
  vectorCollection: string | null;
  validFrom: string;
}

export interface GraphSceneEdge {
  id: string;
  source: string;
  target: string;
  relation: string;
  color: string;
}

export interface GraphScene {
  sceneId: string;
  nodes: GraphSceneNode[];
  edges: GraphSceneEdge[];
  groups: string[];
  continuation: string | null;
}

export interface EntityInspection {
  entityId: string;
  label: string;
  availability: {
    relational: 'available' | 'not_present' | 'unsupported';
    graph: 'available' | 'not_present' | 'unsupported';
    vector: 'available' | 'not_present' | 'stale' | 'unsupported';
    temporal: 'available' | 'not_present' | 'unsupported';
  };
  relational: Record<string, string | number | boolean>;
  graph: { incoming: number; outgoing: number };
  vector: { collection: string | null; nearestScore: number | null };
  temporal: { validFrom: string; causedBy: string | null };
}

export interface SceneRequest {
  seed?: string;
  limit: number;
}

export interface StudioTransport {
  readonly kind: 'mock' | 'local' | 'remote';
  close(): Promise<void>;
  getCapabilities(): Promise<StudioCapabilities>;
  getStatus(): Promise<StudioEnvelope<StudioStatus>>;
  listCollections(): Promise<StudioEnvelope<StudioCollection[]>>;
  listRelationalSchemas(): Promise<StudioEnvelope<RelationalSchemaSummary[]>>;
  executeNamedQuery(request: NamedQueryRequest): Promise<StudioEnvelope<Record<string, unknown>[]>>;
  executeReadOnlyHql(query: string): Promise<StudioEnvelope<Record<string, unknown>[]>>;
  loadGraphScene(request: SceneRequest): Promise<StudioEnvelope<GraphScene>>;
  expandGraphScene(nodeId: string): Promise<StudioEnvelope<GraphScene>>;
  inspectEntity(entityId: string): Promise<StudioEnvelope<EntityInspection>>;
}

export function supportsFeature(
  capabilities: StudioCapabilities,
  feature: StudioFeature,
): boolean {
  return capabilities.readFeatures.includes(feature);
}

export function assertSceneWithinLimits(
  scene: GraphScene,
  limits: StudioCapabilities['limits'],
): void {
  if (scene.nodes.length > limits.sceneNodeCeiling) {
    throw new Error(`Scene contains ${scene.nodes.length} nodes; ceiling is ${limits.sceneNodeCeiling}`);
  }
  if (scene.edges.length > limits.sceneEdgeCeiling) {
    throw new Error(`Scene contains ${scene.edges.length} edges; ceiling is ${limits.sceneEdgeCeiling}`);
  }
}
