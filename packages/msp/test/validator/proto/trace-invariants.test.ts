import { describe, expect, it } from 'vitest'

import predicate from '../../../src/validator/proto/trace-invariants.js'
import type { SymbolGraphReaderLike } from '../../../src/validator/proto/symbol-graph-reader.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'
import type { Edge, EdgeType, Symbol as SymbolNode, SymbolKind } from '../../../src/symbols/types.js'

function atom(
  id: string,
  crosslinks?: Record<string, string[]>,
): AtomicIndexEntry {
  return {
    id,
    type: id.split('--')[0]!.toLowerCase(),
    status: 'stable',
    path: `gks/${id}.md`,
    phase: 0,
    vault_id: 'default',
    crosslinks,
  }
}

function sym(id: string, kind: SymbolKind = 'function'): SymbolNode {
  return {
    id,
    name: id,
    kind,
    file: 'fake.ts',
    start_line: 1,
    end_line: 1,
    exported: false,
    parent_id: null,
    signature: null,
    community_id: null,
    created_at: '2026-05-16T00:00:00.000+07:00',
  }
}

function edge(src_id: string, dst_id: string, type: EdgeType = 'calls', resolved = true): Edge {
  return { src_id, dst_id, type, weight: 1.0, resolved }
}

function mockGraph(parts: Partial<SymbolGraphReaderLike>): SymbolGraphReaderLike {
  return {
    allSymbols: () => [],
    allEdges: () => [],
    getSymbol: () => null,
    getOutgoingEdges: () => [],
    getNeighbors: () => ({ nodes: [], edges: [] }),
    ...parts,
  }
}

