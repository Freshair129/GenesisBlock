import { scrypt, randomBytes, timingSafeEqual } from 'node:crypto'
import { promisify } from 'node:util'
import { readFile, writeFile, mkdir } from 'node:fs/promises'
import { dirname } from 'node:path'

import { authConfigPath } from '../../lib/msp-home.js'
import type { Challenge, StepUpProvider, VerifyRequest, VerifyResult } from './provider.js'
import { globalChallengeStore, type ChallengeStore } from './challenge-store.js'

const scryptAsync = promisify(scrypt)

export interface AuthConfig {
  pin_hash?: string
  pin_salt?: string
}

/**
 * PIN-based Step-up authentication provider.
 * Implements BLUEPRINT--PHASE-5-STEP-UP-AUTH task T5.3.
 *
 * Uses scrypt (built-in node:crypto) for password hashing.
 * Challenges are issued via globalChallengeStore.
 */
export class PinProvider implements StepUpProvider {
  readonly method = 'pin'

  constructor(private store: ChallengeStore = globalChallengeStore) {}

  /**
   * Set or update the PIN hash.
   * BLUEPRINT task T5.6 helper.
   */
  async setPin(pin: string): Promise<void> {
    const salt = randomBytes(16).toString('hex')
    const hash = (await scryptAsync(pin, salt, 64)) as Buffer
    
    const config: AuthConfig = {
      pin_hash: hash.toString('hex'),
      pin_salt: salt,
    }

    const path = authConfigPath()
    await mkdir(dirname(path), { recursive: true })
    await writeFile(path, JSON.stringify(config, null, 2), 'utf8')
  }

  async challenge(action_hash: string): Promise<Challenge> {
    const stored = this.store.issue(action_hash)
    
    return {
      challenge_id: stored.challenge_id,
      method: 'pin',
      prompt: 'Enter your MSP security PIN to proceed.',
      expires_at: new Date(stored.expires_at).toISOString(),
    }
  }

  async verify(req: VerifyRequest): Promise<VerifyResult> {
    const stored = this.store.get(req.challenge_id)
    
    if (!stored) {
      return { success: false, error: 'Challenge not found or expired.' }
    }

    if (stored.action_hash !== req.action_hash) {
      return { success: false, error: 'Challenge binding mismatch (action_hash).' }
    }

    // Load PIN hash
    let config: AuthConfig
    try {
      const data = await readFile(authConfigPath(), 'utf8')
      config = JSON.parse(data)
    } catch {
      return { success: false, error: 'No PIN set. Use `msp-auth set-pin` first.' }
    }

    if (!config.pin_hash || !config.pin_salt) {
      return { success: false, error: 'No PIN set. Use `msp-auth set-pin` first.' }
    }

    // Verify PIN
    const hash = (await scryptAsync(req.solution, config.pin_salt, 64)) as Buffer
    const expectedHash = Buffer.from(config.pin_hash, 'hex')

    if (!timingSafeEqual(hash, expectedHash)) {
      return { success: false, error: 'Invalid PIN.' }
    }

    // Success
    this.store.consume(req.challenge_id)
    return {
      success: true,
      method: 'pin',
      verified_at: new Date().toISOString(),
    }
  }
}
