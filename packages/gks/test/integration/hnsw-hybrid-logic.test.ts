import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { createGenesisGraphBackend } from '../../src/memory/graph/genesis-graph.js'

describe('GenesisDB HNSW Hybrid Search Logic', () => {
  let dir = ''
  
  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), 'gks-hybrid-test-'))
    process.env.GKS_FORCE_NATIVE_TEST = 'true'
  })
  
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true })
    delete process.env.GKS_FORCE_NATIVE_TEST
  })

  it('balances similarity and impact based on alpha', async () => {
    const backend = createGenesisGraphBackend({ path: dir })
    await backend.load()

    // Node A: Exact match for query, but 'draft' status (Low Impact)
    // Status 'draft' = 0.4 SC
    await backend.addNode({
      id: 'NODE--A',
      labels: ['Concept'],
      props: { status: 'draft', embedding: [1, 0, 0, 0] }
    })

    // Node B: High similarity (0.8), but 'stable' status (High Impact)
    // Status 'stable' = 1.0 SC
    await backend.addNode({
      id: 'NODE--B',
      labels: ['Concept'],
      props: { status: 'stable', embedding: [0.8, 0.2, 0, 0] }
    })

    // Add an incoming edge to Node B to boost DD impact
    // 1 incoming edge = 0.1 DD
    await backend.addNode({ id: 'NODE--C', labels: ['Source'], props: {} })
    await backend.addEdge({ from: 'NODE--C', to: 'NODE--B', rel: 'cites' })

    const queryVector = [1, 0, 0, 0]

    // Test 1: Pure Semantic (alpha = 0)
    // Node A should be #1
    const resSemantic = await backend.hybridSearch({
      queryVector,
      k: 2,
      alpha: 0.0
    })
    
    expect(resSemantic).toHaveLength(2)
    expect(resSemantic[0].node.id).toBe('NODE--A')

    // Test 2: Hybrid (alpha = 0.9)
    // Node B should be #1 due to significantly higher K-Impact (1.0 vs 0.4)
    const resHybrid = await backend.hybridSearch({
      queryVector,
      k: 2,
      alpha: 0.9
    })

    expect(resHybrid).toHaveLength(2)
    expect(resHybrid[0].node.id).toBe('NODE--B')
  })
})