describe('PROTO--SYMBOLS-TRACE-INVARIANTS predicate', () => {
  describe('Rule 2: Acyclic Constraint', () => {
    it('passes when graph is acyclic', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { supersedes: ['CONCEPT--B'] }),
          atom('CONCEPT--B', { implements: ['FEAT--C'] }),
          atom('FEAT--C'),
        ],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      const errors = result.violations.filter((v) => v.severity === 'error')
      expect(errors).toEqual([])
      expect(result.ok).toBe(true)
    })

    it('errors on a direct self-loop', async () => {
      const result = await predicate({
        atomicIndex: [atom('CONCEPT--A', { supersedes: ['CONCEPT--A'] })],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      expect(result.ok).toBe(false)
      const v = result.violations.find((v) => v.rule === 'acyclic-constraint')
      expect(v?.message).toMatch(/CONCEPT--A → CONCEPT--A/)
    })

    it('errors on a simple cycle (A -> B -> A)', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { supersedes: ['CONCEPT--B'] }),
          atom('CONCEPT--B', { implements: ['CONCEPT--A'] }),
        ],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      const v = result.violations.find((v) => v.rule === 'acyclic-constraint')
      expect(v?.message).toMatch(/CONCEPT--A → CONCEPT--B → CONCEPT--A/)
    })

    it('errors on a complex cycle with mixed edge types', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { supersedes: ['CONCEPT--B'] }),
          atom('CONCEPT--B', { implements: ['BLUEPRINT--C'] }),
          atom('BLUEPRINT--C', { parent_blueprint: ['CONCEPT--A'] }),
        ],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      const v = result.violations.find((v) => v.rule === 'acyclic-constraint')
      expect(v?.message).toMatch(/CONCEPT--A → CONCEPT--B → BLUEPRINT--C → CONCEPT--A/)
    })

    it('detects multiple disjoint cycles', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('A--ONE', { supersedes: ['B--TWO'] }),
          atom('B--TWO', { supersedes: ['A--ONE'] }),
          atom('C--THREE', { implements: ['D--FOUR'] }),
          atom('D--FOUR', { implements: ['C--THREE'] }),
        ],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      const acyclicViolations = result.violations.filter((v) => v.rule === 'acyclic-constraint')
      expect(acyclicViolations).toHaveLength(2)
    })

    it('ignores non-tracked crosslink edges', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { references: ['CONCEPT--B'] }),
          atom('CONCEPT--B', { references: ['CONCEPT--A'] }),
        ],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      expect(result.violations.some((v) => v.rule === 'acyclic-constraint')).toBe(false)
    })
  })

  describe('Rule 4b: Atom Referential Integrity', () => {
    it('passes when all targets exist', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { references: ['CONCEPT--B'] }),
          atom('CONCEPT--B'),
        ],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      const refErrors = result.violations.filter((v) => v.rule === 'atom-ref-integrity')
      expect(refErrors).toEqual([])
    })

    it('errors when a target is missing', async () => {
      const result = await predicate({
        atomicIndex: [atom('CONCEPT--A', { references: ['CONCEPT--GHOST'] })],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      const v = result.violations.find((v) => v.rule === 'atom-ref-integrity')
      expect(v?.message).toMatch(/references missing target CONCEPT--GHOST/)
      expect(v?.atomId).toBe('CONCEPT--A')
    })

    it('scans every crosslink key, not just known ones', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', {
            some_custom_key: ['CONCEPT--GHOST'],
            references: ['CONCEPT--B'],
          }),
          atom('CONCEPT--B'),
        ],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      const refErrors = result.violations.filter((v) => v.rule === 'atom-ref-integrity')
      expect(refErrors).toHaveLength(1)
      expect(refErrors[0]?.message).toMatch(/some_custom_key/)
    })

    it('halts after 50 violations with a summary entry', async () => {
      const ghostRefs = Array.from({ length: 60 }, (_, i) => `GHOST--${i}`)
      const result = await predicate({
        atomicIndex: [atom('ROOT--A', { references: ghostRefs })],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      const refErrors = result.violations.filter((v) => v.rule === 'atom-ref-integrity')
      expect(refErrors.length).toBeLessThanOrEqual(51) // 50 + 1 summary
      expect(refErrors[refErrors.length - 1]?.message).toMatch(/halted after 50 violations/)
    })
  })

  describe('Rule 4a: Symbol Referential Integrity', () => {
    it('emits an info note when symbolGraph is null', async () => {
      const result = await predicate({
        atomicIndex: [],
        repoRoot: '/tmp',
        symbolGraph: null,
      })
      expect(result.ok).toBe(true)
      expect(result.violations).toHaveLength(1)
      expect(result.violations[0]?.severity).toBe('info')
      expect(result.violations[0]?.message).toMatch(/Symbol graph DB unavailable/)
    })

    it('passes when every resolved edge has a dst symbol', async () => {
      const graph = mockGraph({
        allEdges: () => [edge('a', 'b'), edge('b', 'c')],
        getSymbol: (id) => (id === 'b' || id === 'c' ? sym(id) : null),
      })
      const result = await predicate({ atomicIndex: [], repoRoot: '/tmp', symbolGraph: graph })
      const symErrors = result.violations.filter((v) => v.rule === 'symbol-ref-integrity')
      expect(symErrors).toEqual([])
    })

    it('errors when a resolved edge points to a missing symbol', async () => {
      const graph = mockGraph({
        allEdges: () => [edge('src', 'missing', 'calls', true)],
        getSymbol: () => null,
      })
      const result = await predicate({ atomicIndex: [], repoRoot: '/tmp', symbolGraph: graph })
      const v = result.violations.find((v) => v.rule === 'symbol-ref-integrity')
      expect(v?.severity).toBe('error')
      expect(v?.message).toMatch(/missing symbol/)
    })

    it('skips unresolved edges', async () => {
      const graph = mockGraph({
        allEdges: () => [edge('src', 'dynamic', 'calls', false)],
        getSymbol: () => null,
      })
      const result = await predicate({ atomicIndex: [], repoRoot: '/tmp', symbolGraph: graph })
      const symErrors = result.violations.filter((v) => v.rule === 'symbol-ref-integrity')
      expect(symErrors).toEqual([])
    })

    it('downgrades severity to warning when violations exceed 100', async () => {
      const edges = Array.from({ length: 105 }, (_, i) => edge('src', `missing_${i}`))
      const graph = mockGraph({
        allEdges: () => edges,
        getSymbol: () => null,
      })
      const result = await predicate({ atomicIndex: [], repoRoot: '/tmp', symbolGraph: graph })
      expect(result.ok).toBe(true)
      const symViolations = result.violations.filter((v) => v.rule === 'symbol-ref-integrity')
      expect(symViolations.some((v) => v.severity === 'error')).toBe(false)
      expect(symViolations[symViolations.length - 1]?.message).toMatch(/severities downgraded to warning/)
    })
  })
})
