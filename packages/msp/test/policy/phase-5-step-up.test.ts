import { describe, expect, it, beforeAll, afterAll } from 'vitest'
import { PinProvider } from '../../src/policy/step-up/pin-provider.js'
import { enforcePolicy } from '../../src/policy/pep.js'
import { makeSubject, makeResource, makeContext } from '../../src/policy/types.js'
import { loadPolicies } from '../../src/policy/loader.js'
import { resolve, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { mkdir, rm, writeFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'

const __dirname = dirname(fileURLToPath(import.meta.url))
const TEST_MSP_HOME = resolve(__dirname, '../../../.tmp-msp-home-step-up')

describe('UCF Phase 5: Step-up Auth', () => {
  let policySet: any
  const root = resolve(__dirname, '../../..')

  beforeAll(async () => {
    // Setup clean MSP_HOME
    process.env.MSP_HOME = TEST_MSP_HOME
    await mkdir(TEST_MSP_HOME, { recursive: true })

    // Load real policies (including the new 50-step-up.yaml)
    const policiesDir = resolve(process.cwd(), 'policies')
    policySet = await loadPolicies(policiesDir)
  })

  afterAll(async () => {
    await rm(TEST_MSP_HOME, { recursive: true, force: true })
  })

  it('triggers step-up advice on sensitive action (delete)', async () => {
    const s = makeSubject('user', 'alice')
    const r = makeResource('atom', 'A1')
    const ctx = makeContext('http', 't1')

    const result = await enforcePolicy(r, { root, subject: s, action: 'delete', context: ctx })
    
    expect(result.permitted).toBe(false)
    expect(result.requiresStepUp).toBe(true)
    expect(result.stepUpParams?.method).toBe('pin')
  })

  it('completes full step-up cycle (challenge -> verify -> permit)', async () => {
    const provider = new PinProvider()
    await provider.setPin('1234')

    const s = makeSubject('user', 'alice')
    const r = makeResource('atom', 'A1')
    const ctx = makeContext('http', 't1')

    // 1. Initial attempt -> Deny with advice
    const res1 = await enforcePolicy(r, { root, subject: s, action: 'delete', context: ctx })
    expect(res1.permitted).toBe(false)

    // 2. Issue challenge
    // action_hash is required for binding
    const actionHash = createHash('sha256').update(JSON.stringify({ s, r, action: 'delete' })).digest('hex')
    const challenge = await provider.challenge(actionHash)
    expect(challenge.challenge_id).toBeDefined()

    // 3. Verify solution
    const vResult = await provider.verify({
      challenge_id: challenge.challenge_id,
      solution: '1234',
      action_hash: actionHash
    })
    expect(vResult.success).toBe(true)

    // 4. Update subject and retry
    const sUpdated = { 
      ...s, 
      last_step_up_at: vResult.verified_at,
      last_step_up_method: 'pin',
      attributes: {
        ...s.attributes,
        last_step_up_at: vResult.verified_at,
        last_step_up_method: 'pin'
      }
    }
    
    // Use a fresh context for the retry
    const ctx2 = makeContext('http', 't2')
    const res2 = await enforcePolicy(r, { root, subject: sUpdated, action: 'delete', context: ctx2 })
    expect(res2.permitted).toBe(true)
  })

  it('rejects wrong PIN', async () => {
    const provider = new PinProvider()
    const actionHash = 'test-hash'
    const challenge = await provider.challenge(actionHash)

    const vResult = await provider.verify({
      challenge_id: challenge.challenge_id,
      solution: 'wrong',
      action_hash: actionHash
    })
    expect(vResult.success).toBe(false)
    expect(vResult.error).toContain('Invalid PIN')
  })

  it('rejects replayed challenge (nonce defense)', async () => {
    const provider = new PinProvider()
    const actionHash = 'test-hash'
    const challenge = await provider.challenge(actionHash)

    // First use
    await provider.verify({ challenge_id: challenge.challenge_id, solution: '1234', action_hash: actionHash })
    
    // Replay
    const vResult = await provider.verify({
      challenge_id: challenge.challenge_id,
      solution: '1234',
      action_hash: actionHash
    })
    expect(vResult.success).toBe(false)
    expect(vResult.error).toContain('Challenge not found or expired')
  })

  it('rejects binding mismatch (action_hash defense)', async () => {
    const provider = new PinProvider()
    const challenge = await provider.challenge('hash-A')

    const vResult = await provider.verify({
      challenge_id: challenge.challenge_id,
      solution: '1234',
      action_hash: 'hash-B' // Mismatch
    })
    expect(vResult.success).toBe(false)
    expect(vResult.error).toContain('binding mismatch')
  })
})
