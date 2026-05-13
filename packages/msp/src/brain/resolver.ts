import { globalRoot, globalSubdir } from './global-vault.js';
import { merge } from './merge.js';
import { projectRoot, projectSubdir } from './project-vault.js';
import { routingFor, type RoutingRule } from './routing-table.js';
import { scanDir } from './atom-scanner.js';
import type { BrainHit, BrainQuery, BrainSource } from './types.js';

const DEFAULT_RULE: RoutingRule = {
  read_order: ['project', 'global'],
  write_target: 'project',
};

function ruleFor(query: BrainQuery): RoutingRule {
  if (query.type) return routingFor(query.type);
  return DEFAULT_RULE;
}

async function rootExists(p: string): Promise<boolean> {
  try {
    const fs = await import('node:fs/promises');
    const st = await fs.stat(p);
    return st.isDirectory();
  } catch {
    return false;
  }
}

async function safeGlobalDir(query: BrainQuery): Promise<string | null> {
  // If the global root itself does not exist, skip entirely.
  let root: string;
  try {
    root = globalRoot();
  } catch {
    return null;
  }
  if (!(await rootExists(root))) return null;

  // No type → enumerating every global subdir is out of scope for P3
  // (the BLUEPRINT defers it; default behaviour returns only the root).
  if (!query.type) return null;

  try {
    return globalSubdir(query.type);
  } catch {
    return null;
  }
}

async function safeProjectDir(query: BrainQuery): Promise<string | null> {
  let root: string;
  try {
    root = projectRoot();
  } catch {
    return null;
  }
  if (!(await rootExists(root))) return null;

  if (!query.type) return null;

  try {
    return projectSubdir(query.type, root);
  } catch {
    return null;
  }
}

/**
 * Public entry point: resolve a query against the two brains and
 * return deduped hits.
 *
 * Algorithm (per BLUEPRINT--BRAIN-MERGE-STRATEGY §Resolution):
 *   1. Pick a routing rule (default = project then global when no
 *      type is supplied).
 *   2. For each source in `rule.read_order`, look up the vault
 *      directory.  Skip silently if the source root is missing.
 *   3. `scanDir()` reads the atoms.  Filter by `query.id` when set.
 *   4. Pass the combined list through `merge()` so project shadows
 *      global on id collisions.
 */
export async function resolve(query: BrainQuery): Promise<BrainHit[]> {
  const rule = ruleFor(query);
  const hits: BrainHit[] = [];

  for (const source of rule.read_order) {
    const dir =
      source === 'global'
        ? await safeGlobalDir(query)
        : await safeProjectDir(query);
    if (!dir) continue;

    const records = await scanDir(dir);
    for (const rec of records) {
      if (query.id && rec.id !== query.id) continue;
      const sourceTag: BrainSource = source;
      hits.push({
        atom: { id: rec.id, type: rec.type },
        source: sourceTag,
        path: rec.path,
      });
    }
  }

  return merge(hits);
}
