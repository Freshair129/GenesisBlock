import { useDeferredValue, useEffect, useEffectEvent, useRef } from 'react';
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

  useEffect(() => {
    if (!containerRef.current) {
      return;
    }

    const graph = new Graph();
    const visibleNodes = scene.nodes.filter((node) => {
      if (!deferredFilter) {
        return true;
      }
      return `${node.label} ${node.group} ${node.kind}`.toLocaleLowerCase().includes(deferredFilter);
    });
    const visibleIds = new Set(visibleNodes.map((node) => node.id));

    for (const node of visibleNodes) {
      graph.addNode(node.id, {
        x: node.x,
        y: node.y,
        size: node.id === selectedId ? node.size * 1.7 : node.size,
        label: node.label,
        color: node.id === selectedId ? '#111827' : node.color,
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
      labelFont: 'Bahnschrift',
      labelRenderedSizeThreshold: 7,
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
  }, [deferredFilter, scene, selectedId]);

  return <div ref={containerRef} className="graph-canvas" aria-label="Fixture knowledge graph" />;
}
