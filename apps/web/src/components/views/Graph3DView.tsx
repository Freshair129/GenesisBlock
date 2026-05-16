import React, { useRef, useState, useEffect } from 'react';
import type { Note, Edge, NoteType } from '../../types/gks';
import { NOTE_BY_ID } from '../../data/mockData';
import { GKS_SERVICE } from '../../services/gksService';
import { TypeDot } from '../shared/TypeDot';

interface Graph3DViewProps {
  notes: Note[];
  edges: Edge[];
  focusId: string | null;
  onOpen?: (id: string) => void;
}

interface SimNode extends Note {
  x: number; y: number; z: number;
  vx: number; vy: number; vz: number;
  deg: number;
}

interface SimLink {
  source: string;
  target: string;
  phase: number;
}

export const Graph3DView: React.FC<Graph3DViewProps> = ({ notes, edges, focusId, onOpen }) => {
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ w: 800, h: 600 });
  const [hover, setHover] = useState<{ id: string, x: number, y: number } | null>(null);
  const [params, setParams] = useState({
    autoRotate: true, signals: true, depth: true, speed: 0.3, nodeSize: 1.0,
    showTags: false, showOrphans: true
  });
  const [showFilters, setShowFilters] = useState(false);

  // Camera state
  const camRef = useRef({ yaw: 0.6, pitch: -0.25, dist: 700, target: { x: 0, y: 0, z: 0 } });
  const simRef = useRef<{ 
    nodes: SimNode[], 
    links: SimLink[], 
    byNode: Record<string, SimNode> 
  } | null>(null);
  const alphaRef = useRef(1.0);

  // Sim init
  useEffect(() => {
    const { notes: processedNotes, edges: processedEdges } = GKS_SERVICE.getGraphWithTags(notes, edges, params.showTags, params.showOrphans);
    const byId = Object.fromEntries(processedNotes.map(n => [n.id, n]));
    const R = 280;
    const simNodes: SimNode[] = processedNotes.map((n, i) => {
      const phi = Math.acos(1 - 2 * (i + 0.5) / processedNotes.length);
      const theta = Math.PI * (1 + Math.sqrt(5)) * i;
      return {
        ...n,
        x: R * Math.sin(phi) * Math.cos(theta),
        y: R * Math.sin(phi) * Math.sin(theta),
        z: R * Math.cos(phi),
        vx: 0, vy: 0, vz: 0,
        deg: 0,
      };
    });
    const links: SimLink[] = processedEdges
      .filter(e => byId[e.source] && byId[e.target])
      .map(e => ({ source: e.source, target: e.target, phase: Math.random() }));
    
    simNodes.forEach(n => { 
      n.deg = links.filter(l => l.source === n.id || l.target === n.id).length; 
    });
    
    simRef.current = { 
      nodes: simNodes, 
      links, 
      byNode: Object.fromEntries(simNodes.map(n => [n.id, n])) 
    };
    alphaRef.current = 1.0;
  }, [notes, edges, params.showTags, params.showOrphans]);

  // Resize
  useEffect(() => {
    const ro = new ResizeObserver(es => { 
      for (const e of es) setSize({ w: e.contentRect.width, h: e.contentRect.height }); 
    });
    if (wrapRef.current) ro.observe(wrapRef.current);
    return () => ro.disconnect();
  }, []);

  const project = React.useCallback((x: number, y: number, z: number) => {
    const cam = camRef.current;
    const cy = Math.cos(cam.yaw), sy = Math.sin(cam.yaw);
    const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
    const x1 = cy * x + sy * z;
    const z1 = -sy * x + cy * z;
    const y1 = cp * y - sp * z1;
    const z2 = sp * y + cp * z1;
    const f = 700;
    const zEye = z2 + cam.dist;
    if (zEye <= 1) return null;
    const s = f / zEye;
    return { sx: x1 * s, sy: y1 * s, depth: zEye, scale: s };
  }, []);

  const nodeR = React.useCallback((n: SimNode, scale: number) => {
    const isTag = n.id.startsWith('tag:');
    const base = isTag ? 2.2 : (3 + Math.sqrt(n.deg) * 1.4);
    return Math.max(2.4, base * scale * 0.9 * params.nodeSize);
  }, [params.nodeSize]);

  const pickNode = (mx: number, my: number) => {
    const sim = simRef.current; if (!sim) return null;
    const cx = size.w / 2, cy = size.h / 2;
    let best = null, bestD = Infinity;
    for (const n of sim.nodes) {
      const p = project(n.x, n.y, n.z); if (!p) continue;
      const x = cx + p.sx, y = cy + p.sy;
      const r = nodeR(n, p.scale);
      const dx = mx - x, dy = my - y;
      const d2 = dx * dx + dy * dy;
      if (d2 < (r + 8) * (r + 8) && p.depth < bestD) { best = n; bestD = p.depth; }
    }
    return best;
  };

  const dragRef = useRef({ down: false, lx: 0, ly: 0 });
  const onMouseDown = (e: React.MouseEvent) => { dragRef.current = { down: true, lx: e.clientX, ly: e.clientY }; };
  const onMouseMove = (e: React.MouseEvent) => {
    if (!canvasRef.current) return;
    const rect = canvasRef.current.getBoundingClientRect();
    const mx = e.clientX - rect.left, my = e.clientY - rect.top;
    if (dragRef.current.down) {
      camRef.current.yaw += (e.clientX - dragRef.current.lx) * 0.005;
      camRef.current.pitch += (e.clientY - dragRef.current.ly) * 0.005;
      camRef.current.pitch = Math.max(-1.2, Math.min(1.2, camRef.current.pitch));
      dragRef.current.lx = e.clientX; dragRef.current.ly = e.clientY;
      setParams(p => ({ ...p, autoRotate: false }));
      setHover(null);
    } else {
      const hit = pickNode(mx, my);
      setHover(hit ? { id: hit.id, x: mx, y: my } : null);
    }
  };
  const onMouseUp = (e: React.MouseEvent) => {
    dragRef.current.down = false;
    const rect = canvasRef.current!.getBoundingClientRect();
    const mx = e.clientX - rect.left, my = e.clientY - rect.top;
    const hit = pickNode(mx, my);
    if (hit) onOpen?.(hit.id);
  };
  const onWheel = (e: React.WheelEvent) => {
    camRef.current.dist = Math.max(220, Math.min(1800, camRef.current.dist * Math.exp(e.deltaY * 0.001)));
  };

  const hexA = React.useCallback((hex: string, a: number) => {
    const h = hex.replace("#", "");
    const r = parseInt(h.slice(0, 2), 16), g = parseInt(h.slice(2, 4), 16), b = parseInt(h.slice(4, 6), 16);
    return `rgba(${r},${g},${b},${a})`;
  }, []);

  const draw = React.useCallback(() => {
    const canvas = canvasRef.current; if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    if (canvas.width !== size.w * dpr) { canvas.width = size.w * dpr; canvas.height = size.h * dpr; }
    const ctx = canvas.getContext("2d"); if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = "#181818";
    ctx.fillRect(0, 0, size.w, size.h);

    const sim = simRef.current; if (!sim) return;
    const cx = size.w / 2, cy = size.h / 2;
    const projMap = new Map();
    sim.nodes.forEach(n => { const p = project(n.x, n.y, n.z); if (p) projMap.set(n.id, p); });

    const neighbors = new Set<string>();
    if (focusId) sim.links.forEach(l => { if (l.source === focusId) neighbors.add(l.target); if (l.target === focusId) neighbors.add(l.source); });
    const hoverNbrs = new Set<string>();
    if (hover) sim.links.forEach(l => { if (l.source === hover.id) hoverNbrs.add(l.target); if (l.target === hover.id) hoverNbrs.add(l.source); });

    const orderedLinks = sim.links.map(l => {
      const pa = projMap.get(l.source), pb = projMap.get(l.target);
      if (!pa || !pb) return null;
      return { l, pa, pb, depth: (pa.depth + pb.depth) / 2 };
    }).filter((o): o is any => !!o).sort((a,b) => b.depth - a.depth);

    orderedLinks.forEach(({ l, pa, pb }) => {
      const aId = l.source, bId = l.target;
      const bNode = sim.byNode[bId];
      const hot = (focusId && (aId === focusId || bId === focusId)) || (hover && (aId === hover.id || bId === hover.id));
      const dim = (focusId || hover) && !hot;
      const isHover = hover && (aId === hover.id || bId === hover.id);
      
      const ax = cx + pa.sx, ay = cy + pa.sy, bx = cx + pb.sx, by = cy + pb.sy;
      const mx = (ax+bx)/2, my = (ay+by)/2;
      const perpx = -(by-ay), perpy = (bx-ax), plen = Math.hypot(perpx, perpy) || 1;
      const off = 18 + Math.min(60, Math.hypot(bx-ax, by-ay)*0.08);
      const cxm = mx + (perpx/plen)*off*0.3, cym = my + (perpy/plen)*off*0.3;

      const color = isHover ? "0,220,255" : (hot ? "124,92,255" : "90,100,140");
      ctx.strokeStyle = `rgba(${color}, ${dim ? 0.08 : (hot ? 0.85 : 0.35)})`;
      ctx.lineWidth = hot ? 1.4 : 0.7;
      ctx.beginPath(); ctx.moveTo(ax, ay); ctx.quadraticCurveTo(cxm, cym, bx, by); ctx.stroke();

      if (params.signals && !dim) {
        const t = l.phase, u = 1-t;
        const px = u*u*ax + 2*u*t*cxm + t*t*bx, py = u*u*ay + 2*u*t*cym + t*t*by;
        ctx.beginPath(); ctx.arc(px, py, 1.8, 0, Math.PI*2);
        ctx.fillStyle = bId.startsWith('tag:') ? "#7c5cff" : (GKS_SERVICE.TYPE_META[bNode?.type as NoteType]?.raw || "#6b7390");
        ctx.fill();
      }
    });

    const orderedNodes = sim.nodes.map(n => ({ n, p: projMap.get(n.id) })).filter(o => !!o.p).sort((a,b) => b.p.depth - a.p.depth);
    orderedNodes.forEach(({ n, p }) => {
      const isTag = n.id.startsWith('tag:');
      const meta = isTag ? { raw: '#7c5cff' } : (GKS_SERVICE.TYPE_META[n.type as NoteType] || { raw: "#6b7390" });
      const r = nodeR(n, p.scale);
      const x = cx + p.sx, y = cy + p.sy;
      const isFocus = focusId === n.id, isHovered = hover?.id === n.id, isNbr = neighbors.has(n.id) || hoverNbrs.has(n.id);
      const dim = (focusId || hover) && !isFocus && !isHovered && !isNbr;
      const alpha = (dim ? 0.15 : 1) * (params.depth ? Math.max(0.25, Math.min(1, 800/p.depth)) : 1);

      ctx.beginPath();
      if (isTag) ctx.rect(x-r, y-r, r*2, r*2); else ctx.arc(x, y, r, 0, Math.PI*2);
      ctx.fillStyle = hexA(meta.raw, alpha); ctx.fill();
      
      if (isFocus || isHovered) {
        ctx.strokeStyle = isHovered ? "rgba(0,220,255,0.8)" : "rgba(255,255,255,0.9)";
        ctx.lineWidth = 2; ctx.stroke();
      }

      if (isFocus || isHovered || isNbr || (p.scale > 0.8 && n.deg >= 15)) {
        ctx.font = `${isFocus||isHovered ? "600 " : ""}10px Inter`; ctx.textAlign = "center";
        const tw = ctx.measureText(n.title).width;
        ctx.fillStyle = `rgba(24,24,24,${alpha*0.8})`; ctx.fillRect(x-tw/2-4, y+r+4, tw+8, 14);
        ctx.fillStyle = `rgba(220,220,220,${alpha})`; ctx.fillText(n.title, x, y+r+14);
      }
    });
  }, [size, focusId, hover, params, hexA, nodeR, project]);

  useEffect(() => {
    let raf: number, t0 = performance.now();
    const tick = (now: number) => {
      const dt = Math.min(0.05, (now - t0) / 1000); t0 = now;
      const sim = simRef.current;
      if (sim && alphaRef.current > 0.01) {
        const alpha = alphaRef.current, nodes = sim.nodes, links = sim.links;
        const REPEL = 1800*alpha, LINK_K = 0.05*alpha, LINK_D = 110, CENTER = 0.012*alpha;
        for (let i = 0; i < nodes.length; i++) {
          const a = nodes[i];
          for (let j = i+1; j < nodes.length; j++) {
            const b = nodes[j];
            const dx = b.x-a.x, dy = b.y-a.y, dz = b.z-a.z, d2 = dx*dx+dy*dy+dz*dz+0.1;
            if (d2 > 600*600) continue;
            const d = Math.sqrt(d2), f = REPEL/d2;
            a.vx -= (dx/d)*f; a.vy -= (dy/d)*f; a.vz -= (dz/d)*f;
            b.vx += (dx/d)*f; b.vy += (dy/d)*f; b.vz += (dz/d)*f;
          }
        }
        links.forEach(l => {
          const a = sim.byNode[l.source], b = sim.byNode[l.target]; if (!a || !b) return;
          const dx = b.x-a.x, dy = b.y-a.y, dz = b.z-a.z, d = Math.sqrt(dx*dx+dy*dy+dz*dz)||0.1;
          const f = (d-LINK_D)*LINK_K;
          a.vx += (dx/d)*f; a.vy += (dy/d)*f; a.vz += (dz/d)*f;
          b.vx -= (dx/d)*f; b.vy -= (dy/d)*f; b.vz -= (dz/d)*f;
        });
        nodes.forEach(n => {
          n.vx += -n.x*CENTER; n.vy += -n.y*CENTER; n.vz += -n.z*CENTER;
          n.vx *= 0.8; n.vy *= 0.8; n.vz *= 0.8;
          n.x += n.vx; n.y += n.vy; n.z += n.vz;
        });
        alphaRef.current *= 0.985;
      }
      if (sim) {
        if (params.signals) sim.links.forEach(l => { l.phase = (l.phase + dt*(0.35 + (Math.sin(l.phase*3)+1)*0.18)*params.speed*2) % 1; });
        if (params.autoRotate) camRef.current.yaw += dt*0.08*params.speed;
      }
      draw(); raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick); return () => cancelAnimationFrame(raf);
  }, [params, draw]);

  return (
    <div className="graph-wrap" ref={wrapRef}>
      <canvas ref={canvasRef} className="graph-canvas" onMouseDown={onMouseDown} onMouseMove={onMouseMove} onMouseUp={onMouseUp} onWheel={onWheel} />
      {hover && (
        <div className="node-hover" style={{ left: hover.x, top: hover.y, pointerEvents: 'none' }}>
          <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
            {hover.id.startsWith('tag:') ? <span style={{ color: '#7c5cff', fontWeight: 600 }}>#</span> : <TypeDot type={NOTE_BY_ID[hover.id]?.type as NoteType} />}
            <b>{NOTE_BY_ID[hover.id]?.title || hover.id}</b>
          </div>
        </div>
      )}
      <div className="graph-controls" style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <button className="filter-toggle" onClick={() => setShowFilters(!showFilters)} style={{ background: 'var(--bg-3)', border: '1px solid var(--border)', borderRadius: 6, padding: '4px 8px', color: 'var(--text)', cursor: 'pointer', fontSize: 12 }}>
          Filters {showFilters ? '▼' : '▶'}
        </button>
        {showFilters && (
          <div className="filter-panel" style={{ background: 'var(--bg-3)', border: '1px solid var(--border)', borderRadius: 8, padding: 12, display: 'flex', flexDirection: 'column', gap: 10, minWidth: 160, boxShadow: '0 8px 24px rgba(0,0,0,0.3)' }}>
            <div className="row" style={{ display: 'flex', justifyContent: 'space-between' }}><label style={{ fontSize: 11 }}>Tags</label><input type="checkbox" checked={params.showTags} onChange={e => setParams(p => ({ ...p, showTags: e.target.checked }))} /></div>
            <div className="row" style={{ display: 'flex', justifyContent: 'space-between' }}><label style={{ fontSize: 11 }}>Orphans</label><input type="checkbox" checked={params.showOrphans} onChange={e => setParams(p => ({ ...p, showOrphans: e.target.checked }))} /></div>
            <hr style={{ border: 0, borderTop: '1px solid var(--border)', margin: '4px 0' }} />
            <div className="row" style={{ display: 'flex', justifyContent: 'space-between' }}><label style={{ fontSize: 11 }}>Auto-orbit</label><span className={"toggle" + (params.autoRotate ? " on" : "")} onClick={() => setParams(p => ({ ...p, autoRotate: !p.autoRotate }))} /></div>
            <div className="row"><label style={{ fontSize: 11 }}>Speed</label><input type="range" min="0" max="2" step="0.05" value={params.speed} onChange={e => setParams(p => ({ ...p, speed: +e.target.value }))} style={{ width: '100%' }} /></div>
            <div className="row"><label style={{ fontSize: 11 }}>Node size</label><input type="range" min="0.3" max="3" step="0.1" value={params.nodeSize} onChange={e => setParams(p => ({ ...p, nodeSize: +e.target.value }))} style={{ width: '100%' }} /></div>
          </div>
        )}
      </div>
    </div>
  );
};
