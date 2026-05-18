import { exec } from 'node:child_process'
import { promisify } from 'node:util'

const execAsync = promisify(exec)

/**
 * Result of the git diff command.
 */
export interface GitDiffResult {
  diff: string
  files: string[]
}

/**
 * Extracts the `git diff` of currently staged files.
 */
export async function getStagedDiff(): Promise<GitDiffResult> {
  try {
    // 1. Get list of staged files
    const { stdout: filesRaw } = await execAsync('git diff --name-only --cached')
    const files = filesRaw.split('\n').filter(f => f.trim().length > 0)
    
    if (files.length === 0) {
      return { diff: '', files: [] }
    }

    // 2. Get the full diff of staged changes
    const { stdout: diff } = await execAsync('git diff --cached')
    
    return {
      diff,
      files,
    }
  } catch (err) {
    throw new Error(`Failed to extract git diff: ${(err as Error).message}`)
  }
}
