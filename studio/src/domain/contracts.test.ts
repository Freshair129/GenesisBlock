import { describe, expect, it } from 'vitest';
import { assertSceneWithinLimits, supportsFeature } from './contracts';
import { createFixtureScene } from './scene';
import { createMockTransport } from '../transports/mock';

describe('Studio domain contracts', () => {
  it('caps generated scenes at the product node ceiling', () => {
    const scene = createFixtureScene(5_000);
    expect(scene.nodes).toHaveLength(1_000);
    expect(scene.continuation).toBe('fixture-ceiling');
  });

  it('rejects a scene that exceeds negotiated limits', () => {
    const scene = createFixtureScene(12);
    expect(() =>
      assertSceneWithinLimits(scene, {
        initialSceneNodes: 5,
        sceneNodeCeiling: 10,
        sceneEdgeCeiling: 30,
        expansionNodes: 5,
      }),
    ).toThrow(/ceiling is 10/);
  });

  it('uses explicit capability membership', async () => {
    const capabilities = await createMockTransport().getCapabilities();
    expect(supportsFeature(capabilities, 'graph.scene.read')).toBe(true);
    expect(capabilities.writeFeatures).toEqual([]);
  });
});
