import { runGeminiCli } from '../slm/gemini.js';
import { SlmError } from '../slm/errors.js';
/**
 * Gemini CLI Escalator — delegates to the `gemini` CLI subagent.
 *
 * Per ADR--CODEGEN-MICROTASK-RUNNER:
 * After 3 retries, invoke Gemini CLI via `gemini -p "<escalation_prompt>" -y`.
 *
 * Shares the subprocess wrapper with `slm/gemini.ts` so both call paths
 * stay in sync (env vars, timeouts, error mapping).
 */
export function createGeminiEscalator() {
    return async (task, blueprint, history) => {
        // Construct a comprehensive prompt for Gemini
        const lastAttempt = history[history.length - 1];
        const failureSummary = history
            .map((h) => `Attempt ${h.attempt}: ${h.patternErrors.join(', ')} ${h.acceptanceErrors.join(', ')}`)
            .join('\n');
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
`.trim();
        try {
            const { stdout } = await runGeminiCli(escalationPrompt);
            // The output from gemini CLI might contain some chatter or headers.
            // We rely on the runner's post-processing to clean it up.
            return { ok: true, output: stdout };
        }
        catch (err) {
            const detail = err instanceof SlmError ? `${err.kind}: ${err.message}` : err.message;
            console.error('[escalator] Gemini failed:', detail);
            return { ok: false };
        }
    };
}
