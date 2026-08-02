import type { GraphScene, GraphSceneEdge, GraphSceneNode } from './contracts';

const GROUPS = [
  { name: 'Memory', color: '#0e7490', kind: 'knowledge' as const },
  { name: 'Agents', color: '#b45309', kind: 'agent' as const },
  { name: 'Events', color: '#be123c', kind: 'event' as const },
  { name: 'Artifacts', color: '#4d7c0f', kind: 'artifact' as const },
];

function seededUnit(index: number, salt: number): number {
  const value = Math.sin(index * 12.9898 + salt * 78.233) * 43758.5453;
  return value - Math.floor(value);
}

export function createFixtureScene(requestedNodes = 240): GraphScene {
  const nodeCount = Math.max(1, Math.min(requestedNodes, 1_000));
  const nodes: GraphSceneNode[] = [];
  const edges: GraphSceneEdge[] = [];

  for (let index = 0; index < nodeCount; index += 1) {
    const group = GROUPS[index % GROUPS.length];
    const ring = 4 + (index % 17) * 0.42;
    const angle = index * 2.39996 + seededUnit(index, 2) * 0.4;
    nodes.push({
      id: `entity-${index + 1}`,
      label: `${group.name.slice(0, -1)} ${String(index + 1).padStart(3, '0')}`,
      kind: group.kind,
      group: group.name,
      x: Math.cos(angle) * ring + (seededUnit(index, 3) - 0.5) * 2,
      y: Math.sin(angle) * ring + (seededUnit(index, 5) - 0.5) * 2,
      size: index < 12 ? 8 : 3 + seededUnit(index, 7) * 3,
      color: group.color,
      vectorCollection: index % 7 === 0 ? null : index % 3 === 0 ? 'reasoning' : 'default',
      validFrom: new Date(Date.UTC(2026, 6, 1 + (index % 20))).toISOString(),
    });
  }

  for (let index = 1; index < nodeCount; index += 1) {
    const parent = Math.max(0, Math.floor(index * seededUnit(index, 11)));
    edges.push({
      id: `edge-tree-${index}`,
      source: nodes[parent].id,
      target: nodes[index].id,
      relation: index % 3 === 0 ? 'CAUSED_BY' : index % 2 === 0 ? 'REFERENCES' : 'RELATES_TO',
      color: '#a8a29e',
    });

    if (index > 8 && index % 4 === 0) {
      const cross = Math.floor(seededUnit(index, 13) * index);
      edges.push({
        id: `edge-cross-${index}`,
        source: nodes[index].id,
        target: nodes[cross].id,
        relation: 'SUPPORTS',
        color: '#d6d3d1',
      });
    }
  }

  return {
    sceneId: `fixture-${nodeCount}`,
    nodes,
    edges,
    groups: GROUPS.map((group) => group.name),
    continuation: nodeCount === 1_000 ? 'fixture-ceiling' : null,
  };
}
