import { useEffect, useMemo, useRef, useState } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import type { SuperNode, MetaEdge, GapSuggestion } from '../services/api';

/**
 * Force-directed view of the community meta-graph: SuperNodes (clusters) sized by
 * member count and tinted by semantic drift, linked by MetaEdges (weight =
 * inter-cluster edge count). Structural gaps are overlaid as dashed amber links.
 * Complements the card list in InsightPanel — same data, spatial layout.
 */

interface GraphNode {
  id: number;
  theme: string;
  member_count: number;
  impact: number;
  drift: number | null;
  val: number; // node size hint for force-graph
}
interface GraphLink {
  source: number;
  target: number;
  weight: number;
  gap?: boolean;
}

// Drift 0 → calm blue, higher drift → warmer amber: clusters whose centroid is
// moving stand out. `null` drift (never re-clustered) renders neutral blue.
function driftColor(drift: number | null): string {
  if (drift == null) return '#3b82f6';
  const t = Math.min(1, Math.max(0, drift));
  const r = Math.round(59 + t * (245 - 59));
  const g = Math.round(130 + t * (158 - 130));
  const b = Math.round(246 + t * (11 - 246));
  return `rgb(${r}, ${g}, ${b})`;
}

export function CommunityGraph({
  nodes,
  edges,
  gaps = [],
}: {
  nodes: SuperNode[];
  edges: MetaEdge[];
  gaps?: GapSuggestion[];
}) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(600);
  const HEIGHT = 340;

  // Track the container width so the canvas fills the panel responsively.
  useEffect(() => {
    if (!wrapRef.current) return;
    const el = wrapRef.current;
    const update = () => setWidth(el.clientWidth);
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const data = useMemo(() => {
    const present = new Set(nodes.map((n) => n.cluster_id));
    const gNodes: GraphNode[] = nodes.map((n) => ({
      id: n.cluster_id,
      theme: n.theme,
      member_count: n.member_count,
      impact: n.impact,
      drift: n.drift,
      val: Math.max(1, n.member_count),
    }));
    const gLinks: GraphLink[] = edges
      .filter((e) => present.has(e.from_cluster) && present.has(e.to_cluster))
      .map((e) => ({ source: e.from_cluster, target: e.to_cluster, weight: e.weight }));
    // Overlay structural gaps (semantically close, structurally disconnected).
    for (const g of gaps) {
      if (present.has(g.cluster_a) && present.has(g.cluster_b)) {
        gLinks.push({ source: g.cluster_a, target: g.cluster_b, weight: 1, gap: true });
      }
    }
    return { nodes: gNodes, links: gLinks };
  }, [nodes, edges, gaps]);

  const maxWeight = useMemo(
    () => Math.max(1, ...data.links.filter((l) => !l.gap).map((l) => l.weight)),
    [data.links],
  );

  return (
    <div ref={wrapRef} className="rounded-lg border border-white/5 bg-background/40 overflow-hidden">
      <ForceGraph2D
        graphData={data}
        width={width}
        height={HEIGHT}
        backgroundColor="rgba(0,0,0,0)"
        nodeRelSize={4}
        nodeVal={(n: GraphNode) => n.val}
        nodeColor={(n: GraphNode) => driftColor(n.drift)}
        nodeLabel={(n: GraphNode) =>
          `${n.theme || `Cluster ${n.id}`} — ${n.member_count} nodes, impact ${n.impact.toFixed(
            2,
          )}${n.drift != null ? `, drift ${n.drift.toFixed(3)}` : ''}`
        }
        linkColor={(l: GraphLink) => (l.gap ? '#f59e0b' : 'rgba(148,163,184,0.4)')}
        linkLineDash={(l: GraphLink) => (l.gap ? [4, 3] : null)}
        linkWidth={(l: GraphLink) => (l.gap ? 1 : 1 + (l.weight / maxWeight) * 3)}
        linkLabel={(l: GraphLink) =>
          l.gap ? `structural gap #${l.source} ↔ #${l.target}` : `weight ${l.weight}`
        }
        cooldownTicks={120}
      />
    </div>
  );
}

export default CommunityGraph;
