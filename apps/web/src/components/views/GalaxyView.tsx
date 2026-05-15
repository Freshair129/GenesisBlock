import React, { useRef, useState, useEffect } from 'react';
import type { Note, Edge } from '../../types/gks';
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

export const GalaxyView: React.FC<GalaxyViewProps> = ({ notes, edges, focusId: _focusId, onOpen }) => {
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
  });

  // Camera refs
  const cam    = useRef({ yaw: 0.3,  pitch: -0.42, dist: 950 });
  const camTgt = useRef({ yaw: 0.3,  pitch: -0.42, dist: 950 });
  const lookAt    = useRef({ x: 0, y: 0, z: 0 });
  const lookAtTgt = useRef({ x: 0, y: 0, z: 0 });

  const dataRef = useRef<GalaxyData | null>(null);
  const timeRef = useRef(0); // persistent across effect reruns

  // ── Build galaxy data ────────────────────────────────────────────────────────
  useEffect(() => {
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

    const knodes: GalaxyNode[] = notes.map(n => {
      const ring = TYPE_RING[n.type] ?? TYPE_RING.ENTITY;
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
      const y = ((h3 % 1000) / 1000 - 0.5) * (7 + radius * 0.014);

      const nd: GalaxyNode = { id: n.id, type: n.type, title: n.title, x, y, z, deg: 0, radius };
      nodeMap[n.id] = nd;
      return nd;
    });

    edges.forEach(e => {
      if (nodeMap[e.source]) nodeMap[e.source].deg++;
      if (nodeMap[e.target]) nodeMap[e.target].deg++;
    });

    const klinks: GalaxyLink[] = edges.filter(e => nodeMap[e.source] && nodeMap[e.target]);

    // Background galaxy particles - reduced for performance
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
      const y = (rng() - 0.5) * (5 + radius * 0.014);

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
  }, [notes, edges]);

  // ── Resize observer ──────────────────────────────────────────────────────────
  useEffect(() => {
    const ro = new ResizeObserver(es => {
      for (const e of es) setSize({ w: e.contentRect.width, h: e.contentRect.height });
    });
    if (wrapRef.current) ro.observe(wrapRef.current);
    return () => ro.disconnect();
  }, []);

  // ── Projection ───────────────────────────────────────────────────────────────
  const project = (x: number, y: number, z: number, w: number, h: number): Projected | null => {
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
    
    // Scale relative to viewport to fix aspect ratio issues on different screens
    const baseScale = Math.min(w, h) * 1.1;
    const scale = baseScale / zEye;
    return { sx: x1 * scale, sy: y2 * scale, depth: zEye, scale: scale / (baseScale / 700) };
  };

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
      const dx = e.clientX - drag.current.lx, dy = e.clientY - drag.current.ly;
      camTgt.current.yaw   += dx * 0.005;
      camTgt.current.pitch += dy * 0.005;
      camTgt.current.pitch = Math.max(-1.45, Math.min(0.05, camTgt.current.pitch));
      drag.current.lx = e.clientX; drag.current.ly = e.clientY;
      setParams(p => p.autoRotate ? { ...p, autoRotate: false } : p);
      setHover(null);
    } else {
      const hit = pickNode(mx, my);
      setHover(hit ? { id: hit.id, x: mx, y: my } : null);
    }
  };
  const onMouseUp = (e: React.MouseEvent) => {
    if (!canvasRef.current) return;
    const rect = canvasRef.current.getBoundingClientRect();
    const mx = e.clientX - rect.left, my = e.clientY - rect.top;
    const moved = Math.abs(e.clientX - drag.current.lx) > 4 || Math.abs(e.clientY - drag.current.ly) > 4;
    drag.current.down = false;
    if (!moved) {
      const hit = pickNode(mx, my);
      if (hit) {
        setPinned(hit.id);
        lookAtTgt.current  = { x: hit.x, y: hit.y, z: hit.z };
        camTgt.current.dist = 200;
        onOpen?.(hit.id);
      }
    }
  };
  const onWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    camTgt.current.dist = Math.max(100, Math.min(2200, camTgt.current.dist * Math.exp(e.deltaY * 0.001)));
  };

  // ── Animation loop ────────────────────────────────────────────────────────────
  useEffect(() => {
    let raf: number, t0 = performance.now();

    const tick = (now: number) => {
      const dt = Math.min(0.05, (now - t0) / 1000); t0 = now;
      timeRef.current += dt; // accumulate — survives re-mounts

      if (params.autoRotate) camTgt.current.yaw += dt * 0.2 * params.speed;

      const c = cam.current, ct = camTgt.current;
      c.yaw   += (ct.yaw   - c.yaw)   * 0.08;
      c.pitch += (ct.pitch - c.pitch) * 0.08;
      c.dist  += (ct.dist  - c.dist)  * 0.08;
      const la = lookAt.current, lt = lookAtTgt.current;
      la.x += (lt.x - la.x) * 0.07;
      la.y += (lt.y - la.y) * 0.07;
      la.z += (lt.z - la.z) * 0.07;

      draw(timeRef.current);
      raf = requestAnimationFrame(tick);
    };

    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [size, params, pinned]);

  // ── Draw ──────────────────────────────────────────────────────────────────────
  const draw = (time: number) => {
    const canvas = canvasRef.current; if (!canvas) return;
    const data   = dataRef.current;   if (!data)   return;
    const dpr = window.devicePixelRatio || 1;
    if (canvas.width !== size.w * dpr) { canvas.width = size.w * dpr; canvas.height = size.h * dpr; }
    const ctx = canvas.getContext('2d')!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const W = size.w, H = size.h, CX = W / 2, CY = H / 2;

    // Deep space background
    const bg = ctx.createRadialGradient(CX, CY, 0, CX, CY, Math.max(W, H) * 0.8);
    bg.addColorStop(0, '#05030a');
    bg.addColorStop(1, '#010106');
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, W, H);

    const labelRects: { x1: number, y1: number, x2: number, y2: number }[] = [];

    // Distant star-field
    for (let i = 0; i < 220; i++) {
      const sx  = (i * 1297 + 511) % W;
      const sy  = (i * 859  + 307) % H;
      const twi = 0.3 + Math.sin(time * 1.3 + i * 0.53) * 0.25;
      const sr  = i % 9 === 0 ? 1.4 : 0.6;
      ctx.fillStyle = `rgba(210,215,240,${(twi * 0.55).toFixed(3)})`;
      ctx.beginPath(); ctx.arc(sx, sy, sr, 0, Math.PI * 2); ctx.fill();
    }

    // Core is now just empty or a simple point if desired, but we'll leave it clean.

    // Background particles - using fillRect for speed
    if (params.showParticles) {
      for (const p of data.particles) {
        const prj = project(p.x, p.y, p.z, W, H); if (!prj) continue;
        if (prj.depth > cam.current.dist + 600) continue;
        const px  = CX + prj.sx, py = CY + prj.sy;
        const df     = Math.max(0, Math.min(1, 1 - (prj.depth - 350) / 1400));
        const twinkle = 0.75 + Math.sin(time * 2.1 + p.x * 0.07 + p.z * 0.05) * 0.25;
        const al  = p.alpha * df * twinkle;
        if (al < 0.015) continue;
        const sz  = Math.max(0.4, p.sz * prj.scale * 0.65);
        ctx.fillStyle = `rgba(${p.r},${p.g},${p.b},${al.toFixed(3)})`;
        ctx.fillRect(px - sz, py - sz, sz * 2, sz * 2);
      }
    }

    // Pinned neighbours
    const pinN = pinned ? data.nodeMap[pinned] : null;
    const nbrs = new Set<string>();
    if (pinN) {
      data.klinks.forEach(l => {
        if (l.source === pinN.id) nbrs.add(l.target);
        if (l.target === pinN.id) nbrs.add(l.source);
      });
    }

    // Project all knowledge nodes
    const prjMap = new Map<string, Projected>();
    for (const n of data.knodes) {
      const p = project(n.x, n.y, n.z, W, H);
      if (p) prjMap.set(n.id, p);
    }

    // Edges
    for (const l of data.klinks) {
      const pa = prjMap.get(l.source), pb = prjMap.get(l.target);
      if (!pa || !pb) continue;
      const hot = pinN && (l.source === pinN.id || l.target === pinN.id);
      const dim = pinN && !hot;
      const ax = CX + pa.sx, ay = CY + pa.sy;
      const bx = CX + pb.sx, by = CY + pb.sy;
      const raw = GKS_SERVICE.TYPE_META[data.nodeMap[l.source]?.type as keyof typeof GKS_SERVICE.TYPE_META]?.raw ?? '#888';
      const al  = dim ? 0.05 : (hot ? 0.75 : 0.18);
      ctx.strokeStyle = gxHexA(raw, al);
      ctx.lineWidth   = hot ? 1.6 : 0.7;
      const mx = (ax + bx) / 2, my = (ay + by) / 2;
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.quadraticCurveTo(mx, my - 8, bx, by);
      ctx.stroke();

      if (hot) {
        const t2 = (time * 0.9) % 1, u = 1 - t2;
        const sx = u*u*ax + 2*u*t2*mx + t2*t2*bx;
        const sy_ = u*u*ay + 2*u*t2*(my - 8) + t2*t2*by;
        ctx.shadowColor = raw; ctx.shadowBlur = 8;
        ctx.fillStyle = '#fff';
        ctx.beginPath(); ctx.arc(sx, sy_, 2.5, 0, Math.PI * 2); ctx.fill();
        ctx.shadowBlur = 0;
      }
    }

    // Nodes (depth-sorted)
    const sorted = data.knodes
      .map(n => ({ n, p: prjMap.get(n.id) }))
      .filter((o): o is { n: GalaxyNode; p: Projected } => !!o.p)
      .sort((a, b) => b.p.depth - a.p.depth);

    for (const { n, p } of sorted) {
      const meta  = GKS_SERVICE.TYPE_META[n.type as keyof typeof GKS_SERVICE.TYPE_META]
        ?? { raw: '#a4a9be', label: n.type };
      const x = CX + p.sx, y = CY + p.sy;
      const isFocus = pinN?.id === n.id;
      const isNbr   = nbrs.has(n.id);
      const dim     = !!pinN && !isFocus && !isNbr;
      const df      = Math.max(0.12, Math.min(1, 850 / p.depth));
      const al      = (dim ? 0.14 : 1) * df;
      const pulse   = 1 + Math.sin(time * 2.2 + n.radius * 0.012) * 0.13;
      const r       = Math.max(3, (3.5 + Math.sqrt(n.deg) * 1.6) * Math.max(0.3, p.scale));

      // Halos and spikes removed for 'Dev Tool' sharpness

      // Star core
      ctx.beginPath(); ctx.arc(x, y, r * pulse, 0, Math.PI * 2);
      ctx.fillStyle = gxHexA(meta.raw, al);
      ctx.fill();
      ctx.beginPath(); ctx.arc(x, y, r * 0.38, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255,255,255,${(0.92 * al).toFixed(3)})`;
      ctx.fill();

      // Focus ring
      if (isFocus) {
        ctx.strokeStyle = 'rgba(255,255,255,0.85)';
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.arc(x, y, r * 3.2 + Math.sin(time * 4) * 2, 0, Math.PI * 2);
        ctx.stroke();
      }

      // Label collision detection
      const isHovered = hover?.id === n.id;
      const showLabel = params.showLabels && 
        (isFocus || isNbr || isHovered || (n.deg >= 12 && p.scale > 1.1));
      
      if (showLabel) {
        const txt = n.title;
        const fontSize = (isFocus || isHovered) ? 12 : 11;
        ctx.font = `${(isFocus || isHovered) ? '600 ' : ''}${fontSize}px var(--font-sans)`;
        const tw = ctx.measureText(txt).width;
        const th = fontSize;
        const tx = x - tw / 2;
        const ty = y + r * pulse + 5;

        // Check for collision with existing labels
        const rect = { x1: tx - 4, y1: ty - 2, x2: tx + tw + 4, y2: ty + th + 2 };
        let collision = false;
        if (!isFocus && !isHovered) {
          for (const other of labelRects) {
            if (!(rect.x2 < other.x1 || rect.x1 > other.x2 || rect.y2 < other.y1 || rect.y1 > other.y2)) {
              collision = true; break;
            }
          }
        }

        if (!collision || isFocus || isHovered) {
          ctx.textAlign = 'center'; ctx.textBaseline = 'top';
          ctx.fillStyle = `rgba(230,232,240,${Math.min(1, al * (isFocus || isHovered ? 2 : 1)).toFixed(3)})`;
          ctx.fillText(txt, x, ty);
          labelRects.push(rect);
        }
      }
    }
  };

  return (
    <div className="graph-wrap" ref={wrapRef}>
      <canvas
        ref={canvasRef}
        className="graph-canvas"
        onMouseDown={onMouseDown}
        onMouseMove={onMouseMove}
        onMouseUp={onMouseUp}
        onMouseLeave={() => { drag.current.down = false; setHover(null); }}
        onWheel={onWheel}
      />

      {hover && (() => {
        const note = NOTE_BY_ID[hover.id];
        return (
          <div className="node-hover" style={{ left: hover.x, top: hover.y }}>
            <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
              <TypeDot type={note?.type} />
              <b>{note?.title}</b>
            </div>
            <div style={{ color: 'var(--text-dim)', fontFamily: 'var(--font-mono)', fontSize: 10, marginTop: 2 }}>
              {hover.id}
            </div>
          </div>
        );
      })()}

      <div className="graph-overlay">
        <h4>Galaxy · Knowledge Cosmos</h4>
        <div style={{ color: 'var(--text-mute)', fontSize: 11, marginBottom: 6 }}>
          drag to orbit · scroll to zoom · click star to focus
        </div>
        {Object.entries(GKS_SERVICE.TYPE_META).map(([t, m]) => (
          <div className="legend-row" key={t}>
            <span className="swatch" style={{ background: m.raw }} />
            <span>{m.label}</span>
          </div>
        ))}
      </div>

      <div className="graph-controls">
        <div className="row">
          <label>Auto-orbit</label>
          <span
            className={'toggle' + (params.autoRotate ? ' on' : '')}
            onClick={() => setParams(p => ({ ...p, autoRotate: !p.autoRotate }))}
          />
        </div>
        <div className="row">
          <label>Speed</label>
          <input type="range" min="0" max="2" step="0.05" value={params.speed}
                 onChange={e => setParams(p => ({ ...p, speed: +e.target.value }))} />
        </div>
        <div className="row">
          <label>Particles</label>
          <span
            className={'toggle' + (params.showParticles ? ' on' : '')}
            onClick={() => setParams(p => ({ ...p, showParticles: !p.showParticles }))}
          />
        </div>
        <div className="row">
          <label>Labels</label>
          <span
            className={'toggle' + (params.showLabels ? ' on' : '')}
            onClick={() => setParams(p => ({ ...p, showLabels: !p.showLabels }))}
          />
        </div>
        {pinned && (
          <div className="row">
            <button
              style={{ fontSize: 11, color: 'var(--text-mute)', padding: '2px 0', cursor: 'pointer' }}
              onClick={() => {
                setPinned(null);
                lookAtTgt.current = { x: 0, y: 0, z: 0 };
                camTgt.current.dist = 950;
              }}
            >
              ← Reset view
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
