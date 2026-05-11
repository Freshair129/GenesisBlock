import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import * as symbolCommunity from '../../src/mcp/tools/symbol-community.js'
import * as symbolImpact from '../../src/mcp/tools/symbol-impact.js'
import * as symbolLookup from '../../src/mcp/tools/symbol-lookup.js'
import * as symbolNeighbors from '../../src/mcp/tools/symbol-neighbors.js'
import * as symbolSearch from '../../src/mcp/tools/symbol-search.js'
import { detectCommunities } from '../../src/symbols/communities/leiden.js'
import { deriveLabel } from '../../src/symbols/communities/leiden.js'
import { parseFile } from '../../src/symbols/parser/typescript.js'
import { dumpJsonl } from '../../src/symbols/store/jsonl.js'
import { SymbolStore } from '../../src/symbols/store/sqlite.js'
import type { Edge, Symbol } from '../../src/symbols/types.js'
import { dbPath, symbolsDir } from '../../src/symbols/util.js'

let workDir: string

interface ToolTextResult {
  content: Array<{ type: 'text'; text: string }>
  isError?: boolean
}

function parseJsonResult<T>(r: ToolTextResult): T {
  return JSON.parse(r.content[0].text) as T
}

async function buildFixture(root: string): Promise<{ alphaId: string; helperId: string }> {
  // Three files: foo.ts has alpha() that calls helper(); bar.ts has Bar/beta;
  // baz.ts also calls alpha() so impact has multiple distances.
  mkdirSync(join(root, 'src'), { recursive: true })
  writeFileSync(
    join(root, 'src', 'foo.ts'),
    `export function helper(): number {
  return 42
}

export function alpha(): number {
  return helper() + 1
}
`,
    'utf8',
  )
  writeFileSync(
    join(root, 'src', 'bar.ts'),
    `export class Bar {
  beta(): string {
    return 'beta'
  }
}
`,
    'utf8',
  )
  writeFileSync(
    join(root, 'src', 'baz.ts'),
    `export function callsAlpha(): number {
  return alpha() * 2
}

declare function alpha(): number
`,
    'utf8',
  )

  // Build symbol graph in-process (avoid spawning tsx in tests for speed).
  const allSymbols: Symbol[] = []
  const allEdges: Edge[] = []
  for (const f of ['foo.ts', 'bar.ts', 'baz.ts']) {
    const r = parseFile(join(root, 'src', f), root)
    allSymbols.push(...r.symbols)
    allEdges.push(...r.edges)
  }
  // Resolve cross-file edges by id presence.
  const ids = new Set(allSymbols.map((s) => s.id))
  for (const e of allEdges) {
    if (!e.resolved && ids.has(e.dst_id)) e.resolved = true
  }

  const dir = symbolsDir(root)
  mkdirSync(dir, { recursive: true })
  const store = new SymbolStore()
  store.open(dbPath(root))
  store.migrate()
  store.clearAll()
  for (const s of allSymbols) store.upsertSymbol(s)
  for (const e of allEdges) store.upsertEdge(e)
  // Communities.
  const det = await detectCommunities(allSymbols, allEdges, { resolution: 1.0, seed: 42 })
  const groups = new Map<number, Symbol[]>()
  for (const s of allSymbols) {
    const cid = det.partition.get(s.id)
    if (cid === undefined) continue
    store.setSymbolCommunity(s.id, cid)
    const arr = groups.get(cid) ?? []
    arr.push(s)
    groups.set(cid, arr)
  }
  for (const [cid, members] of groups) {
    store.upsertCommunity({
      id: cid,
      size: members.length,
      label: deriveLabel(members, allEdges),
      modularity: det.modularity,
      parent_id: null,
    })
  }

  // Stamp the meta sentinel that all 5 tools require.
  store.setMeta('last_built_at', new Date().toISOString())
  store.setMeta('symbol_count', String(allSymbols.length))
  store.setMeta('edge_count', String(allEdges.length))
  store.setMeta('community_count', String(groups.size))
  store.setMeta('parser', 'typescript')
  store.setMeta('algorithm', det.algorithm)
  await dumpJsonl(store, dir)
  store.close()

  const alpha = allSymbols.find((s) => s.name === 'alpha' && s.kind === 'function')
  const helper = allSymbols.find((s) => s.name === 'helper' && s.kind === 'function')
  if (!alpha || !helper) {
    throw new Error('fixture missing alpha or helper symbol')
  }
  return { alphaId: alpha.id, helperId: helper.id }
}

beforeEach(() => {
  workDir = mkdtempSync(join(tmpdir(), 'sg-mcp-'))
})

afterEach(() => {
  rmSync(workDir, { recursive: true, force: true })
})

