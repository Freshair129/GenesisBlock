import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { GraphStore } from '../../src/memory/graph.js'
import { createGenesisGraphBackend } from '../../src/memory/graph/genesis-graph.js'
import type { GraphBackend } from '../../src/memory/graph.js'

interface BackendFixture {
  name: string
  create: () => Promise<{ backend: GraphBackend; cleanup?: () => Promise<void> }>
}

const backends: BackendFixture[] = [
  {
    name: 'GraphStore (in-memory)',
    create: async () => ({ backend: new GraphStore() }),
  },
  {
    name: 'GenesisGraphBackend (Phase 0 — JSONL)',
    create: async () => {
      const dir = await mkdtemp(join(tmpdir(), 'gks-genesis-graph-'))
      const backend = createGenesisGraphBackend({ path: dir })
      return {
        backend,
        cleanup: async () => {
          await rm(dir, { recursive: true, force: true })
        },
      }
    },
  },
]

describe.each(backends)('$name', ({ create }) => {
  let g: GraphBackend
  let cleanup: (() => Promise<void>) | undefined

  beforeEach(async () => {
    const fixture = await create()
    g = fixture.backend
    cleanup = fixture.cleanup
    await g.load()
  })

  afterEach(async () => {
    if (cleanup) await cleanup()
  })

  it('adds nodes and edges and indexes both directions', async () => {
    const alice = await g.addNode({ id: 'u:alice', labels: ['User'], props: { name: 'Alice' } })
    const bob = await g.addNode({ id: 'u:bob', labels: ['User'], props: { name: 'Bob' } })
    const e = await g.addEdge({ from: alice.id, to: bob.id, rel: 'KNOWS' })
    expect(await g.size()).toEqual({ nodes: 2, edges: 1 })
    expect(await g.query({ from: alice.id })).toHaveLength(1)
    expect(await g.query({ to: bob.id })).toHaveLength(1)
    expect(await g.query({ rel: 'FOLLOWS' })).toHaveLength(0)
    void e
  })

  it('rejects edges to unknown nodes', async () => {
    await g.addNode({ id: 'a', labels: ['X'] })
    await expect(g.addEdge({ from: 'a', to: 'ghost', rel: 'R' })).rejects.toThrow(/unknown to-node/)
  })

  it('supersede marks prior valid edges invalid and points them at the new one', async () => {
    await g.addNode({ id: 'u', labels: ['User'] })
    await g.addNode({ id: 'city:paris', labels: ['City'], props: { name: 'Paris' } })
    await g.addNode({ id: 'city:berlin', labels: ['City'], props: { name: 'Berlin' } })

    const first = await g.addEdge({ from: 'u', to: 'city:paris', rel: 'LIVES_IN', valid_from: '2022-01-01T00:00:00Z' })
    const second = await g.addEdge({ from: 'u', to: 'city:paris', rel: 'LIVES_IN', valid_from: '2024-06-01T00:00:00Z', supersede: true })

    // Default query hides retired edges.
    const current = await g.query({ from: 'u', rel: 'LIVES_IN' })
    expect(current.map((e) => e.to)).toEqual(['city:paris']) // just the new one
    expect(current).toHaveLength(1)
    expect(current[0]!.id).toBe(second.id)

    // Verify history if the backend supports it (all current ones do)
    const retired = (await g.query({ from: 'u', rel: 'LIVES_IN', includeInvalid: true })).find(e => e.id === first.id)!
    expect(retired.valid_to).toBe(second.valid_from)
    expect(retired.superseded_by).toBe(second.id)
  })

  it('supersede does NOT touch edges with different (from,to,rel)', async () => {
    await g.addNode({ id: 'u', labels: ['User'] })
    await g.addNode({ id: 'p', labels: ['City'] })
    await g.addNode({ id: 'b', labels: ['City'] })
    const a = await g.addEdge({ from: 'u', to: 'p', rel: 'LIVES_IN' })
    await g.addEdge({ from: 'u', to: 'b', rel: 'VISITED', supersede: true }) // different to+rel
    
    const edge = (await g.query({ from: 'u', to: 'p', rel: 'LIVES_IN' }))[0]!
    expect(edge.valid_to).toBeNull()
  })

  it('asOf returns edges valid at that point in time', async () => {
    await g.addNode({ id: 'u', labels: ['User'] })
    await g.addNode({ id: 'p', labels: ['City'] })
    await g.addNode({ id: 'b', labels: ['City'] })
    await g.addEdge({ from: 'u', to: 'p', rel: 'LIVES_IN', valid_from: '2022-01-01T00:00:00Z' })
    await g.addEdge({ from: 'u', to: 'b', rel: 'LIVES_IN', valid_from: '2024-06-01T00:00:00Z', supersede: true })

    const in2023 = await g.query({ from: 'u', rel: 'LIVES_IN', asOf: '2023-06-01T00:00:00Z' })
    expect(in2023.map((e) => e.to)).toEqual(['p'])

    const in2025 = await g.query({ from: 'u', rel: 'LIVES_IN', asOf: '2025-01-01T00:00:00Z' })
    expect(in2025.map((e) => e.to)).toEqual(['b'])
  })

  it('retractEdge invalidates but preserves history', async () => {
    await g.addNode({ id: 'a', labels: ['X'] })
    await g.addNode({ id: 'b', labels: ['X'] })
    const e = await g.addEdge({ from: 'a', to: 'b', rel: 'R' })
    const retracted = await g.retractEdge(e.id, '2025-01-01T00:00:00Z')
    expect(retracted?.valid_to).toBe('2025-01-01T00:00:00Z')

    expect(await g.query({ from: 'a' })).toHaveLength(0) // hidden
    expect(await g.query({ from: 'a', includeInvalid: true })).toHaveLength(1)
  })

  it('neighbors() BFS respects depth + relation + direction', async () => {
    for (const id of ['a', 'b', 'c', 'd']) await g.addNode({ id, labels: ['X'] })
    await g.addEdge({ from: 'a', to: 'b', rel: 'R' })
    await g.addEdge({ from: 'b', to: 'c', rel: 'R' })
    await g.addEdge({ from: 'c', to: 'd', rel: 'R' })

    const depth1 = await g.neighbors('a', { depth: 1 })
    expect(depth1.map((n) => n.node.id)).toEqual(['b'])

    const depth2 = await g.neighbors('a', { depth: 2 })
    expect(depth2.map((n) => n.node.id).sort()).toEqual(['b', 'c'])

    const depth3 = await g.neighbors('a', { depth: 3 })
    expect(depth3.map((n) => n.node.id).sort()).toEqual(['b', 'c', 'd'])

    // 'in' direction from 'd'
    const inbound = await g.neighbors('d', { depth: 3, direction: 'in' })
    expect(inbound.map((n) => n.node.id)).toEqual(['c', 'b', 'a'])
  })

  it('neighbors path carries the edge sequence', async () => {
    for (const id of ['a', 'b', 'c']) await g.addNode({ id, labels: ['X'] })
    const e1 = await g.addEdge({ from: 'a', to: 'b', rel: 'R' })
    const e2 = await g.addEdge({ from: 'b', to: 'c', rel: 'R' })
    const depth2 = await g.neighbors('a', { depth: 2 })
    const c = depth2.find((n) => n.node.id === 'c')!
    expect(c.path.map((e) => e.id)).toEqual([e1.id, e2.id])
  })
})

