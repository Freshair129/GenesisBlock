import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import * as globalVault from '../../src/brain/global-vault.js';
import { resolve } from '../../src/brain/resolver.js';

function atomDoc(id: string, type: string, body = ''): string {
  return `---\nid: ${id}\ntype: ${type}\nstatus: stable\n---\n${body}\n`;
}

async function writeAtom(
  dir: string,
  filename: string,
  id: string,
  type: string,
): Promise<void> {
  await fs.mkdir(dir, { recursive: true });
  await fs.writeFile(path.join(dir, filename), atomDoc(id, type), 'utf8');
}

describe('resolve — combines P1 routing + P2 vaults', () => {
  let tmp: string;
  let globalDir: string;
  let projectRepo: string;
  let projectDeep: string;
  let originalCwd: () => string;

  beforeEach(async () => {
    tmp = await fs.realpath(
      await fs.mkdtemp(path.join(os.tmpdir(), 'brain-resolve-')),
    );

    globalDir = path.join(tmp, 'global');
    projectRepo = path.join(tmp, 'project');
    projectDeep = path.join(projectRepo, 'sub', 'deep');

    // global brain layout: $globalDir/skills/SKILL--FOO.md
    await writeAtom(
      path.join(globalDir, 'skills'),
      'SKILL--FOO.md',
      'SKILL--FOO',
      'SKILL',
    );

    // project brain layout: $projectRepo/{.git,gks/{adr,skill}}
    await fs.mkdir(path.join(projectRepo, '.git'), { recursive: true });
    await writeAtom(
      path.join(projectRepo, 'gks', 'adr'),
      'ADR--BAR.md',
      'ADR--BAR',
      'ADR',
    );
    await writeAtom(
      path.join(projectRepo, 'gks', 'skill'),
      'SKILL--FOO.md',
      'SKILL--FOO',
      'SKILL',
    );
    await fs.mkdir(projectDeep, { recursive: true });

    vi.spyOn(globalVault, 'globalRoot').mockReturnValue(globalDir);
    originalCwd = process.cwd;
    process.cwd = () => projectDeep;
  });

  afterEach(async () => {
    process.cwd = originalCwd;
    vi.restoreAllMocks();
    await fs.rm(tmp, { recursive: true, force: true });
  });

  it('resolves an ADR from the project brain only', async () => {
    const hits = await resolve({ type: 'ADR' });
    expect(hits).toHaveLength(1);
    expect(hits[0]!.atom.id).toBe('ADR--BAR');
    expect(hits[0]!.source).toBe('project');
    expect(hits[0]!.path).toBe(
      path.join(projectRepo, 'gks', 'adr', 'ADR--BAR.md'),
    );
  });

  it('project shadows global for SKILL collisions', async () => {
    const hits = await resolve({ type: 'SKILL' });
    expect(hits).toHaveLength(1);
    expect(hits[0]!.atom.id).toBe('SKILL--FOO');
    expect(hits[0]!.source).toBe('project');
    expect(hits[0]!.path).toBe(
      path.join(projectRepo, 'gks', 'skill', 'SKILL--FOO.md'),
    );
  });

  it('respects explicit id filter (project shadow still applies)', async () => {
    const hits = await resolve({ type: 'SKILL', id: 'SKILL--FOO' });
    expect(hits).toHaveLength(1);
    expect(hits[0]!.source).toBe('project');
    expect(hits[0]!.atom.id).toBe('SKILL--FOO');
  });

  it('returns [] without error when the global subdir is missing', async () => {
    // EPISODE is GLOBAL_ONLY; the global episodic/ dir does not exist.
    const hits = await resolve({ type: 'EPISODE' });
    expect(hits).toEqual([]);
  });

  it('returns [] without error when the global brain root is missing', async () => {
    // Point globalRoot at a nonexistent path.
    vi.spyOn(globalVault, 'globalRoot').mockReturnValue(
      path.join(tmp, 'no-global'),
    );
    const hits = await resolve({ type: 'EPISODE' });
    expect(hits).toEqual([]);
  });

  it('skips malformed atoms silently', async () => {
    // Drop a bogus file alongside the good ADR.
    await fs.writeFile(
      path.join(projectRepo, 'gks', 'adr', 'BROKEN.md'),
      'no frontmatter at all\n',
      'utf8',
    );
    await fs.writeFile(
      path.join(projectRepo, 'gks', 'adr', 'PARTIAL.md'),
      '---\nid: PARTIAL\n---\n', // missing type
      'utf8',
    );
    const hits = await resolve({ type: 'ADR' });
    expect(hits).toHaveLength(1);
    expect(hits[0]!.atom.id).toBe('ADR--BAR');
  });
});
