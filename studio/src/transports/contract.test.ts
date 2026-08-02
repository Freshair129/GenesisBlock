import { describe, expect, it } from 'vitest';
import type { StudioTransport } from '../domain/contracts';
import { createMockTransport } from './mock';

function runTransportContract(name: string, createTransport: () => StudioTransport): void {
  describe(`${name} StudioTransport contract`, () => {
    it('returns versioned capabilities before feature use', async () => {
      const capabilities = await createTransport().getCapabilities();
      expect(capabilities.protocolVersion).not.toBe('');
      expect(capabilities.limits.sceneNodeCeiling).toBeGreaterThan(0);
    });

    it('wraps reads with freshness and provenance metadata', async () => {
      const result = await createTransport().getStatus();
      expect(result.requestId).not.toBe('');
      expect(result.generatedAt).not.toBe('');
      expect(result.warnings.join(' ')).toMatch(/MOCK TRANSPORT/);
    });

    it('enforces the initial scene budget', async () => {
      const result = await createTransport().loadGraphScene({ limit: 999 });
      expect(result.data.nodes.length).toBeLessThanOrEqual(500);
    });

    it('rejects commands outside the read-only HQL family', async () => {
      await expect(createTransport().executeReadOnlyHql('DROP EVERYTHING')).rejects.toThrow(
        /read-only HQL/,
      );
    });

    it('returns availability per space instead of hiding gaps', async () => {
      const result = await createTransport().inspectEntity('entity-1');
      expect(Object.keys(result.data.availability)).toEqual([
        'relational',
        'graph',
        'vector',
        'temporal',
      ]);
    });
  });
}

runTransportContract('mock', createMockTransport);
