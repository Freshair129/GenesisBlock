import fs from 'fs';
import path from 'path';

const GKS_INDEX_PATH = 'C:/Users/freshair/cognitive_system/gks/00_index/atomic_index.jsonl';
const OUTPUT_PATH = 'C:/Users/freshair/cognitive_system/apps/web/src/data/gksData.json';
const GKS_ROOT = 'C:/Users/freshair/cognitive_system/gks';

async function sync() {
  console.log('🔄 Syncing real GKS data (with body scanning)...');
  
  if (!fs.existsSync(GKS_INDEX_PATH)) {
    console.error('❌ atomic_index.jsonl not found!');
    return;
  }

  const lines = fs.readFileSync(GKS_INDEX_PATH, 'utf-8').split('\n').filter(Boolean);
  const rawAtoms = lines.map(line => JSON.parse(line));
  const idMap = new Set(rawAtoms.map(a => a.id));

  const notes = rawAtoms.map(atom => {
    // Read actual file content for the UI body (limited to 1000 chars for perf)
    let realBody = '';
    const fullPath = path.join(GKS_ROOT, atom.path);
    if (fs.existsSync(fullPath)) {
      realBody = fs.readFileSync(fullPath, 'utf-8');
    }

    return {
      id: atom.id,
      title: atom.title || atom.id,
      type: (atom.type || 'CONCEPT').toUpperCase(),
      tags: atom.tags || [],
      path: atom.path,
      body: realBody, // Full body for searching and link extraction
      embed: [Math.random() * 600 - 300, Math.random() * 600 - 300] 
    };
  });

  const edges = [];
  notes.forEach(note => {
    const atom = rawAtoms.find(a => a.id === note.id);
    const cl = atom.crosslinks || {};
    const refs = new Set([
      ...(cl.references || []),
      ...(cl.supersedes || []),
      ...(cl.superseded_by || []),
      ...(cl.implements || []),
      ...(cl.implemented_by || []),
      ...(cl.parent_blueprint || []),
      ...(cl.child_blueprints || []),
      ...(cl.related || [])
    ]);
    
    // Scan real body for wikilinks [[ID]]
    const matches = note.body.match(/\[\[(.*?)\]\]/g);
    if (matches) {
      matches.forEach(m => {
        const targetId = m.slice(2, -2).split('|')[0].trim();
        if (idMap.has(targetId)) {
          refs.add(targetId);
        }
      });
    }

    refs.forEach(target => {
      if (target && target !== note.id && idMap.has(target)) {
        edges.push({ source: note.id, target });
      }
    });
  });

  const tagCounts = {};
  notes.forEach(n => n.tags.forEach(t => { tagCounts[t] = (tagCounts[t]||0)+1; }));

  const data = {
    notes,
    edges,
    tags: Object.entries(tagCounts).sort((a,b)=>b[1]-a[1]),
    daily: [] 
  };

  fs.writeFileSync(OUTPUT_PATH, JSON.stringify(data, null, 2));
  console.log(`✅ Synced ${notes.length} atoms and ${edges.length} edges (Metadata + Real Body Wikilinks).`);
}

sync();
