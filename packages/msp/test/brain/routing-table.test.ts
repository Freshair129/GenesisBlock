import { describe, expect, it } from 'vitest';

import { routingFor, writeTargetFor } from '../../src/brain/routing-table.js';
import type { AtomType } from '../../src/brain/types.js';

const PROJECT_ONLY_TYPES: AtomType[] = [
  'ADR',
  'FEAT',
  'BLUEPRINT',
  'AUDIT',
  'CONCEPT',
  'FRAMEWORK',
  'SPEC',
  'PROTOCOL',
  'MOD',
  'MASTER',
  'HOTFIX',
  'INC',
  'ISSUE',
];

const GLOBAL_FIRST_TYPES: AtomType[] = ['SKILL', 'ALGO', 'PROTO', 'PARAMS'];

const GLOBAL_ONLY_TYPES: AtomType[] = ['EPISODE', 'IDENTITY', 'REGISTRY'];

describe('routingFor — project-only atom types', () => {
  for (const t of PROJECT_ONLY_TYPES) {
    it(`${t} reads project only and writes project`, () => {
      const rule = routingFor(t);
      expect(rule.read_order).toEqual(['project']);
      expect(rule.write_target).toBe('project');
    });
  }
});

describe('routingFor — global-first, project-fallback types', () => {
  for (const t of GLOBAL_FIRST_TYPES) {
    it(`${t} reads global then project; write is conflict sentinel`, () => {
      const rule = routingFor(t);
      expect(rule.read_order).toEqual(['global', 'project']);
      expect(rule.write_target).toBe('conflict');
    });
  }
});

describe('routingFor — global-only atom types', () => {
  for (const t of GLOBAL_ONLY_TYPES) {
    it(`${t} reads global only and writes global`, () => {
      const rule = routingFor(t);
      expect(rule.read_order).toEqual(['global']);
      expect(rule.write_target).toBe('global');
    });
  }
});

describe('writeTargetFor — resolves the conflict sentinel via vault_id', () => {
  it('SKILL with no vault_id writes to global by default', () => {
    expect(writeTargetFor('SKILL', undefined)).toBe('global');
  });

  it('SKILL with vault_id set writes to project', () => {
    expect(writeTargetFor('SKILL', 'team-a')).toBe('project');
  });

  it('PROTO with vault_id set writes to project', () => {
    expect(writeTargetFor('PROTO', 'default')).toBe('project');
  });

  it('ADR ignores vault_id and stays project', () => {
    expect(writeTargetFor('ADR', undefined)).toBe('project');
    expect(writeTargetFor('ADR', 'anything')).toBe('project');
  });

  it('IDENTITY ignores vault_id and stays global', () => {
    expect(writeTargetFor('IDENTITY', 'anything')).toBe('global');
  });
});

describe('routingFor — unknown types', () => {
  it('throws on unknown atom type', () => {
    expect(() => routingFor('NOPE' as AtomType)).toThrow(/unknown atom type/);
  });
});
