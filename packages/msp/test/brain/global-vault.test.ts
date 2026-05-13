import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { exists, globalRoot, globalSubdir } from '../../src/brain/global-vault.js';

const ORIGINAL_PLATFORM = process.platform;

function setPlatform(value: NodeJS.Platform): void {
  Object.defineProperty(process, 'platform', { value, configurable: true });
}

function restorePlatform(): void {
  Object.defineProperty(process, 'platform', {
    value: ORIGINAL_PLATFORM,
    configurable: true,
  });
}

describe('globalRoot — Windows', () => {
  beforeEach(() => {
    setPlatform('win32');
    vi.stubEnv('USERPROFILE', 'C:\\Users\\testuser');
  });

  afterEach(() => {
    restorePlatform();
    vi.unstubAllEnvs();
  });

  it('uses USERPROFILE + .brain on win32', () => {
    expect(globalRoot()).toBe(path.join('C:\\Users\\testuser', '.brain'));
  });

  it('throws if USERPROFILE is unset on win32', () => {
    vi.stubEnv('USERPROFILE', '');
    expect(() => globalRoot()).toThrow(/USERPROFILE/);
  });
});

describe('globalRoot — Linux/macOS', () => {
  beforeEach(() => {
    setPlatform('linux');
  });

  afterEach(() => {
    restorePlatform();
    vi.unstubAllEnvs();
  });

  it('honours XDG_DATA_HOME when set', () => {
    vi.stubEnv('XDG_DATA_HOME', '/custom/xdg');
    expect(globalRoot()).toBe(path.join('/custom/xdg', 'brain'));
  });

  it('falls back to homedir + .brain when XDG_DATA_HOME is unset', () => {
    vi.stubEnv('XDG_DATA_HOME', '');
    expect(globalRoot()).toBe(path.join(os.homedir(), '.brain'));
  });
});

describe('globalSubdir', () => {
  beforeEach(() => {
    setPlatform('linux');
    vi.stubEnv('XDG_DATA_HOME', '/x');
  });

  afterEach(() => {
    restorePlatform();
    vi.unstubAllEnvs();
  });

  it('returns skills/ for SKILL', () => {
    expect(globalSubdir('SKILL')).toBe(path.join('/x', 'brain', 'skills'));
  });

  it('returns proto/ for PROTO', () => {
    expect(globalSubdir('PROTO')).toBe(path.join('/x', 'brain', 'proto'));
  });

  it('returns params/ for PARAMS', () => {
    expect(globalSubdir('PARAMS')).toBe(path.join('/x', 'brain', 'params'));
  });

  it('returns episodic/ for EPISODE', () => {
    expect(globalSubdir('EPISODE')).toBe(path.join('/x', 'brain', 'episodic'));
  });

  it('throws for atom types not routed to global (e.g. ADR)', () => {
    expect(() => globalSubdir('ADR')).toThrow(/not routed to the global brain/);
  });
});

describe('exists', () => {
  let tmp: string;

  beforeEach(async () => {
    tmp = await fs.mkdtemp(path.join(os.tmpdir(), 'brain-global-'));
    setPlatform('linux');
  });

  afterEach(async () => {
    restorePlatform();
    vi.unstubAllEnvs();
    await fs.rm(tmp, { recursive: true, force: true });
  });

  it('returns false when globalRoot() does not exist', async () => {
    vi.stubEnv('XDG_DATA_HOME', path.join(tmp, 'missing'));
    expect(await exists()).toBe(false);
  });

  it('returns true when globalRoot() exists', async () => {
    vi.stubEnv('XDG_DATA_HOME', tmp);
    await fs.mkdir(path.join(tmp, 'brain'), { recursive: true });
    expect(await exists()).toBe(true);
  });
});
