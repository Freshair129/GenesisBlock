import React from 'react';
import { NOTE_BY_ID, GKS_DATA } from '../data/mockData';
import type { Note } from '../types/gks';
import { TYPE_META } from '../types/gks';

export function snippet(text: string, len = 110): string {
  const flat = text.replace(/^#.*$/gm, "").replace(/\[\[([^\]|]+)\]\]/g, "$1").replace(/\s+/g, " ").trim();
  return flat.length > len ? flat.slice(0, len) + "…" : flat;
}

// Build a sparse bag of terms from note tags + title + body
export function bagFor(note: { title: string, tags: string[], body: string }): Record<string, number> {
  const txt = (note.title + " " + (note.tags || []).join(" ") + " " + note.body)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ");
  const counts: Record<string, number> = {};
  txt.split(/\s+/).forEach(w => { if (w && w.length > 2) counts[w] = (counts[w]||0)+1; });
  return counts;
}

const NOTE_BAGS: Record<string, Record<string, number>> = Object.fromEntries(
  GKS_DATA.notes.map(n => [n.id, bagFor(n)])
);

export function bagCosine(a: Record<string, number>, b: Record<string, number>): number {
  let dot = 0, na = 0, nb = 0;
  for (const k in a) { na += a[k]*a[k]; if (b[k]) dot += a[k]*b[k]; }
  for (const k in b) { nb += b[k]*b[k]; }
  return dot / (Math.sqrt(na)*Math.sqrt(nb) || 1);
}

export function searchScore(query: string, note: Note): number {
  if (!query.trim()) return 0;
  const q = bagFor({ title: query, tags: [], body: query });
  const cos = bagCosine(q, NOTE_BAGS[note.id]);
  const titleHit = note.title.toLowerCase().includes(query.toLowerCase()) ? 0.25 : 0;
  return Math.min(1, cos * 1.8 + titleHit);
}

export function similarity(aId: string, bId: string): number {
  if (aId === bId) return 1;
  const A = NOTE_BY_ID[aId], B = NOTE_BY_ID[bId];
  if (!A || !B) return 0;
  const aTags = new Set(A.tags || []), bTags = new Set(B.tags || []);
  let overlap = 0; aTags.forEach(t => bTags.has(t) && overlap++);
  const jacc = overlap / (aTags.size + bTags.size - overlap || 1);
  const aAdj = GKS_DATA.adj[aId];
  const linkBoost = (aAdj?.out?.has(bId) || aAdj?.in?.has(bId)) ? 0.25 : 0;
  const typeBoost = A.type === B.type ? 0.08 : 0;
  return Math.min(1, jacc * 0.9 + linkBoost + typeBoost);
}

export function renderInline(text: string, onOpen?: (id: string) => void): (string | React.JSX.Element)[] {
  const parts: (string | React.JSX.Element)[] = [];
  const re = /\[\[([A-Za-z0-9-]+--[A-Za-z0-9-]+)(?:\|([^\]]+))?\]\]/g;
  let last = 0, m;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) parts.push(text.slice(last, m.index));
    const id = m[1];
    const label = m[2] || (NOTE_BY_ID[id]?.title ?? id);
    const dead = !NOTE_BY_ID[id];
    parts.push(
      <span key={m.index}
            className={"wikilink" + (dead ? " dead" : "")}
            onClick={(e) => { e.stopPropagation(); if (!dead) onOpen?.(id); }}>
        {label}
      </span>
    );
    last = m.index + m[0].length;
  }
  if (last < text.length) parts.push(text.slice(last));
  return parts;
}

export function renderBody(body: string, onOpen?: (id: string) => void): React.JSX.Element[] {
    const lines = body.split(/\n/);
    const out: React.JSX.Element[] = [];
    let i = 0;
    let listBuf: string[] | null = null;
    const flushList = () => {
      if (!listBuf) return;
      out.push(<ul key={"u"+out.length}>{listBuf.map((li, k) => <li key={k}>{renderInline(li, onOpen)}</li>)}</ul>);
      listBuf = null;
    };
    while (i < lines.length) {
      const raw = lines[i];
      const line = raw.trimEnd();
      if (/^# /.test(line)) {
        flushList();
      } else if (/^## /.test(line)) {
        flushList();
        out.push(<h2 key={"h"+i}>{line.slice(3)}</h2>);
      } else if (/^>\s?/.test(line)) {
        flushList();
        out.push(<blockquote key={"q"+i}>{renderInline(line.replace(/^>\s?/,""), onOpen)}</blockquote>);
      } else if (/^[-*]\s+/.test(line)) {
        if (!listBuf) listBuf = [];
        listBuf.push(line.replace(/^[-*]\s+/,""));
      } else if (line.trim() === "") {
        flushList();
      } else {
        flushList();
        out.push(<p key={"p"+i}>{renderInline(line, onOpen)}</p>);
      }
      i++;
    }
    flushList();
    return out;
  }

export function getGraphWithTags(notes: Note[], edges: Edge[], showTags: boolean, showOrphans: boolean) {
  let resultNotes = [...notes];
  let resultEdges = [...edges];

  if (showTags) {
    const tagSet = new Set<string>();
    notes.forEach(n => n.tags?.forEach(t => tagSet.add(t)));
    
    tagSet.forEach(tagName => {
      const tagId = `tag:${tagName}`;
      resultNotes.push({
        id: tagId,
        title: `#${tagName}`,
        type: 'concept' as any, // Virtual type
        tags: [],
        body: `Virtual tag node for ${tagName}`
      });
      
      notes.forEach(n => {
        if (n.tags?.includes(tagName)) {
          resultEdges.push({ source: tagId, target: n.id });
        }
      });
    });
  }

  if (!showOrphans) {
    const connected = new Set<string>();
    resultEdges.forEach(e => {
      connected.add(e.source);
      connected.add(e.target);
    });
    resultNotes = resultNotes.filter(n => connected.has(n.id));
  }

  return { notes: resultNotes, edges: resultEdges };
}

export const GKS_SERVICE = {
  TYPE_META,
  D: GKS_DATA,
  NOTE_BY_ID,
  snippet,
  searchScore,
  similarity,
  NOTE_BAGS,
  bagFor,
  bagCosine,
  renderInline,
  renderBody,
  getGraphWithTags
};
