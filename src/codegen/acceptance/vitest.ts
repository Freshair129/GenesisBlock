import { spawn } from 'node:child_process'
import { resolve } from 'node:path'

import type { AcceptanceRunner } from '../types.js'
import {
  cleanupSandbox,
  copyVerification,
  createSandbox,
  scaffoldSandbox,
  writeCandidate,
} from './sandbox.js'
import { parseVitestJson } from './parse-results.js'
import { AcceptanceError, type VitestOpts } from './types.js'

const DEFAULT_TIMEOUT_MS = 60_000

interface SpawnResult {
  stdout: string
  stderr: string
  code: number
  timedOut: boolean
}

function spawnVitest(cwd: string, vitestBin: string, timeoutMs: number): Promise<SpawnResult> {
  return new Promise((resolveProm, rejectProm) => {
    const [cmd, ...rest] = vitestBin.split(/\s+/)
    const args = [...rest, 'run', '--reporter=json']
    const child = spawn(cmd!, args, {
      cwd,
      env: { ...process.env, NO_COLOR: '1' },
    })

    let stdout = ''
    let stderr = ''
    let timedOut = false
    const timer = setTimeout(() => {
      timedOut = true
      child.kill('SIGTERM')
      setTimeout(() => child.kill('SIGKILL'), 5_000).unref()
    }, timeoutMs)

    child.stdout.on('data', (d) => (stdout += d.toString()))
    child.stderr.on('data', (d) => (stderr += d.toString()))
    child.on('error', (err) => {
      clearTimeout(timer)
      const code = (err as NodeJS.ErrnoException).code
      if (code === 'ENOENT') {
        rejectProm(new AcceptanceError(`vitest binary not found via '${vitestBin}'`, 'vitest-not-found', err))
      } else {
        rejectProm(new AcceptanceError(`spawn vitest: ${err.message}`, 'spawn-failed', err))
      }
    })
    child.on('close', (code) => {
      clearTimeout(timer)
      resolveProm({ stdout, stderr, code: code ?? -1, timedOut })
    })
  })
}

export function createVitestAcceptance(opts: VitestOpts): AcceptanceRunner {
  const repoRoot = resolve(opts.repoRoot)
  const verificationFiles = opts.verificationFiles
  const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS
  const vitestBin = opts.vitestBin ?? 'npx vitest'

  return async (task, code) => {
    const sandbox = await createSandbox()
    try {
      await scaffoldSandbox(sandbox, repoRoot)
      await writeCandidate(sandbox, task.geography, code)
      await copyVerification(sandbox, repoRoot, verificationFiles)

      const result = await spawnVitest(sandbox, vitestBin, timeoutMs)
      if (result.timedOut) {
        return [`acceptance: timeout after ${timeoutMs}ms`]
      }
      return parseVitestJson(result.stdout, result.stderr, result.code)
    } finally {
      await cleanupSandbox(sandbox)
    }
  }
}

export { AcceptanceError, type VitestOpts } from './types.js'
