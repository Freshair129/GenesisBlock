import { useDeferredValue, useEffect, useEffectEvent, useMemo, useRef } from 'react';
import Graph from 'graphology';
import Sigma from 'sigma';
import type { GraphScene, GraphSceneNode } from '../../domain/contracts';

interface GraphCanvasProps {
  scene: GraphScene;
  filter: string;
  selectedId: string | null;
  onSelect: (node: GraphSceneNode) => void;
}

export function GraphCanvas({ scene, filter, selectedId, onSelect }: GraphCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const deferredFilter = useDeferredValue(filter.trim().toLocaleLowerCase());
  const selectNode = useEffectEvent(onSelect);
  const visibleNodes = useMemo(() => scene.nodes.filter((node) => {
    if (!deferredFilter) return true;
    return `${node.label} ${node.group} ${node.kind}`.toLocaleLowerCase().includes(deferredFilter);
  }), [deferredFilter, scene.nodes]);
  const relationshipsByNode = useMemo(() => {
    const labels = new Map<string, string>();
    const relationships = new Map<string, string[]>();
    const visibleIds = new Set(visibleNodes.map((node) => node.id));
    for (const node of scene.nodes) {
      labels.set(node.id, node.label);
      relationships.set(node.id, []);
    }
    for (const edge of scene.edges) {
      if (!visibleIds.has(edge.source) || !visibleIds.has(edge.target)) continue;
      relationships.get(edge.source)?.push(`${edge.relation} to ${labels.get(edge.target) ?? edge.target}`);
      relationships.get(edge.target)?.push(`${edge.relation} from ${labels.get(edge.source) ?? edge.source}`);
    }
    return relationships;
  }, [scene.edges, scene.nodes, visibleNodes]);

  useEffect(() => {
    if (!containerRef.current) {
      return;
    }

    const graph = new Graph();
    const visibleIds = new Set(visibleNodes.map((node) => node.id));

    for (const node of visibleNodes) {
      graph.addNode(node.id, {
        x: node.x,
        y: node.y,
        size: node.id === selectedId ? node.size * 1.7 : node.size,
        label: node.label,
        color: node.id === selectedId ? '#111827' : node.color,
        forceLabel: node.id === selectedId,
        highlighted: node.id === selectedId,
      });
    }

    for (const edge of scene.edges) {
      if (visibleIds.has(edge.source) && visibleIds.has(edge.target)) {
        graph.addEdgeWithKey(edge.id, edge.source, edge.target, {
          color: edge.color,
          size: 0.7,
        });
      }
    }

    const renderer = new Sigma(graph, containerRef.current, {
      allowInvalidContainer: false,
      defaultEdgeColor: '#d6d3d1',
      labelColor: { color: '#292524' },
      labelDensity: 0.15,
      labelFont: 'Bahnschrift',
      labelGridCellSize: 180,
      labelRenderedSizeThreshold: 9,
      renderEdgeLabels: false,
      zIndex: true,
    });

    renderer.on('clickNode', ({ node }) => {
      const selected = scene.nodes.find((candidate) => candidate.id === node);
      if (selected) {
        selectNode(selected);
      }
    });

    return () => renderer.kill();
  }, [deferredFilter, scene.edges, scene.nodes, selectedId, visibleNodes]);

  return (
    <>
      <div ref={containerRef} className="graph-canvas" aria-hidden="true" />
      <details className="graph-node-browser">
        <summary>Browse scene nodes ({visibleNodes.length})</summary>
        <ul aria-label="Graph scene nodes">
          {visibleNodes.map((node) => {
            const relationships = relationshipsByNode.get(node.id) ?? [];
            const relationshipSummary = relationships.length
              ? `${relationships.length} visible relationships: ${relationships.slice(0, 3).join('; ')}${relationships.length > 3 ? `; and ${relationships.length - 3} more` : ''}`
              : 'No visible relationships';
            return (
              <li key={node.id}>
                <button
                  type="button"
                  aria-label={`${node.label}, ${node.kind}, ${node.group}. ${relationshipSummary}. Select entity.`}
                  aria-pressed={selectedId === node.id}
                  onClick={() => onSelect(node)}
                >
                  <strong>{node.label}</strong>
                  <span>{node.kind} · {node.group}</span>
                  {relationships.length > 0 && <small>{relationshipSummary}</small>}
                </button>
              </li>
            );
          })}
        </ul>
        {visibleNodes.length === 0 && <p role="status">No scene nodes match this filter.</p>}
      </details>
    </>
  );
}