describe('symbol MCP tools', () => {
  it('msp_symbol_lookup returns hits for a fixture symbol', async () => {
    await buildFixture(workDir)
    const handler = symbolLookup.handler({ root: workDir })
    const result = (await handler({ name: 'alpha', root: workDir })) as ToolTextResult
    expect(result.isError).not.toBe(true)
    const parsed = parseJsonResult<{ ok: boolean; hits: Array<{ name: string; kind: string }> }>(result)
    expect(parsed.ok).toBe(true)
    expect(parsed.hits.length).toBeGreaterThanOrEqual(1)
    expect(parsed.hits[0].name).toBe('alpha')
  }, 30_000)

  it('msp_symbol_neighbors(id, 1) returns the right set', async () => {
    const { alphaId } = await buildFixture(workDir)
    const handler = symbolNeighbors.handler({ root: workDir })
    const result = (await handler({ id: alphaId, depth: 1, root: workDir })) as ToolTextResult
    expect(result.isError).not.toBe(true)
    const parsed = parseJsonResult<{
      ok: boolean
      center: { id: string }
      nodes: Array<{ id: string; name: string }>
      edges: Array<{ src_id: string; dst_id: string }>
    }>(result)
    expect(parsed.ok).toBe(true)
    expect(parsed.center.id).toBe(alphaId)
    // alpha calls helper — helper should be in the neighbor set.
    expect(parsed.nodes.some((n) => n.name === 'helper')).toBe(true)
  }, 30_000)

  it('msp_symbol_impact returns callers', async () => {
    const { helperId } = await buildFixture(workDir)
    const handler = symbolImpact.handler({ root: workDir })
    const result = (await handler({ id: helperId, root: workDir })) as ToolTextResult
    expect(result.isError).not.toBe(true)
    const parsed = parseJsonResult<{
      ok: boolean
      callers: Array<{ symbol: { name: string } | null; distance: number }>
      count: number
    }>(result)
    expect(parsed.ok).toBe(true)
    // alpha calls helper — alpha should appear as a caller.
    expect(parsed.callers.some((c) => c.symbol?.name === 'alpha')).toBe(true)
  }, 30_000)

  it('msp_symbol_community returns members', async () => {
    const { alphaId } = await buildFixture(workDir)
    const handler = symbolCommunity.handler({ root: workDir })
    const result = (await handler({ id: alphaId, root: workDir })) as ToolTextResult
    expect(result.isError).not.toBe(true)
    const parsed = parseJsonResult<{
      ok: boolean
      community: { id: number; size: number }
      members: Array<{ id: string }>
    }>(result)
    expect(parsed.ok).toBe(true)
    expect(parsed.community.size).toBeGreaterThanOrEqual(1)
    expect(parsed.members.some((m) => m.id === alphaId)).toBe(true)
  }, 30_000)

  it('msp_symbol_search ranks by score', async () => {
    await buildFixture(workDir)
    const handler = symbolSearch.handler({ root: workDir })
    const result = (await handler({ query: 'alpha', root: workDir })) as ToolTextResult
    expect(result.isError).not.toBe(true)
    const parsed = parseJsonResult<{
      ok: boolean
      hits: Array<{ name: string; score: number }>
    }>(result)
    expect(parsed.ok).toBe(true)
    expect(parsed.hits.length).toBeGreaterThanOrEqual(1)
    // Top hit should match name "alpha" exactly.
    expect(parsed.hits[0].name).toBe('alpha')
    // Scores should be in descending order.
    for (let i = 1; i < parsed.hits.length; i++) {
      expect(parsed.hits[i - 1].score).toBeGreaterThanOrEqual(parsed.hits[i].score)
    }
  }, 30_000)

  it("all 5 tools return errorResult with 'graph not built' when meta missing", async () => {
    // Fresh tmpdir — no graph has been built.
    const ctx = { root: workDir }
    const tools = [symbolLookup, symbolNeighbors, symbolImpact, symbolCommunity, symbolSearch]
    const argsByName: Record<string, Record<string, unknown>> = {
      msp_symbol_lookup: { name: 'whatever', root: workDir },
      msp_symbol_neighbors: { id: 'src/foo.ts:bar:func', root: workDir },
      msp_symbol_impact: { id: 'src/foo.ts:bar:func', root: workDir },
      msp_symbol_community: { id: 'src/foo.ts:bar:func', root: workDir },
      msp_symbol_search: { query: 'whatever', root: workDir },
    }
    for (const tool of tools) {
      const handler = tool.handler(ctx)
      const args = argsByName[tool.name]
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const result = (await handler(args as any)) as ToolTextResult
      expect(result.isError).toBe(true)
      expect(result.content[0].text).toMatch(/graph not built/)
    }
  })
})
