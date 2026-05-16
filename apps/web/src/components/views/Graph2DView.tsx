import React, { useRef, useState, useEffect } from 'react';
import type { Note, Edge, NoteType } from '../../types/gks';
import { GKS_SERVICE } from '../../services/gksService';

interface Graph2DViewProps {
  notes: Note[];
  edges: Edge[];
  focusId: string | null;
  onOpen?: (id: string) => void;
}

interface SimNode extends Note {
  x: number;
  y: number;
  vx: number;
  vy: number;
  deg: number;
}

interface SimLink {
  source: SimNode;
  target: SimNode;
}

export const Graph2DView: React.FC<Graph2DViewProps> = ({ notes, edges, focusId, onOpen }) => {
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ w: 800, h: 600 });
  const [nodeSize, setNodeSize] = useState(1.0);
  const [showTags, setShowTags] = useState(false);
  const [showOrphans, setShowOrphans] = useState(true);
  const [showFilters, setShowFilters] = useState(false);

  // Simulation state
  const simRef = useRef<{ nodes: SimNode[]; links: SimLink[] }>({ nodes: [], links: [] });
  const transform = useRef({ x: 0, y: 0, k: 1 });
  const drag = useRef<{ lx: number; ly: number; node: SimNode | null }>({ lx: 0, ly: 0, node: null });
  const [isDragging, setIsDragging] = useState(false);
  const alphaRef = useRef(1.0); // Simulation energy level

  // Initialize simulation
  useEffect(() => {
    const { notes: processedNotes, edges: processedEdges } = GKS_SERVICE.getGraphWithTags(notes, edges, showTags, showOrphans);

    const nodeMap = new Map<string, SimNode>();
    const simNodes: SimNode[] = processedNotes.map(n => {
      const sn: SimNode = {
        ...n,
        x: (Math.random() - 0.5) * 600,
        y: (Math.random() - 0.5) * 600,
        vx: 0,
        vy: 0,
        deg: 0,
      };
      nodeMap.set(n.id, sn);
      return sn;
    });

    const simLinks: SimLink[] = processedEdges
      .map(e => {
        const s = nodeMap.get(e.source);
        const t = nodeMap.get(e.target);
        if (s && t) {
          s.deg++;
          t.deg++;
          return { source: s, target: t };
        }
        return null;
      })
      .filter((l): l is SimLink[] => !!l)
      .flat();

    simRef.current = { nodes: simNodes, links: simLinks };
  }, [notes, edges, showTags, showOrphans]);

  // Resize handler
  useEffect(() => {
    const ro = new ResizeObserver(entries => {
      for (const entry of entries) {
        setSize({ w: entry.contentRect.width, h: entry.contentRect.height });
      }
    });
    if (wrapRef.current) ro.observe(wrapRef.current);
    return () => ro.disconnect();
  }, []);

  // Animation Loop
  useEffect(() => {
    let raf: number;
    alphaRef.current = 1.0; // Reset simulation energy on data change

    const tick = () => {
      const { nodes, links } = simRef.current;
      if (!nodes.length) return;

      // ── Simulation Physics ──────────────────────────────────────────────────
      if (alphaRef.current > 0.01) {
        const repulsion = -150 * alphaRef.current;
        const linkForce = 0.045 * alphaRef.current;
        const centerForce = 0.008 * alphaRef.current;
        const friction = 0.82;

        // Repulsion
        for (let i = 0; i < nodes.length; i++) {
          const a = nodes[i];
          for (let j = i + 1; j < nodes.length; j++) {
            const b = nodes[j];
            const dx = b.x - a.x;
            const dy = b.y - a.y;
            const d2 = dx * dx + dy * dy + 1;
            if (d2 > 400 * 400) continue;
            const f = repulsion / d2;
            const fx = dx * f, fy = dy * f;
            a.vx += fx; a.vy += fy;
            b.vx -= fx; b.vy -= fy;
          }
        }

        // Links
        for (const l of links) {
          const dx = l.target.x - l.source.x;
          const dy = l.target.y - l.source.y;
          const d = Math.sqrt(dx * dx + dy * dy) || 1;
          const f = (d - 80) * linkForce;
          const fx = (dx / d) * f, fy = (dy / d) * f;
          l.source.vx += fx; l.source.vy += fy;
          l.target.vx -= fx; l.target.vy -= fy;
        }

        // Integration & Centering
        for (const n of nodes) {
          n.vx -= n.x * centerForce;
          n.vy -= n.y * centerForce;
          n.vx *= friction;
          n.vy *= friction;
          n.x += n.vx;
          n.y += n.vy;
        }

        alphaRef.current *= 0.992; // Cool down
      }

      draw();
      raf = requestAnimationFrame(tick);
    };

    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [size, focusId]);

  const draw = () => {
    const canvas = canvasRef.current; if (!canvas) return;
    const ctx = canvas.getContext('2d'); if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    if (canvas.width !== size.w * dpr) {
      canvas.width = size.w * dpr;
      canvas.height = size.h * dpr;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    const { nodes, links } = simRef.current;
    const { x, y, k } = transform.current;
    const CX = size.w / 2 + x, CY = size.h / 2 + y;

    ctx.fillStyle = '#111111';
    ctx.fillRect(0, 0, size.w, size.h);

    // Edges
    ctx.beginPath();
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
    ctx.lineWidth = 0.5 * k;
    for (const l of links) {
      const isFocus = focusId && (l.source.id === focusId || l.target.id === focusId);
      const isHover = hover && (l.source.id === hover.id || l.target.id === hover.id);
      if (isFocus || isHover) continue; // Draw highlighted later
      ctx.moveTo(CX + l.source.x * k, CY + l.source.y * k);
      ctx.lineTo(CX + l.target.x * k, CY + l.target.y * k);
    }
    ctx.stroke();

    // Hover Edges (Accent links)
    if (hover) {
      ctx.beginPath();
      ctx.strokeStyle = 'rgba(0, 220, 255, 0.45)'; // Cyan accent for hover
      ctx.lineWidth = 1.0 * k;
      for (const l of links) {
        if (l.source.id === hover.id || l.target.id === hover.id) {
          ctx.moveTo(CX + l.source.x * k, CY + l.source.y * k);
          ctx.lineTo(CX + l.target.x * k, CY + l.target.y * k);
        }
      }
      ctx.stroke();
    }

    // Hot Edges (Focus links)
    if (focusId) {
      ctx.beginPath();
      ctx.strokeStyle = 'rgba(124, 92, 255, 0.6)'; // Purple for focus
      ctx.lineWidth = 1.2 * k;
      for (const l of links) {
        if (l.source.id === focusId || l.target.id === focusId) {
          ctx.moveTo(CX + l.source.x * k, CY + l.source.y * k);
          ctx.lineTo(CX + l.target.x * k, CY + l.target.y * k);
        }
      }
      ctx.stroke();
    }

    // Nodes
    const neighbors = new Set<string>();
    if (focusId) {
      links.forEach(l => {
        if (l.source.id === focusId) neighbors.add(l.target.id);
        if (l.target.id === focusId) neighbors.add(l.source.id);
      });
    }

    for (const n of nodes) {
      const nx = CX + n.x * k, ny = CY + n.y * k;
      if (nx < -50 || nx > size.w + 50 || ny < -50 || ny > size.h + 50) continue;

      const isFocus = n.id === focusId;
      const isNbr = neighbors.has(n.id);
      const isDim = focusId && !isFocus && !isNbr;
      const r = Math.max(2, (3 + Math.sqrt(n.deg) * 1.5) * k * nodeSize);

      const isTag = n.id.startsWith('tag:');
      const meta = isTag ? { raw: 'rgba(124, 92, 255, 0.9)' } : (GKS_SERVICE.TYPE_META[n.type as NoteType] || { raw: '#6b7390' });
      
      ctx.fillStyle = isDim ? 'rgba(60, 60, 70, 0.3)' : meta.raw;
      ctx.beginPath();
      if (isTag) {
        ctx.rect(nx - 3*k, ny - 3*k, 6*k, 6*k); // Square for tags
      } else {
        ctx.arc(nx, ny, r, 0, Math.PI * 2);
      }
      ctx.fill();

      if (isFocus) {
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 2;
        ctx.stroke();
      } else if (hover && n.id === hover.id) {
        ctx.strokeStyle = 'rgba(0, 220, 255, 0.8)';
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }

      // Labels
      const isHovered = hover && n.id === hover.id;
      if (k > 0.6 && (isFocus || isNbr || isHovered || (k > 1.2 && n.deg > 5))) {
        ctx.fillStyle = isFocus || isHovered ? '#fff' : 'rgba(230, 232, 240, 0.7)';
        ctx.font = `${isFocus || isHovered ? '600 ' : ''}10px var(--font-sans)`;
        ctx.textAlign = 'center';
        ctx.fillText(n.title, nx, ny + r + 12);
      }
    }
  };

  // Interaction handlers
  const handleMouseDown = (e: React.MouseEvent) => {
    const canvas = canvasRef.current; if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left, my = e.clientY - rect.top;
    const { x, y, k } = transform.current;
    const CX = size.w / 2 + x, CY = size.h / 2 + y;

    // Hit detection for nodes
    let hit: SimNode | null = null;
    for (const n of simRef.current.nodes) {
      const nx = CX + n.x * k, ny = CY + n.y * k;
      const r = Math.max(5, (3 + Math.sqrt(n.deg) * 1.5) * k);
      const dx = mx - nx, dy = my - ny;
      if (dx * dx + dy * dy < r * r) {
        hit = n; break;
      }
    }

    drag.current = { lx: e.clientX, ly: e.clientY, node: hit };
    setIsDragging(true);
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      const dx = e.clientX - drag.current.lx;
      const dy = e.clientY - drag.current.ly;
      const { k } = transform.current;

      if (drag.current.node) {
        // Drag individual node
        drag.current.node.x += dx / k;
        drag.current.node.y += dy / k;
        drag.current.node.vx = 0;
        drag.current.node.vy = 0;
        alphaRef.current = 1.0; // Wake up simulation
      } else {
        // Pan the whole view
        transform.current.x += dx;
        transform.current.y += dy;
      }
      
      drag.current.lx = e.clientX;
      drag.current.ly = e.clientY;
    } else {
      // Hover detection
      const canvas = canvasRef.current; if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left, my = e.clientY - rect.top;
      const { x, y, k } = transform.current;
      const CX = size.w / 2 + x, CY = size.h / 2 + y;

      let hit = null;
      for (const n of simRef.current.nodes) {
        const nx = CX + n.x * k, ny = CY + n.y * k;
        const r = Math.max(5, (3 + Math.sqrt(n.deg) * 1.5) * k);
        const dx = mx - nx, dy = my - ny;
        if (dx * dx + dy * dy < r * r) {
          hit = n; break;
        }
      }
      setHover(hit ? { id: hit.id, x: mx, y: my } : null);
    }
  };

  const handleMouseUp = (e: React.MouseEvent) => {
    if (isDragging) {
      const dx = Math.abs(e.clientX - drag.current.lx);
      const dy = Math.abs(e.clientY - drag.current.ly);
      if (dx < 3 && dy < 3) {
        // Was a click
        if (drag.current.node) {
          onOpen?.(drag.current.node.id);
        } else if (hover) {
          onOpen?.(hover.id);
        }
      }
    }
    drag.current.node = null;
    setIsDragging(false);
  };

  const handleWheel = (e: React.WheelEvent) => {
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    transform.current.k = Math.max(0.1, Math.min(5, transform.current.k * delta));
  };

  return (
    <div className="graph-wrap" ref={wrapRef} style={{ background: '#111' }}>
      <canvas
        ref={canvasRef}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onWheel={handleWheel}
        style={{ cursor: isDragging ? 'grabbing' : 'grab' }}
      />
      <div className="graph-overlay" style={{ pointerEvents: 'none' }}>
        <h4>Obsidian Style · 2D Force</h4>
        <div style={{ color: 'var(--text-mute)', fontSize: 11 }}>
          drag to pan · scroll to zoom · click to focus
        </div>
      </div>
      <div className="graph-controls" style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <button className="filter-toggle" onClick={() => setShowFilters(!showFilters)}
                style={{ background: 'var(--bg-3)', border: '1px solid var(--border)', borderRadius: 6, padding: '4px 8px', color: 'var(--text)', cursor: 'pointer', fontSize: 12 }}>
          Filters {showFilters ? '▼' : '▶'}
        </button>
        {showFilters && (
          <div className="filter-panel" style={{ background: 'var(--bg-3)', border: '1px solid var(--border)', borderRadius: 8, padding: 12, display: 'flex', flexDirection: 'column', gap: 10, minWidth: 160, boxShadow: '0 8px 24px rgba(0,0,0,0.3)' }}>
            <div className="row" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <label style={{ fontSize: 11, color: 'var(--text-mute)' }}>Tags</label>
              <input type="checkbox" checked={showTags} onChange={e => setShowTags(e.target.checked)} />
            </div>
            <div className="row" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <label style={{ fontSize: 11, color: 'var(--text-mute)' }}>Orphans</label>
              <input type="checkbox" checked={showOrphans} onChange={e => setShowOrphans(e.target.checked)} />
            </div>
            <div className="row" style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <label style={{ fontSize: 11, color: 'var(--text-mute)' }}>Node size</label>
              <input type="range" min="0.3" max="3" step="0.1" value={nodeSize}
                     onChange={e => setNodeSize(+e.target.value)} style={{ width: '100%' }} />
            </div>
          </div>
        )}
      </div>
      {hover && (
        <div className="node-hover" style={{ left: hover.x, top: hover.y, pointerEvents: 'none' }}>
          <b>{hover.title || hover.id}</b>
        </div>
      )}
    </div>
  );
};
