import { describe, expect, it } from 'vitest'
import { evaluateOperator, evaluateCondition } from '../../src/policy/operators.js'

describe('policy operators', () => {
  describe('evaluateOperator', () => {
    it('eq', () => {
      expect(evaluateOperator('eq', 1, 1)).toBe(true)
      expect(evaluateOperator('eq', 1, 2)).toBe(false)
      expect(evaluateOperator('eq', 'a', 'a')).toBe(true)
    })

    it('ne', () => {
      expect(evaluateOperator('ne', 1, 2)).toBe(true)
      expect(evaluateOperator('ne', 1, 1)).toBe(false)
    })

    it('in', () => {
      expect(evaluateOperator('in', 'a', ['a', 'b'])).toBe(true)
      expect(evaluateOperator('in', 'c', ['a', 'b'])).toBe(false)
    })

    it('ni', () => {
      expect(evaluateOperator('ni', 'c', ['a', 'b'])).toBe(true)
      expect(evaluateOperator('ni', 'a', ['a', 'b'])).toBe(false)
    })

    it('contains', () => {
      expect(evaluateOperator('contains', ['a', 'b'], 'a')).toBe(true)
      expect(evaluateOperator('contains', ['a', 'b'], 'c')).toBe(false)
    })

    it('gt, ge, lt, le', () => {
      expect(evaluateOperator('gt', 2, 1)).toBe(true)
      expect(evaluateOperator('ge', 2, 2)).toBe(true)
      expect(evaluateOperator('lt', 1, 2)).toBe(true)
      expect(evaluateOperator('le', 2, 2)).toBe(true)
    })

    it('exists, not_exists', () => {
      expect(evaluateOperator('exists', 'val', null)).toBe(true)
      expect(evaluateOperator('exists', null, null)).toBe(false)
      expect(evaluateOperator('not_exists', null, null)).toBe(true)
      expect(evaluateOperator('not_exists', 'val', null)).toBe(false)
    })

    it('matches', () => {
      expect(evaluateOperator('matches', 'alice@google.com', '^.*@google.com$')).toBe(true)
      expect(evaluateOperator('matches', 'alice@other.com', '^.*@google.com$')).toBe(false)
    })

    it('intersect', () => {
      expect(evaluateOperator('intersect', ['a', 'b'], ['b', 'c'])).toBe(true)
      expect(evaluateOperator('intersect', ['a', 'b'], ['c', 'd'])).toBe(false)
    })
  })

  describe('evaluateCondition', () => {
    const data = {
      subject: { id: 'alice', role: 'admin', groups: ['hr', 'it'] },
      resource: { kind: 'atom', sensitivity: 'high' },
      context: { origin: 'http' }
    }

    it('simple path match', () => {
      expect(evaluateCondition({ 'subject.id': { eq: 'alice' } }, data)).toBe(true)
      expect(evaluateCondition({ 'resource.sensitivity': { eq: 'low' } }, data)).toBe(false)
    })

    it('all_of', () => {
      expect(evaluateCondition({
        all_of: [
          { 'subject.role': { eq: 'admin' } },
          { 'context.origin': { eq: 'http' } }
        ]
      }, data)).toBe(true)
    })

    it('any_of', () => {
      expect(evaluateCondition({
        any_of: [
          { 'subject.role': { eq: 'user' } },
          { 'subject.id': { eq: 'alice' } }
        ]
      }, data)).toBe(true)
    })

    it('nested path and operators', () => {
      expect(evaluateCondition({
        'subject.groups': { contains: 'hr' },
        'resource.sensitivity': { ne: 'low' }
      }, data)).toBe(true)
    })
  })
})
