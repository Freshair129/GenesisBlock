import { describe, expect, it, beforeAll } from 'vitest'
import { hydrateSubject } from '../../src/policy/subject.js'
import { evaluatePolicy } from '../../src/policy/pdp.js'
import { makeResource, makeContext } from '../../src/policy/types.js'
import { loadPolicies } from '../../src/policy/loader.js'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { Identity } from '../../src/identity/types.js'

const __dirname = dirname(fileURLToPath(import.meta.url))

describe('UCF Phase 4: User-level ABAC', () => {
  let policySet: any

  beforeAll(async () => {
    // Load real policies from the project root
    const policiesDir = resolve(__dirname, '../../../../policies')
    policySet = await loadPolicies(policiesDir)
  })

  it('hydrates subject correctly from identity', () => {
    const identity: Identity = {
      schemaVersion: 1,
      profile: {
        name: 'alice',
        role: 'researcher',
        tier: 'T2',
        originStory: 'test',
        createdAt: '2026-05-17',
        guardrails: [],
        extensions: {},
        roles: ['admin'],
        clearance: 50,
        mfaStatus: true,
        tenantIds: ['T1', 'T2']
      },
      voice: { tone: [], formality: 'neutral', languagePreference: 'auto', responseCadence: 'normal' },
      preferences: {}
    }
    
    const subject = hydrateSubject(identity)
    expect(subject.kind).toBe('user')
    expect(subject.id).toBe('alice')
    expect(subject.attributes.roles).toEqual(['admin'])
    expect(subject.attributes.clearance).toBe(50)
    expect(subject.attributes.mfa_status).toBe(true)
    expect(subject.attributes.tenant_ids).toEqual(['T1', 'T2'])
  })

  it('enforces multi-tenant isolation', () => {
    const identity: Identity = {
      schemaVersion: 1,
      profile: {
        name: 'alice',
        role: 'user',
        tier: 'T2',
        originStory: '',
        createdAt: '',
        guardrails: [],
        extensions: {},
        roles: [],
        clearance: 0,
        mfaStatus: false,
        tenantIds: ['T1']
      },
      voice: { tone: [], formality: 'neutral', languagePreference: 'auto', responseCadence: 'normal' },
      preferences: {}
    }
    const alice = hydrateSubject(identity)
    
    const context = makeContext('http', 't1')
    
    // Resource in same tenant
    const res1 = makeResource('atom', 'A1', {}, { tenant_id: 'T1' })
    const dec1 = evaluatePolicy(alice, res1, 'read', context, policySet)
    expect(dec1.effect).toBe('permit')
    
    // Resource in different tenant
    const res2 = makeResource('atom', 'A2', {}, { tenant_id: 'T2' })
    const dec2 = evaluatePolicy(alice, res2, 'read', context, policySet)
    expect(dec2.effect).toBe('deny')
    expect(dec2.reasoning.some(r => r.rule_id === 'multi-tenant-isolation')).toBe(true)
  })

  it('blocks PII (SSN) from LLM exposure', () => {
    const identity: Identity = {
      schemaVersion: 1,
      profile: {
        name: 'alice',
        role: 'user',
        tier: 'T2',
        originStory: '',
        createdAt: '',
        guardrails: [],
        extensions: {},
        roles: [],
        clearance: 0,
        mfaStatus: false,
        tenantIds: []
      },
      voice: { tone: [], formality: 'neutral', languagePreference: 'auto', responseCadence: 'normal' },
      preferences: {}
    }
    const alice = hydrateSubject(identity)
    
    const context = makeContext('http', 't1')
    
    // Clean body
    const res1 = makeResource('atom', 'A1', {}, { body: 'Hello world' })
    const dec1 = evaluatePolicy(alice, res1, 'expose-to-llm', context, policySet)
    expect(dec1.effect).toBe('permit')
    
    // SSN body
    const res2 = makeResource('atom', 'A2', {}, { body: 'My SSN is 123-45-6789' })
    const dec2 = evaluatePolicy(alice, res2, 'expose-to-llm', context, policySet)
    expect(dec2.effect).toBe('deny')
    expect(dec2.reasoning.some(r => r.rule_id === 'pii-block-ssn')).toBe(true)
  })
})
