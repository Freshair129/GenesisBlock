import { promises as fs } from 'node:fs';
import path from 'node:path';

import { parse as parseYaml } from 'yaml';

import type { AtomType } from './types.js';

/**
 * Scanner-local atom record (richer than the locked `AtomRecord` in
 * `types.ts`, which only carries `{ id, type }`).  Defined here on
 * purpose so we do not mutate the P1 public-API surface.
 */
export interface AtomRecord {
  id: string;
  type: AtomType;
  path: string;
  frontmatter: Record<string, unknown>;
}

const FRONT_DELIM = '---';

function extractFrontmatter(text: string): Record<string, unknown> | null {
  // Normalize CRLF → LF so the parser is platform-agnostic.
  const normalized = text.replace(/\r\n/g, '\n');
  if (!normalized.startsWith(`${FRONT_DELIM}\n`)) return null;
  const end = normalized.indexOf(`\n${FRONT_DELIM}\n`, FRONT_DELIM.length + 1);
  // Also support a `---` end-of-file (no trailing newline / no body).
  let yamlSlice: string;
  if (end === -1) {
    const tailIdx = normalized.lastIndexOf(`\n${FRONT_DELIM}`);
    if (tailIdx === -1 || tailIdx < FRONT_DELIM.length) return null;
    yamlSlice = normalized.slice(FRONT_DELIM.length + 1, tailIdx);
  } else {
    yamlSlice = normalized.slice(FRONT_DELIM.length + 1, end);
  }

  try {
    const data = parseYaml(yamlSlice);
    if (data && typeof data === 'object' && !Array.isArray(data)) {
      return data as Record<string, unknown>;
    }
    return null;
  } catch {
    return null;
  }
}

/**
 * Scan a single vault directory (one level deep) and return parsed
 * atoms.  Malformed atoms (missing `id`/`type`, broken frontmatter,
 * unreadable files) are skipped silently — we never throw from here
 * so that a bad atom does not poison the whole resolution.
 *
 * If `absPath` does not exist, returns `[]`.
 */
export async function scanDir(absPath: string): Promise<AtomRecord[]> {
  let entries: string[];
  try {
    entries = await fs.readdir(absPath);
  } catch {
    return [];
  }

  const out: AtomRecord[] = [];
  for (const name of entries) {
    if (!name.endsWith('.md')) continue;
    const full = path.join(absPath, name);
    let stat: Awaited<ReturnType<typeof fs.stat>>;
    try {
      stat = await fs.stat(full);
    } catch {
      continue;
    }
    if (!stat.isFile()) continue;

    let text: string;
    try {
      text = await fs.readFile(full, 'utf8');
    } catch {
      continue;
    }

    const data = extractFrontmatter(text);
    if (!data) continue;

    const id = data['id'];
    const type = data['type'];
    if (typeof id !== 'string' || typeof type !== 'string') continue;

    out.push({
      id,
      type: type as AtomType,
      path: full,
      frontmatter: data,
    });
  }
  return out;
}
