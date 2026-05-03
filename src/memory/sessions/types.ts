export interface SessionTurn {
  sessionId: string
  episodicId: string
  turnId: number
  msgId: string
  speakerId: string
  content: string
  learnId?: string
}

export interface OpenOpts {
  episodicId: string
  root: string
  namespace?: string
}

export interface Session {
  appendTurn(row: SessionTurn): Promise<void>
  close(): Promise<void>
}

export class SessionLockedError extends Error {
  constructor(public readonly holderPid: number, public readonly path: string) {
    super(`session lock held by pid ${holderPid} at ${path}`)
    this.name = 'SessionLockedError'
  }
}

export class SessionSchemaError extends Error {
  constructor(public readonly missingFields: string[]) {
    super(`session row missing required fields: ${missingFields.join(', ')}`)
    this.name = 'SessionSchemaError'
  }
}
