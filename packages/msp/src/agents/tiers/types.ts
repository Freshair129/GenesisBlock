export interface TierAdapter {
  readonly name: 'T1' | 'T2' | 'T3'
  healthcheck(): Promise<boolean>
  run(prompt: string, opts: RunOpts): Promise<RunResult>
}

export interface RunOpts {
  timeout_ms: number
  capture_stderr: boolean
  /** UCF Phase 6: Optional stdin to pipe to the process. */
  stdin?: string
}

export interface RunResult {
  ok: boolean
  output: string
  stderr?: string
  exit_code: number
}
