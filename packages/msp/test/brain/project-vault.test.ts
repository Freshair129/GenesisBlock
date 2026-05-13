import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import {
  projectRoot,
  projectSubdir,
} from '../../src/brain/project-vault.js';

describe('projectRoot — walk-up', () => {
  let tmp: string;

  beforeEach(async () => {
    tmp = await fs.realpath(
      await fs.mkdtemp(path.join(os.tmpdir(), 'brain-project-')),
    );
  });

  afterEach(async () => {
    await fs.rm(tmp, { recursive: true, force: true });
  });

  it('finds gks/ when called from a deep subdirectory', async () => {
    const repo = path.join(tmp, 'repo');
    await fs.mkdir(path.join(repo, '.git'), { recursive: true });
    await fs.mkdir(path.join(repo, 'gks'), { recursive: true });
    const deep = path.join(repo, 'sub', 'deep');
    await fs.mkdir(deep, { recursive: true });

    expect(projectRoot(deep)).toBe(path.join(repo, 'gks'));
  });

  it('returns gks/ when cwd is already the repo root', async () => {
    const repo = path.join(tmp, 'repo');
    await fs.mkdir(path.join(repo, '.git'), { recursive: true });
    await fs.mkdir(path.join(repo, 'gks'), { recursive: true });

    expect(projectRoot(repo)).toBe(path.join(repo, 'gks'));
  });

  it('throws when no ancestor has both .git/ and gks/', async () => {
    const noRepo = path.join(tmp, 'no-repo', 'deep');
    await fs.mkdir(noRepo, { recursive: true });
    expect(() => projectRoot(noRepo)).toThrow(/no ancestor/);
  });

  it('throws when only .git/ exists without gks/', async () => {
    const repo = path.join(tmp, 'partial');
    await fs.mkdir(path.join(repo, '.git'), { recursive: true });
    expect(() => projectRoot(repo)).toThrow(/no ancestor/);
  });

  it('throws when only gks/ exists without .git/', async () => {
    const repo = path.join(tmp, 'partial2');
    await fs.mkdir(path.join(repo, 'gks'), { recursive: true });
    expect(() => projectRoot(repo)).toThrow(/no ancestor/);
  });
});

describe('projectSubdir', () => {
  it('joins explicit root with the per-type subdir', () => {
    const root = path.join('/repo', 'gks');
    expect(projectSubdir('ADR', root)).toBe(path.join(root, 'adr'));
    expect(projectSubdir('CONCEPT', root)).toBe(path.join(root, 'concept'));
    expect(projectSubdir('FEAT', root)).toBe(path.join(root, 'feat'));
    expect(projectSubdir('BLUEPRINT', root)).toBe(path.join(root, 'blueprint'));
    expect(projectSubdir('FRAMEWORK', root)).toBe(path.join(root, 'framework'));
    expect(projectSubdir('SPEC', root)).toBe(path.join(root, 'spec'));
  });

  it('throws for atom types not routed to project (e.g. IDENTITY)', () => {
    expect(() => projectSubdir('IDENTITY', '/repo/gks')).toThrow(
      /not routed to the project brain/,
    );
  });
});
