export interface VitestOpts {
  repoRoot: string
  verificationFiles: { src: string; dest: string }[]
  timeoutMs?: number
  vitestBin?: string
}

export type AcceptanceErrorKind = 'vitest-not-found' | 'spawn-failed' | 'sandbox' | 'timeout'

export class AcceptanceError extends Error {
  constructor(
    message: string,
    public readonly kind: AcceptanceErrorKind,
    public readonly cause?: unknown,
  ) {
    super(message)
    this.name = 'AcceptanceError'
  }
}
