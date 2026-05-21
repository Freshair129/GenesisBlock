/**
 * Gemini CLI SLM client — first-class T2/T3 codegen subagent.
 *
 * Invokes the `gemini` CLI binary with `gemini -p <prompt> -y` (the `-y` flag
 * auto-accepts confirmations so the call is non-interactive). The same
 * helper backs the final-escalator path in `escalator/gemini.ts` — one
 * subprocess wrapper, two callers.
 *
 * Selection: set `MSP_SLM_PROVIDER=gemini` to make Gemini the primary SLM
 * for codegen microtasks. Otherwise it stays as the escalation tier after
 * Ollama exhausts retries (see `codegen/runner.ts`).
 */
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { SlmError } from './errors.js';
const execFileAsync = promisify(execFile);
const DEFAULT_BIN = 'gemini';
const DEFAULT_TIMEOUT_MS = 600_000;
const DEFAULT_MAX_BUFFER = 20 * 1024 * 1024;
export function createGeminiClient(opts = {}) {
    return async (call) => {
        const { stdout } = await runGeminiCli(call.prompt, opts);
        return stdout;
    };
}
/**
 * Shared subprocess wrapper. Used both by the SLM client (this file) and
 * the final escalator (`escalator/gemini.ts`).
 */
export async function runGeminiCli(prompt, opts = {}) {
    const bin = opts.binPath ?? process.env.GEMINI_BIN ?? DEFAULT_BIN;
    const extraArgs = opts.extraArgs ?? ['-y'];
    const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    const maxBuffer = opts.maxBuffer ?? DEFAULT_MAX_BUFFER;
    const model = opts.model ?? process.env.GEMINI_MODEL;
    const args = ['-p', prompt, ...extraArgs];
    if (model)
        args.push('-m', model);
    try {
        const { stdout, stderr } = await execFileAsync(bin, args, {
            maxBuffer,
            timeout: timeoutMs,
            shell: process.platform === 'win32',
        });
        return { stdout: stdout ?? '', stderr: stderr ?? '' };
    }
    catch (err) {
        const e = err;
        if (e.killed && e.signal === 'SIGTERM') {
            throw new SlmError(`gemini timeout after ${timeoutMs}ms`, 'timeout', err);
        }
        if (e.code === 'ENOENT') {
            throw new SlmError(`gemini CLI not found at '${bin}' — install gemini-cli or set GEMINI_BIN`, 'config', err);
        }
        const detail = e.stderr ? `: ${e.stderr.slice(0, 200)}` : '';
        throw new SlmError(`gemini cli failed${detail}`, 'runtime', err);
    }
}
