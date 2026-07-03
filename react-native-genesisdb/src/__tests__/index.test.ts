import { NativeModules } from 'react-native';

const mockNative = {
  open: jest.fn(),
  close: jest.fn(),
  addNode: jest.fn(),
  search: jest.fn(),
  executeHql: jest.fn(),
  retrieveContext: jest.fn(),
  flushIndex: jest.fn(),
};

// Must run before `../index` is required — NativeGenesisDb.ts captures
// NativeModules.GenesisDb once at module-eval time. ts-jest compiles to
// commonjs and preserves source order, so this assignment lands before the
// `import` below triggers the require.
(NativeModules as Record<string, unknown>).GenesisDb = mockNative;

// eslint-disable-next-line import/first
import { GenesisDB, GenesisDBError } from '../index';

beforeEach(() => {
  jest.clearAllMocks();
});

describe('GenesisDB (JSON pass-through layer)', () => {
  it('open() resolves the dbId returned by the native module', async () => {
    mockNative.open.mockResolvedValue(7);
    const db = await GenesisDB.open('/tmp/genesisdb');
    expect(mockNative.open).toHaveBeenCalledWith('/tmp/genesisdb');

    mockNative.addNode.mockResolvedValue(JSON.stringify({ id: 'n1' }));
    await db.addNode({ labels: [] });
    expect(mockNative.addNode.mock.calls[0][0]).toBe(7);
  });

  it('addNode sends snake_case wire JSON and parses the response', async () => {
    mockNative.open.mockResolvedValue(1);
    const db = await GenesisDB.open('/tmp/genesisdb');

    const wireOutput = {
      id: 'n1',
      labels: ['Person'],
      props: null,
      valid_from: '2026-07-03T00:00:00Z',
      clock: { time: 1, peer_id: 'p1' },
    };
    mockNative.addNode.mockResolvedValue(JSON.stringify(wireOutput));

    const result = await db.addNode({ labels: ['Person'], valid_from: '2026-07-03T00:00:00Z' });

    const sentJson = JSON.parse(mockNative.addNode.mock.calls[0][1]);
    expect(sentJson).toEqual({ labels: ['Person'], valid_from: '2026-07-03T00:00:00Z' });
    expect(result).toEqual(wireOutput);
  });

  it('does not touch keys inside the opaque props object', async () => {
    mockNative.open.mockResolvedValue(1);
    const db = await GenesisDB.open('/tmp/genesisdb');
    mockNative.addNode.mockResolvedValue(JSON.stringify({ id: 'n1', props: {}, labels: [], valid_from: 'x', clock: { time: 0, peer_id: 'p' } }));

    await db.addNode({ labels: [], props: { userName: 'Ada', nested: { someField: 1 } } });

    const sentJson = JSON.parse(mockNative.addNode.mock.calls[0][1]);
    // camelCase keys inside `props` must survive verbatim — a generic
    // deep-recasing layer would have mangled these into snake_case.
    expect(sentJson.props).toEqual({ userName: 'Ada', nested: { someField: 1 } });
  });

  it('retrieveContext builds the target_id/tier/budget/fuzzy wire payload', async () => {
    mockNative.open.mockResolvedValue(1);
    const db = await GenesisDB.open('/tmp/genesisdb');
    mockNative.retrieveContext.mockResolvedValue(
      JSON.stringify({ nodes: [], edges: [], super_nodes: [], token_estimate: 0, reasoning_path: 'H1' })
    );

    await db.retrieveContext('n1', 'H2', 500, true);

    const sentJson = JSON.parse(mockNative.retrieveContext.mock.calls[0][1]);
    expect(sentJson).toEqual({ target_id: 'n1', tier: 'H2', budget: 500, fuzzy: true });
  });

  it('propagates a native rejection', async () => {
    mockNative.open.mockResolvedValue(1);
    const db = await GenesisDB.open('/tmp/genesisdb');
    mockNative.addNode.mockRejectedValue(new Error('boom'));

    await expect(db.addNode({ labels: [] })).rejects.toThrow('boom');
  });

  it('rejects calls made after close()', async () => {
    mockNative.open.mockResolvedValue(1);
    mockNative.close.mockResolvedValue(undefined);
    const db = await GenesisDB.open('/tmp/genesisdb');
    await db.close();

    expect(mockNative.close).toHaveBeenCalledWith(1);
    await expect(db.addNode({ labels: [] })).rejects.toThrow(GenesisDBError);
  });
});
