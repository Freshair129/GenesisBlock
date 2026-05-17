import { randomUUID } from 'node:crypto'

export interface StoredChallenge {
  challenge_id: string
  action_hash: string
  expires_at: number
  consumed_at?: number
}

/**
 * Server-side store for step-up challenges.
 * Implements TTL expiry and replay defense (consumed_at).
 *
 * BLUEPRINT--PHASE-5-STEP-UP-AUTH task T5.2.
 */
export class ChallengeStore {
  private challenges = new Map<string, StoredChallenge>()
  private ttlMs: number

  /** @param ttlSeconds Default 300 (5 minutes). */
  constructor(ttlSeconds = 300) {
    this.ttlMs = ttlSeconds * 1000
  }

  /** Issue a new challenge ID bound to an action hash. */
  issue(action_hash: string): StoredChallenge {
    const challenge_id = randomUUID()
    const expires_at = Date.now() + this.ttlMs
    const challenge: StoredChallenge = { challenge_id, action_hash, expires_at }
    this.challenges.set(challenge_id, challenge)
    return challenge
  }

  /**
   * Look up and validate a challenge.
   * Checks for existence, expiry, and reuse.
   */
  get(challenge_id: string): StoredChallenge | null {
    const c = this.challenges.get(challenge_id)
    if (!c) return null

    // Expired?
    if (Date.now() > c.expires_at) {
      this.challenges.delete(challenge_id)
      return null
    }

    // Already used? (Replay defense)
    if (c.consumed_at) return null

    return c
  }

  /** Mark a challenge as successfully consumed. */
  consume(challenge_id: string): void {
    const c = this.challenges.get(challenge_id)
    if (c) {
      c.consumed_at = Date.now()
    }
  }

  /** Internal: periodic cleanup of expired/consumed entries. */
  prune(): void {
    const now = Date.now()
    for (const [id, c] of this.challenges.entries()) {
      if (now > c.expires_at || (c.consumed_at && now > c.consumed_at + 60000)) {
        this.challenges.delete(id)
      }
    }
  }
}

/** Default singleton instance. */
export const globalChallengeStore = new ChallengeStore()
