import { describe, expect, it } from 'vitest'
import { makeSubject, makeResource, makeContext, makeDecision } from '../../src/policy/types.js'

describe('policy types', () => {
  describe('makeSubject', () => {
    it('creates a subject with defaults', () => {
      const s = makeSubject('user', 'alice')
      expect(s).toEqual({
        kind: 'user',
        id: 'alice',
        attributes: {},
      })
    })

    it('accepts attributes', () => {
      const s = makeSubject('user', 'alice', { role: 'admin' })
      expect(s.attributes).toEqual({ role: 'admin' })
    })
  })

  describe('makeResource', () => {
    it('creates a resource with defaults', () => {
      const r = makeResource('atom', 'ATOM-1')
      expect(r).toEqual({
        kind: 'atom',
        id: 'ATOM-1',
        namespace: {},
        attributes: {},
      })
    })

    it('accepts namespace and attributes', () => {
      const r = makeResource('atom', 'ATOM-1', { tenant_id: 't1' }, { sensitivity: 'high' })
      expect(r.namespace).toEqual({ tenant_id: 't1' })
      expect(r.attributes).toEqual({ sensitivity: 'high' })
    })
  })

  describe('makeContext', () => {
    it('creates a context with defaults', () => {
      const ctx = makeContext('http', 't-123')
      expect(ctx.origin).toBe('http')
      expect(ctx.trace_id).toBe('t-123')
      expect(ctx.time).toBeInstanceOf(Date)
    })

    it('accepts overrides', () => {
      const time = new Date('2026-05-14T00:00:00Z')
      const ctx = makeContext('cli', 't-456', { time, purpose: 'test' })
      expect(ctx.time).toEqual(time)
      expect(ctx.purpose).toBe('test')
    })
  })

  describe('makeDecision', () => {
    it('creates a permit decision from string reasoning', () => {
      const d = makeDecision('permit', 'matched rule 1')
      expect(d.effect).toBe('permit')
      expect(d.reasoning).toEqual([{ description: 'matched rule 1', matched: true }])
      expect(d.obligations).toEqual([])
    })

    it('accepts structured reasoning', () => {
      const d = makeDecision('deny', [{ rule_id: 'R1', description: 'denied', matched: true }])
      expect(d.effect).toBe('deny')
      expect(d.reasoning[0].rule_id).toBe('R1')
    })
  })
})
