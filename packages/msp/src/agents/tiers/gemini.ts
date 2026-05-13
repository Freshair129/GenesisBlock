import { runCli } from './spawn-helper.js'
import type { RunOpts, RunResult, TierAdapter } from './types.js'

const BIN = 'gemini'
const HEALTHCHECK_TIMEOUT_MS = 3000

/**
 * T2 — Gemini CLI (`@google/gemini-cli`, installed via npm globally).
 *
 * Invocation pattern (verified 2026-05-14 against `gemini --help`, v0.42.0):
 *   gemini --approval-mode yolo -p "<prompt>"
 *
 * - `-p / --prompt`: non-interactive (headless) mode
 * - `--approval-mode yolo`: auto-approve all tool calls (no prompts)
 * - `--version` (`-v`): supported, returns version string + exit 0
 *
 * Healthcheck: `gemini --version` returns exit 0 if installed.
 */
export const geminiAdapter: TierAdapter = {
  name: 'T2',
  async healthcheck(): Promise<boolean> {
    const result = await runCli(BIN, ['--version'], {
      timeout_ms: HEALTHCHECK_TIMEOUT_MS,
      capture_stderr: false,
    })
    return result.exit_code === 0
  },
  async run(prompt: string, opts: RunOpts): Promise<RunResult> {
    return runCli(BIN, ['--approval-mode', 'yolo', '-p', prompt], opts)
  },
}
