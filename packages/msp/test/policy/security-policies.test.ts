import { describe, expect, it, beforeAll } from 'vitest'
import { enforcePolicy } from '../../src/policy/pep.js'
import { makeSubject, makeResource, makeContext } from '../../src/policy/types.js'
import { loadPolicies } from '../../src/policy/loader.js'
import { resolve } from 'node:path'

describe('Security Domain Pack: Policies', () => {
  let policySet: any
  const root = resolve(process.cwd())

  beforeAll(async () => {
    const policiesDir = resolve(root, 'policies')
    policySet = await loadPolicies(policiesDir)
  })

  it('blocks secrets from cloud-tier agents (T2)', async () => {
    const s = makeSubject('subagent', 'gemini', { tier: 'T2' })
    const r = makeResource('atom', 'SECRET_ATOM', {}, { has_secret: true })
    const ctx = makeContext('mcp-stdio', 't1')

    const result = await enforcePolicy(r, { root, subject: s, action: 'expose-to-llm', context: ctx })
    expect(result.permitted).toBe(false)
    expect(result.decision.reasoning.some(r => r.rule_id === 'block-secrets-from-cloud')).toBe(true)
  })

  it('requires Step-up for vaulted secrets', async () => {
    const s = makeSubject('user', 'alice')
    const r = makeResource('atom', 'VAULT_ATOM', {}, { encryption_level: 'vault' })
    const ctx = makeContext('http', 't1')

    const result = await enforcePolicy(r, { root, subject: s, action: 'modify', context: ctx })
    expect(result.permitted).toBe(false)
    expect(result.requiresStepUp).toBe(true)
  })
})
