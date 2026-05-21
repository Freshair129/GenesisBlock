import React, { useRef, useState, useEffect } from 'react';
import type { Note, Edge, NoteType } from '../../types/gks';
import { NOTE_BY_ID } from '../../data/mockData';
import { GKS_SERVICE } from '../../services/gksService';
import { TypeDot } from '../shared/TypeDot';

interface GalaxyViewProps {
  notes: Note[];
  edges: Edge[];
  focusId: string | null;
  onOpen?: (id: string) => void;
}

interface GalaxyNode {
  id: string;
  type: string;
  title: string;
  x: number; y: number; z: number;
  deg: number;
  radius: number;
}

interface GalaxyLink {
  source: string;
  target: string;
}

interface Particle {
  x: number; y: number; z: number;
  r: number; g: number; b: number;
  alpha: number; sz: number;
}

interface GalaxyData {
  knodes: GalaxyNode[];
  klinks: GalaxyLink[];
  nodeMap: Record<string, GalaxyNode>;
  particles: Particle[];
}

interface Projected {
  sx: number; sy: number; depth: number; scale: number;
}

function gxHexA(hex: string, a: number): string {
  const h = hex.replace('#', '');
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return `rgba(${r},${g},${b},${a.toFixed(3)})`;
}

export const GalaxyView: React.FC<GalaxyViewProps> = ({ notes, edges, onOpen }) => {
  const wrapRef   = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ w: 800, h: 600 });
  const [hover,  setHover]  = useState<{ id: string; x: number; y: number } | null>(null);
  const [pinned, setPinned] = useState<string | null>(null);
  const [params, setParams] = useState({
    autoRotate:    true,
    speed:         0.55,
    showParticles: true,
    showLabels:    true,
    nodeSize:      1.0,
    showTags:      false,
    showOrphans:   true
  });
  const [showFilters, setShowFilters] = useState(false);

  // Camera refs
  const cam    = useRef({ yaw: 0.3,  pitch: -0.42, dist: 950 });
  const camTgt = useRef({ yaw: 0.3,  pitch: -0.42, dist: 950 });
  const lookAt    = useRef({ x: 0, y: 0, z: 0 });
  const lookAtTgt = useRef({ x: 0, y: 0, z: 0 });

  const dataRef = useRef<GalaxyData | null>(null);
  const timeRef = useRef(0);

  // ── Build galaxy data ────────────────────────────────────────────────────────
  useEffect(() => {
    const { notes: processedNotes, edges: processedEdges } = GKS_SERVICE.getGraphWithTags(notes, edges, params.showTags, params.showOrphans);

    const TYPE_RING: Record<string, { rMin: number; rMax: number; armBias: number }> = {
      MOC:     { rMin:   0, rMax:  55, armBias: 0   },
      CONCEPT: { rMin:  45, rMax: 130, armBias: 0.5 },
      FEAT:    { rMin: 100, rMax: 210, armBias: 1   },
      FLOW:    { rMin: 150, rMax: 270, armBias: 2   },
      ENTITY:  { rMin: 200, rMax: 370, armBias: 0.5 },
    };

    function hash(s: string): number {
      let h = 2166136261 >>> 0;
      for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
      return h >>> 0;
    }

    const nodeMap: Record<string, GalaxyNode> = {};
    const ARM_COUNT = 3;

    const knodes: GalaxyNode[] = processedNotes.map(n => {
      const isTag = n.id.startsWith('tag:');
      const ring = isTag ? { rMin: 350, rMax: 450, armBias: 1 } : (TYPE_RING[n.type] ?? TYPE_RING.ENTITY);
      const h1 = hash(n.id);
      const h2 = hash(n.id + '_arm');
      const h3 = hash(n.id + '_y');

      const radius  = ring.rMin + (h1 % 10000) / 10000 * (ring.rMax - ring.rMin);
      const armIdx  = (h2 % ARM_COUNT + Math.floor(ring.armBias)) % ARM_COUNT;
      const baseAng = (armIdx / ARM_COUNT) * Math.PI * 2;
      const spinAng = baseAng + radius * 0.013 + ((h2 % 1000) / 1000 - 0.5) * 0.8;
      const scatter = ((h1 % 1000) / 1000 - 0.5) * radius * 0.22;

      const x = Math.cos(spinAng) * radius + scatter;
      const z = Math.sin(spinAng) * radius + scatter;
      const y = ((h3 % 1000) / 1000 - 0.5) * (40 + radius * 0.2);

      const nd: GalaxyNode = { id: n.id, type: n.type, title: n.title, x, y, z, deg: 0, radius };
      nodeMap[n.id] = nd;
      return nd;
    });

    processedEdges.forEach(e => {
      if (nodeMap[e.source]) nodeMap[e.source].deg++;
      if (nodeMap[e.target]) nodeMap[e.target].deg++;
    });

    const klinks: GalaxyLink[] = processedEdges.filter(e => nodeMap[e.source] && nodeMap[e.target]);

    const PART_N = 3500;
    const particles: Particle[] = [];
    let s = 0x9e3779b9;
    const rng = () => { s ^= s << 13; s ^= s >> 17; s ^= s << 5; return (s >>> 0) / 4294967296; };

    for (let i = 0; i < PART_N; i++) {
      const arm    = i % ARM_COUNT;
      const rFrac  = Math.pow(rng(), 0.5);
      const radius = rFrac * 400;
      const baseA  = (arm / ARM_COUNT) * Math.PI * 2;
      const spinA  = baseA + radius * 0.013 + (rng() - 0.5) * 2.0;
      const scat   = (rng() - 0.5) * radius * 0.28;
      const x = Math.cos(spinA) * radius + scat;
      const z = Math.sin(spinA) * radius + scat;
      const y = (rng() - 0.5) * (20 + radius * 0.15);

      const t = rFrac;
      particles.push({
        x, y, z,
        r: Math.round(255 * (1 - t * 0.25)),
        g: Math.round(160 * (1 - t * 0.65) + t * 30),
        b: Math.round(60 + t * 140),
        alpha: 0.25 + rng() * 0.5,
        sz:    0.4  + rng() * (radius < 80 ? 1.6 : 0.9),
      });
    }

    dataRef.current = { knodes, klinks, nodeMap, particles };
  }, [notes, edges, params.showTags, params.showOrphans]);

  // ── Resize observer ──────────────────────────────────────────────────────────
  useEffect(() => {
    const ro = new ResizeObserver(es => {
      for (const e of es) setSize({ w: e.contentRect.width, h: e.contentRect.height });
    });
    if (wrapRef.current) ro.observe(wrapRef.current);
    return () => ro.disconnect();
  }, []);

  // ── Projection ───────────────────────────────────────────────────────────────
  const project = React.useCallback((x: number, y: number, z: number, w: number, h: number): Projected | null => {
    const c = cam.current, la = lookAt.current;
    x -= la.x; y -= la.y; z -= la.z;
    const cy_ = Math.cos(c.yaw), sy_ = Math.sin(c.yaw);
    const x1  =  cy_ * x + sy_ * z;
    const z1  = -sy_ * x + cy_ * z;
    const cp  = Math.cos(c.pitch), sp = Math.sin(c.pitch);
    const y2  = cp * y - sp * z1;
    const z2  = sp * y + cp * z1;
    const zEye = z2 + c.dist;
    if (zEye <= 1) return null;
    const baseScale = Math.min(w, h) * 1.1;
    const scale = baseScale / zEye;
    return { sx: x1 * scale, sy: y2 * scale, depth: zEye, scale: scale / (baseScale / 700) };
  }, []);

  // ── Pick ─────────────────────────────────────────────────────────────────────
  const pickNode = (mx: number, my: number): GalaxyNode | null => {
    const data = dataRef.current; if (!data) return null;
    const cx = size.w / 2, cy_ = size.h / 2;
    let best: GalaxyNode | null = null, bestD = Infinity;
    for (const n of data.knodes) {
      const p = project(n.x, n.y, n.z, size.w, size.h); if (!p) continue;
      const x = cx + p.sx, y = cy_ + p.sy;
      const r = Math.max(5, (4 + Math.sqrt(n.deg) * 1.5) * p.scale);
      const dx = mx - x, dy = my - y;
      if (dx * dx + dy * dy < (r + 10) * (r + 10) && p.depth < bestD) {
        best = n; bestD = p.depth;
      }
    }
    return best;
  };

  // ── Drag handlers ────────────────────────────────────────────────────────────
  const drag = useRef({ down: false, lx: 0, ly: 0 });

  const onMouseDown = (e: React.MouseEvent) => {
    drag.current = { down: true, lx: e.clientX, ly: e.clientY };
  };
  const onMouseMove = (e: React.MouseEvent) => {
    if (!canvasRef.current) return;
    const rect = canvasRef.current.getBoundingClientRect();
    const mx = e.clientX - rect.left, my = e.clientY - rect.top;
    if (drag.current.down) {
      camTgt.current.yaw   += (e.clientX - drag.current.lx) * 0.005;
      camTgt.current.pitch += (e.clientY - drag.current.ly) * 0.005;
      camTgt.current.pitch = Math.max(-1.45, Math.min(1.45, camTgt.current.pitch));
      drag.current.lx = e.clientX; drag.current.ly = e.clientY;
      setParams(p => ({ ...p, autoRotate: false }));
      setHover(null);
    } else {
      const hit = pickNode(mx, my);
      setHover(hit ? { id: hit.id, x: mx, y: my } : null);
    }
  };
  const onMouseUp = (e: React.MouseEvent) => {
    drag.current.down = false;
    const rect = canvasRef.current!.getBoundingClientRect();
    const mx = e.clientX - rect.left, my = e.clientY - rect.top;
    const hit = pickNode(mx, my);
    if (hit) {
      setPinned(hit.id);
      lookAtTgt.current  = { x: hit.x, y: hit.y, z: hit.z };
      camTgt.current.dist = 200;
      onOpen?.(hit.id);
    }
  };
  const onWheel = (e: React.WheelEvent) => {
    camTgt.current.dist = Math.max(100, Math.min(2200, camTgt.current.dist * Math.exp(e.deltaY * 0.001)));
  };

  // Touch: 1 finger = orbit, 2 fingers = pinch-zoom, tap = focus
  const touch = useRef({ active: false, sx: 0, sy: 0, lx: 0, ly: 0, moved: false, pinch: 0 });
  const onTouchStart = (e: React.TouchEvent) => {
    if (e.touches.length === 1) {
      const t = e.touches[0];
      touch.current = { active: true, sx: t.clientX, sy: t.clientY, lx: t.clientX, ly: t.clientY, moved: false, pinch: 0 };
      setHover(null);
    } else if (e.touches.length === 2) {
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      touch.current.active = false;
      touch.current.moved = true;
      touch.current.pinch = Math.hypot(dx, dy);
    }
  };
  const onTouchMove = (e: React.TouchEvent) => {
    if (e.touches.length === 2 && touch.current.pinch > 0) {
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      const d = Math.hypot(dx, dy) || 1;
      camTgt.current.dist = Math.max(100, Math.min(2200, camTgt.current.dist * (touch.current.pinch / d)));
      touch.current.pinch = d;
      return;
    }
    if (e.touches.length === 1 && touch.current.active) {
      const t = e.touches[0];
      camTgt.current.yaw += (t.clientX - touch.current.lx) * 0.005;
      camTgt.current.pitch += (t.clientY - touch.current.ly) * 0.005;
      camTgt.current.pitch = Math.max(-1.45, Math.min(1.45, camTgt.current.pitch));
      touch.current.lx = t.clientX; touch.current.ly = t.clientY;
      if (Math.abs(t.clientX - touch.current.sx) > 6 || Math.abs(t.clientY - touch.current.sy) > 6) {
        touch.current.moved = true;
      }
      setParams(p => ({ ...p, autoRotate: false }));
    }
  };
  const onTouchEnd = (e: React.TouchEvent) => {
    if (e.touches.length > 0) return;
    const tr = touch.current;
    if (tr.active && !tr.moved && canvasRef.current) {
      const rect = canvasRef.current.getBoundingClientRect();
      const hit = pickNode(tr.sx - rect.left, tr.sy - rect.top);
      if (hit) {
        setPinned(hit.id);
        lookAtTgt.current = { x: hit.x, y: hit.y, z: hit.z };
        camTgt.current.dist = 200;
        onOpen?.(hit.id);
      }
    }
    touch.current = { active: false, sx: 0, sy: 0, lx: 0, ly: 0, moved: false, pinch: 0 };
  };

  // ── Draw ──────────────────────────────────────────────────────────────────────
  const draw = React.useCallback((time: number) => {
    const canvas = canvasRef.current; if (!canvas) return;
    const data   = dataRef.current;   if (!data)   return;
    const dpr = window.devicePixelRatio || 1;
    if (canvas.width !== size.w * dpr) { canvas.width = size.w * dpr; canvas.height = size.h * dpr; }
    const ctx = canvas.getContext('2d')!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const W = size.w, H = size.h, CX = W / 2, CY = H / 2;

    ctx.fillStyle = '#05030a';
    ctx.fillRect(0, 0, W, H);

    if (params.showParticles) {
      for (const p of data.particles) {
        const prj = project(p.x, p.y, p.z, W, H); if (!prj) continue;
        const px  = CX + prj.sx, py = CY + prj.sy;
        const df     = Math.max(0, Math.min(1, 1 - (prj.depth - 350) / 1400));
        const twinkle = 0.75 + Math.sin(time * 2.1 + p.x * 0.07) * 0.25;
        const al  = p.alpha * df * twinkle;
        if (al < 0.02) continue;
        const sz  = Math.max(0.4, p.sz * prj.scale * 0.65);
        ctx.fillStyle = `rgba(${p.r},${p.g},${p.b},${al.toFixed(3)})`;
        ctx.fillRect(x, y, p.sz * proj.scale, p.sz * proj.scale);
      }
    }

    const prjMap = new Map<string, Projected>();
    data.knodes.forEach(n => { const p = project(n.x, n.y, n.z, W, H); if (p) prjMap.set(n.id, p); });

    const neighbors = new Set<string>();
    if (pinned) data.klinks.forEach(l => { if (l.source === pinned) neighbors.add(l.target); if (l.target === pinned) neighbors.add(l.source); });
    const hoverNbrs = new Set<string>();
    if (hover) data.klinks.forEach(l => { if (l.source === hover.id) hoverNbrs.add(l.target); if (l.target === hover.id) hoverNbrs.add(l.source); });

    // Edges
    data.klinks.forEach(l => {
      const pa = prjMap.get(l.source), pb = prjMap.get(l.target);
      if (!pa || !pb) return;
      const isFocus = pinned && (l.source === pinned || l.target === pinned);
      const isHover = hover && (l.source === hover.id || l.target === hover.id);
      const hot = isFocus || isHover;
      const dim = (pinned || hover) && !hot;
      const ax = CX + pa.sx, ay = CY + pa.sy, bx = CX + pb.sx, by = CY + pb.sy;
      const mx = (ax+bx)/2, my = (ay+by)/2;
      
      const edgeColor = isHover ? "0,220,255" : (isFocus ? "124,92,255" : "90,100,140");
      ctx.strokeStyle = `rgba(${edgeColor}, ${dim ? 0.05 : (hot ? 0.75 : 0.18)})`;
      ctx.lineWidth = hot ? 1.6 : 0.7;
      ctx.beginPath(); ctx.moveTo(ax, ay); ctx.quadraticCurveTo(mx, my-8, bx, by); ctx.stroke();
    });

    // Nodes
    const sorted = data.knodes.map(n => ({ n, p: prjMap.get(n.id) })).filter(o => !!o.p).sort((a,b) => b.p!.depth - a.p!.depth);
    sorted.forEach(({ n, p }) => {
      const isTag = n.id.startsWith('tag:');
      const meta = isTag ? { raw: '#7c5cff' } : (GKS_SERVICE.TYPE_META[n.type as NoteType] || { raw: "#a4a9be" });
      const x = CX + p!.sx, y = CY + p!.sy;
      const isFocus = pinned === n.id, isHovered = hover?.id === n.id, isNbr = neighbors.has(n.id) || hoverNbrs.has(n.id);
      const dim = (pinned || hover) && !isFocus && !isHovered && !isNbr;
      const alpha = (dim ? 0.14 : 1) * Math.max(0.12, Math.min(1, 850/p!.depth));
      const r = Math.max(3, (3.5 + Math.sqrt(n.deg) * 1.6) * Math.max(0.3, p!.scale) * params.nodeSize);

      ctx.beginPath();
      if (isTag) ctx.rect(x-r, y-r, r*2, r*2); else ctx.arc(x, y, r, 0, Math.PI*2);
      ctx.fillStyle = gxHexA(meta.raw, alpha); ctx.fill();
      ctx.beginPath(); ctx.arc(x, y, r*0.38, 0, Math.PI*2); ctx.fillStyle = `rgba(255,255,255,${0.92*alpha})`; ctx.fill();

      if (isFocus || isHovered) {
        ctx.strokeStyle = isHovered ? "rgba(0,220,255,0.85)" : "rgba(255,255,255,0.85)";
        ctx.lineWidth = 2; ctx.beginPath(); ctx.arc(x, y, r*3.2, 0, Math.PI*2); ctx.stroke();
      }

      if (params.showLabels && (isFocus || isHovered || isNbr || (n.deg >= 12 && p!.scale > 1.1))) {
        ctx.font = `${isFocus||isHovered ? '600 ' : ''}11px Inter`; ctx.textAlign = 'center'; ctx.textBaseline = 'top';
        ctx.fillStyle = `rgba(230,232,240,${alpha})`; ctx.fillText(n.title, x, y + r + 5);
      }
    });
  }, [size, params, pinned, hover, project]);

  // ── Animation loop ────────────────────────────────────────────────────────────
  useEffect(() => {
    let raf: number, t0 = performance.now();
    const tick = (now: number) => {
      const dt = Math.min(0.05, (now - t0) / 1000); t0 = now;
      timeRef.current += dt;
      if (params.autoRotate) camTgt.current.yaw += dt * 0.2 * params.speed;
      const c = cam.current, ct = camTgt.current;
      c.yaw += (ct.yaw - c.yaw) * 0.08; c.pitch += (ct.pitch - c.pitch) * 0.08; c.dist += (ct.dist - c.dist) * 0.08;
      const la = lookAt.current, lt = lookAtTgt.current;
      la.x += (lt.x - la.x) * 0.07; la.y += (lt.y - la.y) * 0.07; la.z += (lt.z - la.z) * 0.07;
      draw(timeRef.current); raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick); return () => cancelAnimationFrame(raf);
  }, [params, draw]);

  return (
    <div className="graph-wrap" ref={wrapRef}>
      <canvas ref={canvasRef} className="graph-canvas" onMouseDown={onMouseDown} onMouseMove={onMouseMove} onMouseUp={onMouseUp} onWheel={onWheel} onTouchStart={onTouchStart} onTouchMove={onTouchMove} onTouchEnd={onTouchEnd} />
      {hover && (
        <div className="node-hover" style={{ left: hover.x, top: hover.y, pointerEvents: 'none' }}>
          <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
            {hover.id.startsWith('tag:') ? <span style={{ color: '#7c5cff', fontWeight: 600 }}>#</span> : <TypeDot type={NOTE_BY_ID[hover.id]?.type} />}
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
            <div className="row" style={{ display: 'flex', justifyContent: 'space-between' }}><label style={{ fontSize: 11 }}>Auto-orbit</label><span className={"toggle" + (params.autoRotate ? ' on' : '')} onClick={() => setParams(p => ({ ...p, autoRotate: !p.autoRotate }))} /></div>
            <div className="row"><label style={{ fontSize: 11 }}>Speed</label><input type="range" min="0" max="2" step="0.05" value={params.speed} onChange={e => setParams(p => ({ ...p, speed: +e.target.value }))} style={{ width: '100%' }} /></div>
            <div className="row"><label style={{ fontSize: 11 }}>Node size</label><input type="range" min="0.3" max="3" step="0.1" value={params.nodeSize} onChange={e => setParams(p => ({ ...p, nodeSize: +e.target.value }))} style={{ width: '100%' }} /></div>
          </div>
        )}
        {pinned && <div className="row"><button style={{ fontSize: 11, color: 'var(--text-mute)', padding: '2px 0', cursor: 'pointer' }} onClick={() => { setPinned(null); lookAtTgt.current = { x: 0, y: 0, z: 0 }; camTgt.current.dist = 950; }}>← Reset view</button></div>}
      </div>
    </div>
  );
};
