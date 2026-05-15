import { describe, expect, it } from 'vitest'
import { evaluatePolicy } from '../../src/policy/pdp.js'
import { makeSubject, makeResource, makeContext } from '../../src/policy/types.js'
import { type PolicySet } from '../../src/policy/loader.js'

describe('PDP evaluatePolicy', () => {
  const policySet: PolicySet = {
    version: 1,
    policies: [
      {
        id: 'P1',
        rules: [
          {
            id: 'R1-deny-hr',
            effect: 'deny',
            match: { action: ['read'] },
            condition: { 'resource.attributes.department': { eq: 'HR' } }
          },
          {
            id: 'R2-permit-all',
            effect: 'permit',
            priority: 0,
            rules: [] // Should be ignored or handle empty rules
          } as any
        ]
      }
    ]
  }

  it('permits by default when no rules match', async () => {
    const s = makeSubject('user', 'alice')
    const r = makeResource('atom', 'A1')
    const ctx = makeContext('http', 't1')
    
    const decision = evaluatePolicy(s, r, 'recall', ctx, { version: 1, policies: [] })
    expect(decision.effect).toBe('permit')
    expect(decision.reasoning.some(t => t.description.includes('Default posture'))).toBe(true)
  })

  it('denies when a deny rule matches', async () => {
    const s = makeSubject('user', 'alice')
    const r = makeResource('atom', 'A1', {}, { department: 'HR' })
    const ctx = makeContext('http', 't1')
    
    const decision = evaluatePolicy(s, r, 'read', ctx, policySet)
    expect(decision.effect).toBe('deny')
    expect(decision.reasoning.find(t => t.rule_id === 'R1-deny-hr')?.matched).toBe(true)
  })

  it('honors rule priority (deny wins over permit if higher priority)', async () => {
    const ps: PolicySet = {
      version: 1,
      policies: [
        {
          id: 'P1',
          rules: [
            { id: 'LOW', effect: 'permit', priority: 10 },
            { id: 'HIGH', effect: 'deny', priority: 100 }
          ]
        }
      ]
    }
    const s = makeSubject('user', 'alice')
    const r = makeResource('atom', 'A1')
    const ctx = makeContext('http', 't1')
    
    const decision = evaluatePolicy(s, r, 'read', ctx, ps)
    expect(decision.effect).toBe('deny')
  })
})
