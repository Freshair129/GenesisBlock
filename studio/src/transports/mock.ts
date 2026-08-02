import {
  assertSceneWithinLimits,
  type EntityInspection,
  type GraphScene,
  type SceneRequest,
  type StudioCapabilities,
  type StudioCollection,
  type StudioEnvelope,
  type StudioStatus,
  type StudioTransport,
} from '../domain/contracts';
import { createFixtureScene } from '../domain/scene';

const capabilities: StudioCapabilities = {
  protocolVersion: 'studio-mock-v0',
  engineVersion: 'fixture-only',
  mode: 'local',
  readFeatures: [
    'status.read',
    'relational.read',
    'graph.scene.read',
    'graph.scene.expand',
    'vector.read',
    'hql.read',
    'entity.inspect',
  ],
  writeFeatures: [],
  authFeatures: [],
  limits: {
    initialSceneNodes: 500,
    sceneNodeCeiling: 1_000,
    sceneEdgeCeiling: 3_000,
    expansionNodes: 100,
  },
  consistency: 'mock-fixture',
  unsupportedReasons: {
    mutation: 'S0 is intentionally read-only.',
    persistence: 'The mock transport never opens a GenesisBlockDB data root.',
  },
};

function envelope<T>(data: T, warnings: string[] = []): StudioEnvelope<T> {
  return {
    requestId: crypto.randomUUID(),
    frontier: null,
    generatedAt: new Date().toISOString(),
    truncated: false,
    warnings: ['MOCK TRANSPORT: no database was opened.', ...warnings],
    data,
  };
}

async function pause(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 80));
}

export function createMockTransport(): StudioTransport {
  const baseScene = createFixtureScene(240);

  return {
    kind: 'mock',
    async close() {},
    async getCapabilities() {
      await pause();
      return structuredClone(capabilities);
    },
    async getStatus() {
      await pause();
      const status: StudioStatus = {
        nodeCount: 12_480,
        edgeCount: 31_904,
        collectionCount: 3,
        indexLag: 17,
        logicalClock: 84_291,
        memoryUsageMb: 186.4,
      };
      return envelope(status);
    },
    async listCollections() {
      await pause();
      const collections: StudioCollection[] = [
        { name: 'default', dimension: 1_024, metric: 'cosine', vectorCount: 9_118, indexLag: 12 },
        { name: 'reasoning', dimension: 768, metric: 'cosine', vectorCount: 2_940, indexLag: 5 },
        { name: 'archive', dimension: 384, metric: 'l2', vectorCount: 422, indexLag: 0 },
      ];
      return envelope(collections);
    },
    async listRelationalSchemas() {
      await pause();
      return envelope([
        { namespace: 'knowledge', version: 4, tables: 6, namedQueries: [{ name: 'recent_evidence', parameters: [], defaultLimit: 25, maxLimit: 100 }] },
        { namespace: 'agents', version: 2, tables: 3, namedQueries: [{ name: 'agent_by_id', parameters: ['agent_id'], defaultLimit: 1, maxLimit: 1 }] },
      ]);
    },
    async executeNamedQuery(request) {
      await pause();
      return envelope([{ query: request.queryName, parameters: request.parameters, status: 'fixture-only' }]);
    },
    async executeReadOnlyHql(query: string) {
      await pause();
      if (!/^\s*(SEARCH|TRAVERSE|MATCH|HYBRID|CONTEXT)\b/i.test(query)) {
        throw new Error('S0 mock accepts only the approved read-only HQL command family.');
      }
      return envelope([{ query, result: 'fixture-only' }]);
    },
    async loadGraphScene(request: SceneRequest) {
      await pause();
      const limit = Math.min(request.limit, capabilities.limits.initialSceneNodes);
      const scene = limit === baseScene.nodes.length ? baseScene : createFixtureScene(limit);
      assertSceneWithinLimits(scene, capabilities.limits);
      return envelope(scene);
    },
    async expandGraphScene(nodeId: string) {
      await pause();
      const scene = createFixtureScene(capabilities.limits.expansionNodes);
      scene.sceneId = `expand-${nodeId}`;
      assertSceneWithinLimits(scene, capabilities.limits);
      return envelope(scene, ['Expansion is a standalone fixture in S0; scene merge lands in S1.']);
    },
    async inspectEntity(entityId: string) {
      await pause();
      const node = baseScene.nodes.find((candidate) => candidate.id === entityId);
      if (!node) {
        throw new Error(`Unknown fixture entity: ${entityId}`);
      }
      const graph: EntityInspection['graph'] = {
        incoming: baseScene.edges.filter((edge) => edge.target === entityId).length,
        outgoing: baseScene.edges.filter((edge) => edge.source === entityId).length,
      };
      const inspection: EntityInspection = {
        entityId,
        label: node.label,
        availability: {
          relational: 'available',
          graph: 'available',
          vector: node.vectorCollection ? 'stale' : 'not_present',
          temporal: 'available',
        },
        relational: { kind: node.kind, group: node.group, verified: entityId.endsWith('1') },
        graph,
        vector: {
          collection: node.vectorCollection,
          nearestScore: node.vectorCollection ? 0.873 : null,
        },
        temporal: { validFrom: node.validFrom, causedBy: graph.incoming > 0 ? 'fixture-event' : null },
      };
      return envelope(inspection);
    },
  };
}

export type { GraphScene };
