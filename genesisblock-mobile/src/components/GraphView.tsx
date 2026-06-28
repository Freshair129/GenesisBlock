import { useEffect, useRef } from "react";
import Graph from "graphology";
import Sigma from "sigma";
import type { NodeOutput, EdgeOutput } from "../lib/api";

interface GraphViewProps {
  nodes: NodeOutput[];
  edges: EdgeOutput[];
  onNodeTap: (id: string) => void;
}

// Governance-tier palette (derived from labels[0]).
const TIER_COLORS: Record<string, string> = {
  MASTER: "#e3b341", // gold
  EXPERT: "#388bfd", // blue
  STANDARD: "#3fb950", // green
};
const DEFAULT_COLOR = "#8b949e"; // grey — OBSERVER / unknown

function tierColor(node: NodeOutput): string {
  const tier = (node.labels[0] ?? "").toUpperCase();
  return TIER_COLORS[tier] ?? DEFAULT_COLOR;
}

// Map impact (0..1-ish, may be null) to a node radius.
function nodeSize(node: NodeOutput): number {
  const impact = typeof node.impact === "number" ? node.impact : 0;
  const clamped = Math.max(0, Math.min(1, impact));
  return 4 + clamped * 12; // 4 (small default) .. 16
}

// Short, human-ish label for the node.
function nodeLabel(node: NodeOutput): string {
  const props = node.props;
  if (props && typeof props === "object") {
    const candidate = props.name ?? props.title ?? props.label;
    if (typeof candidate === "string" && candidate.length > 0) return candidate;
  }
  return node.id;
}

export default function GraphView({ nodes, edges, onNodeTap }: GraphViewProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const sigmaRef = useRef<Sigma | null>(null);
  // Keep the latest callback without forcing a graph rebuild on identity change.
  const onNodeTapRef = useRef(onNodeTap);
  onNodeTapRef.current = onNodeTap;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const graph = new Graph({ multi: true, type: "directed" });

    // Add nodes. Deterministic circular initial layout (computed inline to
    // avoid an extra layout package); Sigma's camera handles pan/zoom/pinch.
    const count = Math.max(nodes.length, 1);
    nodes.forEach((node, i) => {
      if (graph.hasNode(node.id)) return; // guard against duplicate ids
      const angle = (2 * Math.PI * i) / count;
      graph.addNode(node.id, {
        x: Math.cos(angle),
        y: Math.sin(angle),
        size: nodeSize(node),
        color: tierColor(node),
        label: nodeLabel(node),
      });
    });

    // Add edges. Guard against endpoints not in the node set.
    edges.forEach((edge) => {
      if (!graph.hasNode(edge.from) || !graph.hasNode(edge.to)) return;
      if (graph.hasEdge(edge.id)) return;
      try {
        graph.addEdgeWithKey(edge.id, edge.from, edge.to, {
          label: edge.rel,
          size: 1,
          color: "#30363d",
        });
      } catch {
        // Ignore malformed / duplicate edges rather than crashing the view.
      }
    });

    const renderer = new Sigma(graph, container, {
      renderLabels: true,
      labelColor: { color: "#e6edf3" },
      labelFont: "system-ui, sans-serif",
      defaultEdgeColor: "#30363d",
      allowInvalidContainer: true,
    });
    sigmaRef.current = renderer;

    renderer.on("clickNode", ({ node }) => {
      onNodeTapRef.current(node);
    });

    return () => {
      renderer.kill();
      sigmaRef.current = null;
    };
  }, [nodes, edges]);

  return <div ref={containerRef} className="graph-view" />;
}
