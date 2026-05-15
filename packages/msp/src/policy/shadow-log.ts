import { appendFile, mkdir } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import {
  type Action,
  type Decision,
  type RequestContext,
  type Resource,
  type Subject,
} from './types.js'

export interface ShadowLogEntry {
  t: string
  trace_id: string
  subject: Subject
  resource: Resource
  action: Action
  context: RequestContext
  decision: Decision
  policy_version: number
}

/**
 * Append an entry to the shadow policy log.
 */
export async function logShadowDecision(
  entry: Omit<ShadowLogEntry, 't'>,
  logPath: string,
): Promise<void> {
  const fullEntry: ShadowLogEntry = {
    t: new Date().toISOString(),
    ...entry,
  }

  try {
    await mkdir(dirname(logPath), { recursive: true })
    await appendFile(logPath, JSON.stringify(fullEntry) + '\n', 'utf8')
  } catch (err) {
    console.error(`[policy] failed to write shadow log: ${(err as Error).message}`)
  }
}
