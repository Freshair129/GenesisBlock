export type SlmErrorKind = 'network' | 'http' | 'parse' | 'timeout' | 'config' | 'runtime'

export class SlmError extends Error {
  constructor(
    message: string,
    public readonly kind: SlmErrorKind,
    public readonly cause?: unknown,
  ) {
    super(message)
    this.name = 'SlmError'
  }
}
