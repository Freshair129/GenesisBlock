import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import type { SlmCall, SlmClient } from '../types.js'
import { SlmError } from './errors.js'
import type { QwenOpts } from './types.js'

const execFileAsync = promisify(execFile)

/**
 * Qwen CLI client — delegates to a local python script.
 * 
 * Per ADR--CODEGEN-MICROTASK-RUNNER:
 * Execute `python G:\qwen-cli\qwen.py --code --no-stream` with the assembled prompt.
 */
export function createQwenClient(opts: QwenOpts = {}): SlmClient {
  const qwenPath = opts.path ?? process.env.MSP_QWEN_PATH ?? 'G:\\qwen-cli\\qwen.py'
  const model = opts.model ?? 'qwen2.5-coder:7b-instruct' // or whatever is default in qwen.py

  return async ({ prompt, lastFailure }: SlmCall): Promise<string> => {
    // If we have a lastFailure, we might want to append it to the prompt or handle it.
    // The prompt builder already handles this, so we just pass the prompt.

    const args = [qwenPath, '--code', '--no-stream']
    if (model) {
      args.push('--model', model)
    }
    if (opts.temperature !== undefined) {
      args.push('--temp', opts.temperature.toString())
    }
    args.push(prompt)

    try {
      const { stdout } = await execFileAsync('python', args, {
        maxBuffer: 10 * 1024 * 1024, // 10MB
      })
      return stdout
    } catch (err) {
      throw new SlmError(
        `Qwen CLI execution failed: ${(err as Error).message}`,
        'runtime',
      )
    }
  }
}
