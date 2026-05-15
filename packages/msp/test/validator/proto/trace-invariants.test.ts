import { describe, expect, it } from 'vitest'

import predicate from '../../../src/validator/proto/trace-invariants.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'

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
  } as AtomicIndexEntry
}

describe('PROTO--SYMBOLS-TRACE-INVARIANTS predicate (Atom Graph Rules)', () => {
  describe('Rule 2: Acyclic Constraint', () => {
    it('passes when graph is acyclic', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { supersedes: ['CONCEPT--B'] }),
          atom('CONCEPT--B', { implements: ['FEAT--C'] }),
          atom('FEAT--C'),
        ],
        repoRoot: '/tmp',
      })
      expect(result.ok).toBe(true)
      expect(result.violations).toEqual([])
    })

    it('errors on a direct self-loop', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { supersedes: ['CONCEPT--A'] }),
        ],
        repoRoot: '/tmp',
      })
      expect(result.ok).toBe(false)
      expect(result.violations[0]?.message).toMatch(/Cycle in atom graph/)
      expect(result.violations[0]?.message).toMatch(/CONCEPT--A → CONCEPT--A/)
    })

    it('errors on a simple cycle (A -> B -> A)', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { supersedes: ['CONCEPT--B'] }),
          atom('CONCEPT--B', { implements: ['CONCEPT--A'] }),
        ],
        repoRoot: '/tmp',
      })
      expect(result.ok).toBe(false)
      expect(result.violations[0]?.message).toMatch(/CONCEPT--A → CONCEPT--B → CONCEPT--A/)
    })

    it('errors on a complex cycle with mixed edge types', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { supersedes: ['CONCEPT--B'] }),
          atom('CONCEPT--B', { implements: ['BLUEPRINT--C'] }),
          atom('BLUEPRINT--C', { parent_blueprint: ['CONCEPT--A'] }),
        ],
        repoRoot: '/tmp',
      })
      expect(result.ok).toBe(false)
      expect(result.violations[0]?.message).toMatch(/CONCEPT--A → CONCEPT--B → BLUEPRINT--C → CONCEPT--A/)
    })

    it('detects multiple disjoint cycles', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('A', { supersedes: ['B'] }),
          atom('B', { supersedes: ['A'] }),
          atom('C', { implements: ['D'] }),
          atom('D', { implements: ['C'] }),
        ],
        repoRoot: '/tmp',
      })
      expect(result.ok).toBe(false)
      expect(result.violations).toHaveLength(2)
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
      })
      expect(result.ok).toBe(true)
    })

    it('errors when a target is missing', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { references: ['CONCEPT--GHOST'] }),
        ],
        repoRoot: '/tmp',
      })
      expect(result.ok).toBe(false)
      expect(result.violations[0]?.message).toMatch(/references missing target CONCEPT--GHOST/)
    })

    it('scans all keys in crosslinks', async () => {
      const result = await predicate({
        atomicIndex: [
          atom('CONCEPT--A', { 
            any_random_key: ['CONCEPT--GHOST'],
            another_key: ['CONCEPT--B'] 
          }),
          atom('CONCEPT--B'),
        ],
        repoRoot: '/tmp',
      })
      expect(result.ok).toBe(false)
      expect(result.violations).toHaveLength(1)
      expect(result.violations[0]?.message).toMatch(/any_random_key/)
    })

    it('halts after 50 violations', async () => {
      const largeIndex = [atom('ROOT', { refs: Array.from({length: 60}, (_, i) => `MISSING--${i}`) })]
      const result = await predicate({
        atomicIndex: largeIndex,
        repoRoot: '/tmp',
      })
      expect(result.ok).toBe(false)
      // 51 because the last one is the "halted" message
      expect(result.violations.length).toBe(52) 
      expect(result.violations[result.violations.length - 1]?.message).toMatch(/Referential integrity check halted/)
    })
  })

  describe('Symbol Graph Rules (1, 3, 4a)', () => {
    it('gracefully skips when symbolGraph is null', async () => {
      const result = await predicate({
        atomicIndex: [],
        repoRoot: '/tmp',
        symbolGraph: null,
      } as any)
      expect(result.ok).toBe(true)
      expect(result.violations[0]?.severity).toBe('info')
      expect(result.violations[0]?.message).toMatch(/Symbol graph DB unavailable/)
    })

    it('passes when no violations exist', async () => {
      const mockGraph = {
        allSymbols: () => [{ id: 'entry', kind: 'route' }, { id: 'leaf', kind: 'function' }],
        allEdges: () => [{ src_id: 'entry', dst_id: 'leaf', type: 'calls', resolved: true }],
        getSymbol: (id: string) => id === 'entry' || id === 'leaf' ? {} : null,
        getOutgoingEdges: (srcId: string) => srcId === 'entry' ? [{ src_id: 'entry', dst_id: 'leaf', type: 'calls', resolved: true }] : [],
      }
      const result = await predicate({
        atomicIndex: [],
        repoRoot: '/tmp',
        symbolGraph: mockGraph,
      } as any)
      expect(result.ok).toBe(true)
    })

    it('Rule 4a: errors on missing resolved dst_id', async () => {
      const mockGraph = {
        allSymbols: () => [],
        allEdges: () => [{ src_id: 'src', dst_id: 'missing', type: 'calls', resolved: true }],
        getSymbol: () => null,
        getOutgoingEdges: () => [],
      }
      const result = await predicate({
        atomicIndex: [],
        repoRoot: '/tmp',
        symbolGraph: mockGraph,
      } as any)
      expect(result.ok).toBe(false)
      expect(result.violations[0]?.severity).toBe('error')
      expect(result.violations[0]?.message).toMatch(/points to missing symbol/)
    })

    it('Rule 4a: downgrades to warning if over 100 violations', async () => {
      const edges = Array.from({length: 105}, (_, i) => ({ src_id: 'src', dst_id: `missing_${i}`, type: 'calls', resolved: true }))
      const mockGraph = {
        allSymbols: () => [],
        allEdges: () => edges,
        getSymbol: () => null,
        getOutgoingEdges: () => [],
      }
      const result = await predicate({
        atomicIndex: [],
        repoRoot: '/tmp',
        symbolGraph: mockGraph,
      } as any)
      // Downgrades to warning, so ok is true
      expect(result.ok).toBe(true)
      expect(result.violations.some(v => v.severity === 'error')).toBe(false)
      expect(result.violations[0]?.message).toMatch(/points to missing symbol/)
      expect(result.violations[result.violations.length - 1]?.message).toMatch(/Over 100 threshold reached/)
    })

    it('Rule 1: detects termination naturally (depth or cycle)', async () => {
      const mockGraph = {
        allSymbols: () => [{ id: 'entry', kind: 'page' }, { id: 'a', kind: 'function' }],
        allEdges: () => [],
        getSymbol: (id: string) => ({ id }),
        getOutgoingEdges: (srcId: string) => {
          if (srcId === 'entry') return [{ src_id: 'entry', dst_id: 'a', type: 'calls', resolved: true }]
          if (srcId === 'a') return [{ src_id: 'a', dst_id: 'entry', type: 'calls', resolved: true }] // cycle
          return []
        },
      }
      const result = await predicate({
        atomicIndex: [],
        repoRoot: '/tmp',
        symbolGraph: mockGraph,
      } as any)
      expect(result.ok).toBe(true) // Cycles are valid terminations in Rule 1
    })
  })
})
