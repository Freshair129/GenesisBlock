import { spawn } from 'node:child_process'
import { sep } from 'node:path'
import type { RunOpts, RunResult } from './types.js'

/**
 * On Windows, bare binary names like `gemini` need a shell to resolve the
 * `.cmd` / `.exe` / `.bat` extension via PATHEXT. But `shell: true` mangles
 * quoted arguments (see Node DEP0190).
 */
function needsShellOnWindows(bin: string): boolean {
  if (process.platform !== 'win32') return false
  // Already an absolute or relative path → spawn directly.
  if (bin.includes('/') || bin.includes(sep)) return false
  // Already has an extension → spawn directly (Node can resolve via PATH).
  if (/\.[a-z0-9]+$/i.test(bin)) return false
  return true
}

/**
 * Spawn an external CLI binary, capture stdout/stderr, and enforce a timeout.
 *
 * Supports optional stdin piping to bypass shell argument limits (UCF Phase 6).
 */
export async function runCli(
  bin: string,
  args: readonly string[],
  opts: RunOpts,
): Promise<RunResult> {
  return new Promise<RunResult>((resolve) => {
    const useShell = needsShellOnWindows(bin)
    const child = spawn(bin, args, {
      stdio: [opts.stdin ? 'pipe' : 'ignore', 'pipe', 'pipe'],
      shell: useShell,
      windowsHide: true,
    })

    if (opts.stdin && child.stdin) {
      child.stdin.write(opts.stdin)
      child.stdin.end()
    }

    let stdout = ''
    let stderr = ''
    let settled = false
    let timedOut = false

    const settle = (result: RunResult): void => {
      if (settled) return
      settled = true
      if (timer) clearTimeout(timer)
      resolve(result)
    }

    const timer = setTimeout(() => {
      timedOut = true
      try {
        child.kill('SIGKILL')
      } catch {
        // Best effort — child may have already exited.
      }
      settle({
        ok: false,
        output: 'timeout',
        stderr: opts.capture_stderr ? stderr : undefined,
        exit_code: -1,
      })
    }, opts.timeout_ms)

    child.stdout?.on('data', (chunk: Buffer | string) => {
      stdout += typeof chunk === 'string' ? chunk : chunk.toString('utf8')
    })

    child.stderr?.on('data', (chunk: Buffer | string) => {
      stderr += typeof chunk === 'string' ? chunk : chunk.toString('utf8')
    })

    child.on('error', () => {
      // ENOENT (binary not installed) or other spawn failure. Only fires when
      // `shell: false`; with `shell: true` on Windows, cmd.exe reports failure
      // via exit code 1 instead.
      settle({
        ok: false,
        output: '',
        stderr: opts.capture_stderr ? stderr : undefined,
        exit_code: -1,
      })
    })

    child.on('close', (code: number | null) => {
      if (timedOut) return // settle() already called in timer handler
      const exitCode = typeof code === 'number' ? code : -1
      settle({
        ok: exitCode === 0,
        output: stdout,
        stderr: opts.capture_stderr ? stderr : undefined,
        exit_code: exitCode,
      })
    })
  })
}
