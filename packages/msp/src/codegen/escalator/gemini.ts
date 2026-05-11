import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import type { Blueprint, Escalator, Task } from '../types.js'

const execFileAsync = promisify(execFile)

/**
 * Gemini CLI Escalator — delegates to the `gemini` CLI subagent.
 * 
 * Per ADR--CODEGEN-MICROTASK-RUNNER:
 * After 3 retries, invoke Gemini CLI via `gemini -p "<escalation_prompt>" -y`.
 */
export function createGeminiEscalator(): Escalator {
  return async (task: Task, blueprint: Blueprint, history: any[]): Promise<{ ok: boolean; output?: string }> => {
    // Construct a comprehensive prompt for Gemini
    const lastAttempt = history[history.length - 1]
    const failureSummary = history
      .map((h) => `Attempt ${h.attempt}: ${h.patternErrors.join(', ')} ${h.acceptanceErrors.join(', ')}`)
      .join('\n')

    const escalationPrompt = `
You are an expert software engineer. A smaller model (SLM) failed to complete a coding task after 3 attempts.
Your goal is to fix the code to pass all acceptance criteria.

# TASK
${task.prompt}

# ACCEPTANCE CRITERIA
${task.acceptance.join('\n')}

# GEOGRAPHY
Targets: ${task.geography.join(', ')}

# BLUEPRINT CONTEXT
${blueprint.body}

# FAILURE HISTORY
${failureSummary}

# LAST ATTEMPT OUTPUT
\`\`\`ts
${lastAttempt?.cleanedOutput ?? ''}
\`\`\`

Please provide the corrected code for the target files. 
Return ONLY the raw code for the files, no explanations, no markdown blocks unless they are part of the file content.
If there are multiple files, separate them with a clear marker or follow the standard GKS atom format if applicable.
(Note: The runner expects a single string which it will post-process).
`.trim()

    try {
      // gemini -p "prompt" -y
      // -y automatically accepts the command
      const { stdout } = await execFileAsync('gemini', ['-p', escalationPrompt, '-y'], {
        maxBuffer: 20 * 1024 * 1024, // 20MB
      })
      
      // The output from gemini CLI might contain some chatter or headers.
      // We rely on the runner's post-processing to clean it up.
      return { ok: true, output: stdout }
    } catch (err) {
      console.error('[escalator] Gemini failed:', (err as Error).message)
      return { ok: false }
    }
  }
}
