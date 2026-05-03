import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { runTask } from '../../src/codegen/runner.js'
import type { SlmClient, AcceptanceRunner } from '../../src/codegen/types.js'
import { CodegenError } from '../../src/codegen/types.js'

let originalCwd: string
let root: string

beforeEach(async () => {
  originalCwd = process.cwd()
  root = await mkdtemp(join(tmpdir(), 'msp-runner-'))
  await mkdir(join(root, 'gks/blueprint'), { recursive: true })
  await mkdir(join(root, 'tasks'), { recursive: true })
  // Stable blueprint.
  await writeFile(
    join(root, 'gks/blueprint/BLUEPRINT--TEST.md'),
    `---\nid: BLUEPRINT--TEST\nphase: 3\ntype: blueprint\nstatus: stable\nvault_id: TEST\n---\nbody`,
  )
  process.chdir(root)
})

afterEach(() => {
  process.chdir(originalCwd)
})

async function makeTask(yaml: string): Promise<string> {
  const path = join(root, 'tasks/T1.task.yaml')
  await writeFile(path, yaml)
  return path
}

const TASK_OK = `id: TEST
parent_blueprint: BLUEPRINT--TEST
prompt: |
  Make a thing
acceptance:
  - it works
geography:
  - src/foo.ts
`

describe('runTask', () => {
  it('succeeds when SLM emits clean code', async () => {
    const path = await makeTask(TASK_OK)
    const slm: SlmClient = async () => 'export const handler = async () => 42\n'
    const r = await runTask(path, { slmClient: slm })
    expect(r.exitCode).toBe(0)
    expect(r.finalStatus).toBe('success')
    expect(r.attempts).toHaveLength(1)
    expect(r.output).toContain('handler')
  })

  it('retries up to maxRetries when SLM emits forbidden pattern', async () => {
    const path = await makeTask(TASK_OK)
    let calls = 0
    const slm: SlmClient = async () => {
      calls++
      return 'export default function bad() {}\n' // export default is forbidden
    }
    const r = await runTask(path, { slmClient: slm, maxRetries: 3, escalate: false })
    expect(calls).toBe(3)
    expect(r.exitCode).toBe(1)
    expect(r.finalStatus).toBe('pattern-fail')
  })

  it('passes lastFailure context into the next prompt', async () => {
    const path = await makeTask(TASK_OK)
    const seenFailures: Array<unknown> = []
    const slm: SlmClient = async ({ lastFailure }) => {
      seenFailures.push(lastFailure)
      // First call emits bad code; second emits good code.
      return seenFailures.length === 1
        ? 'export default function bad() {}\n'
        : 'export const handler = async () => 1\n'
    }
    const r = await runTask(path, { slmClient: slm, maxRetries: 3 })
    expect(r.exitCode).toBe(0)
    expect(seenFailures[0]).toBeUndefined()
    expect(seenFailures[1]).toMatchObject({ kind: 'pattern' })
  })

  it('exits 1 when acceptance fails after all retries', async () => {
    const path = await makeTask(TASK_OK)
    const slm: SlmClient = async () => 'export const x = 1\n'
    const accept: AcceptanceRunner = async () => ['fixture failure']
    const r = await runTask(path, { slmClient: slm, acceptanceRunner: accept, maxRetries: 3, escalate: false })
    expect(r.exitCode).toBe(1)
    expect(r.finalStatus).toBe('acceptance-fail')
  })

  it('escalates to Gemini after retries when escalator is provided', async () => {
    const path = await makeTask(TASK_OK)
    const slm: SlmClient = async () => 'export default function bad() {}\n'
    const r = await runTask(path, {
      slmClient: slm,
      maxRetries: 2,
      escalate: true,
      escalator: async () => ({ ok: true, output: 'export const ok = 1\n' }),
    })
    expect(r.exitCode).toBe(3)
    expect(r.finalStatus).toBe('escalated-success')
    expect(r.escalation?.layer).toBe('gemini')
  })

  it('returns exit 4 when escalator also fails', async () => {
    const path = await makeTask(TASK_OK)
    const slm: SlmClient = async () => 'export default function bad() {}\n'
    const r = await runTask(path, {
      slmClient: slm,
      maxRetries: 2,
      escalate: true,
      escalator: async () => ({ ok: false }),
    })
    expect(r.exitCode).toBe(4)
    expect(r.finalStatus).toBe('escalated-fail')
  })

  it('throws CodegenError when blueprint is not stable', async () => {
    await writeFile(
      join(root, 'gks/blueprint/BLUEPRINT--TEST.md'),
      `---\nid: BLUEPRINT--TEST\nphase: 3\ntype: blueprint\nstatus: draft\nvault_id: TEST\n---\nbody`,
    )
    const path = await makeTask(TASK_OK)
    await expect(runTask(path)).rejects.toBeInstanceOf(CodegenError)
  })

  it('throws CodegenError when blueprint is missing', async () => {
    await writeFile(
      join(root, 'tasks/T1.task.yaml'),
      `id: T\nparent_blueprint: BLUEPRINT--MISSING\nprompt: p\nacceptance: [a]\ngeography: [s]\n`,
    )
    await expect(runTask(join(root, 'tasks/T1.task.yaml'))).rejects.toBeInstanceOf(CodegenError)
  })

  it('--dry-run returns success without calling SLM', async () => {
    const path = await makeTask(TASK_OK)
    let called = false
    const slm: SlmClient = async () => {
      called = true
      return 'export const x = 1\n'
    }
    const r = await runTask(path, { slmClient: slm, dryRun: true })
    expect(called).toBe(false)
    expect(r.exitCode).toBe(0)
    expect(r.attempts[0]!.cleanedOutput).toContain('TEST')
  })
})
