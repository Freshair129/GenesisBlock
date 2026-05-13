import { describe, expect, it } from 'vitest';

import { merge } from '../../src/brain/merge.js';
import type { BrainHit } from '../../src/brain/types.js';

function hit(
  id: string,
  source: 'global' | 'project',
  path = `/fake/${source}/${id}`,
): BrainHit {
  return { atom: { id, type: 'SKILL' }, source, path };
}

describe('merge — project shadows global', () => {
  it('drops a global hit when a project hit exists for the same id', () => {
    const out = merge([hit('SKILL--X', 'global'), hit('SKILL--X', 'project')]);
    expect(out).toHaveLength(1);
    expect(out[0]!.source).toBe('project');
  });

  it('drops global regardless of order (project listed first)', () => {
    const out = merge([hit('SKILL--X', 'project'), hit('SKILL--X', 'global')]);
    expect(out).toHaveLength(1);
    expect(out[0]!.source).toBe('project');
  });
});

describe('merge — disjoint ids are all preserved', () => {
  it('keeps both when ids differ', () => {
    const out = merge([hit('SKILL--A', 'global'), hit('SKILL--B', 'project')]);
    expect(out).toHaveLength(2);
    expect(out.map((h) => h.atom.id)).toEqual(['SKILL--A', 'SKILL--B']);
  });

  it('keeps a global-only hit unchanged', () => {
    const out = merge([hit('SKILL--A', 'global')]);
    expect(out).toEqual([hit('SKILL--A', 'global')]);
  });

  it('keeps a project-only hit unchanged', () => {
    const out = merge([hit('SKILL--A', 'project')]);
    expect(out).toEqual([hit('SKILL--A', 'project')]);
  });
});

describe('merge — ordering', () => {
  it('preserves source-internal ordering across the result', () => {
    const input: BrainHit[] = [
      hit('SKILL--A', 'global'),
      hit('SKILL--B', 'global'),
      hit('SKILL--C', 'project'),
      hit('SKILL--D', 'project'),
    ];
    const out = merge(input);
    expect(out.map((h) => h.atom.id)).toEqual([
      'SKILL--A',
      'SKILL--B',
      'SKILL--C',
      'SKILL--D',
    ]);
  });

  it('preserves remaining order after shadowing removes a global entry', () => {
    const input: BrainHit[] = [
      hit('SKILL--A', 'global'),
      hit('SKILL--B', 'global'),
      hit('SKILL--B', 'project'),
      hit('SKILL--C', 'project'),
    ];
    const out = merge(input);
    expect(out.map((h) => `${h.source}:${h.atom.id}`)).toEqual([
      'global:SKILL--A',
      'project:SKILL--B',
      'project:SKILL--C',
    ]);
  });
});

describe('merge — edge cases', () => {
  it('returns empty array for empty input', () => {
    expect(merge([])).toEqual([]);
  });

  it('handles two global hits for the same id (both kept; no project to shadow)', () => {
    const a = hit('SKILL--A', 'global', '/g1/a');
    const b = hit('SKILL--A', 'global', '/g2/a');
    const out = merge([a, b]);
    expect(out).toHaveLength(2);
    expect(out.every((h) => h.source === 'global')).toBe(true);
  });

  it('handles two project hits for the same id (both kept)', () => {
    const a = hit('SKILL--A', 'project', '/p1/a');
    const b = hit('SKILL--A', 'project', '/p2/a');
    const out = merge([a, b]);
    expect(out).toHaveLength(2);
    expect(out.every((h) => h.source === 'project')).toBe(true);
  });

  it('drops every global entry for an id when any project entry exists', () => {
    const out = merge([
      hit('SKILL--A', 'global', '/g1/a'),
      hit('SKILL--A', 'global', '/g2/a'),
      hit('SKILL--A', 'project', '/p/a'),
    ]);
    expect(out).toHaveLength(1);
    expect(out[0]!.path).toBe('/p/a');
  });
});
