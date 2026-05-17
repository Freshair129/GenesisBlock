/**
 * Step-up authentication method.
 * 'pin' = 4-6 digit numeric/alphanumeric code.
 * 'passkey' = WebAuthn (future).
 * 'signed-token' = ephemeral JWS (future).
 */
export type StepUpMethod = 'pin' | 'passkey' | 'signed-token'

/**
 * A challenge issued by a provider.
 * Must be verified by the user to complete step-up.
 */
export interface Challenge {
  /** Unique ID for this challenge (nonce-like). */
  challenge_id: string
  /** The method requested (pin/passkey). */
  method: StepUpMethod
  /** Human-readable prompt (e.g. "Enter your 6-digit PIN to continue"). */
  prompt: string
  /** ISO timestamp when this challenge expires. */
  expires_at: string
  /**
   * Opaque provider-specific context (e.g. a WebAuthn public key challenge).
   * Not used for PIN.
   */
  context?: Record<string, any>
}

/** Result of a verification attempt. */
export interface VerifyResult {
  /** True iff verification succeeded. */
  success: boolean
  /** Human-readable reason on failure. */
  error?: string
  /** The method that was successfully verified. */
  method?: StepUpMethod
  /** ISO timestamp of successful verification. */
  verified_at?: string
}

/** Request to verify a previously issued challenge. */
export interface VerifyRequest {
  challenge_id: string
  /** The solution (e.g. the PIN string, or WebAuthn signature). */
  solution: string
  /**
   * SHA-256 hash of the original (Subject, Resource, Action, Context) 4-tuple.
   * Prevents challenge reuse for a different action (binding).
   */
  action_hash: string
}

/**
 * Common interface for all Step-up authentication providers.
 *
 * Implements BLUEPRINT--PHASE-5-STEP-UP-AUTH.
 */
export interface StepUpProvider {
  /** The method this provider handles. */
  method: StepUpMethod

  /**
   * Issue a new challenge for a given action.
   * @param action_hash Binding for the specific request.
   */
  challenge(action_hash: string): Promise<Challenge>

  /**
   * Verify a solution against a previously issued challenge.
   */
  verify(req: VerifyRequest): Promise<VerifyResult>
}
