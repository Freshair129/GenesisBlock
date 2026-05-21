import { runCli } from './spawn-helper.js';
const BIN = 'qwen';
const HEALTHCHECK_TIMEOUT_MS = 3000;
/**
 * T1 — Qwen (`packages/qwen-cli/qwen.py`, also installed as `qwen.exe` shim).
 *
 * Invocation pattern (verified 2026-05-14 against `qwen --help`):
 *   qwen [options] <prompt-positional>
 *
 * The Python Ollama-wrapper CLI takes the prompt as POSITIONAL args (joined
 * with spaces), NOT a `--prompt` flag. It does not support `--version` —
 * `--help` is the healthcheck signal instead.
 *
 * Healthcheck: `qwen --help` returns exit 0 if the binary is on PATH.
 */
export const qwenAdapter = {
    name: 'T1',
    async healthcheck() {
        const result = await runCli(BIN, ['--help'], {
            timeout_ms: HEALTHCHECK_TIMEOUT_MS,
            capture_stderr: false,
        });
        return result.exit_code === 0;
    },
    async run(prompt, opts) {
        // Prompt is passed as a single positional argument. `runCli` forwards it
        // through Node's spawn; on Windows the spawn-helper uses `shell: true`
        // for bare binary names — prompts are treated as trusted-internal input
        // (sourced from the orchestrator, never raw user input).
        return runCli(BIN, [prompt], opts);
    },
};
