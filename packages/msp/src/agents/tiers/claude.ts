import { runCli } from './spawn-helper.js'
import type { RunOpts, RunResult, TierAdapter } from './types.js'

const BIN = 'claude'
const HEALTHCHECK_TIMEOUT_MS = 3000

/**
 * T3 — Claude Code CLI (`@anthropic-ai/claude-code`).
 *
 * Invocation pattern (verified 2026-05-14 against `claude --help`, v2.1.140):
 *   claude --print "<prompt>"
 *
 * - `-p / --print`: print response and exit (non-interactive mode); skips
 *   workspace-trust dialog when stdout is not a TTY.
 * - `-v / --version`: returns version string + exit 0
 *
 * Healthcheck: `claude --version` returns exit 0 if installed.
 */
export const claudeAdapter: TierAdapter = {
  name: 'T3',
  async healthcheck(): Promise<boolean> {
    const result = await runCli(BIN, ['--version'], {
      timeout_ms: HEALTHCHECK_TIMEOUT_MS,
      capture_stderr: false,
    })
    return result.exit_code === 0
  },
  async run(prompt: string, opts: RunOpts): Promise<RunResult> {
    return runCli(BIN, ['--print', prompt], opts)
  },
}
