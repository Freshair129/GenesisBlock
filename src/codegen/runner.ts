import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

import { loadTask } from './load-task.js'
import { checkForbiddenPatterns, checkImports } from './forbidden-patterns.js'
import { postProcess } from './post-process.js'
import { buildPrompt, promptHash } from './prompt-builder.js'
import {
  CodegenError,
  type AcceptanceRunner,
  type AttemptRecord,
  type Blueprint,
  type RunOptions,
  type RunResult,
  type SlmClient,
} from './types.js'

const DEFAULT_MAX_RETRIES = 3
const DEFAULT_MODEL = 'mock-slm'

/** Mock SLM that echoes a code skeleton. Used when no real SLM is wired. */
const defaultMock: SlmClient = async ({ prompt }) => {
  // The mock returns whatever pattern the prompt explicitly hints at via
  // a `// MOCK_OUTPUT:` comment in the task prompt. Otherwise a stub.
  const m = prompt.match(/MOCK_OUTPUT:\s*([\s\S]+?)(?:\n##|$)/)
  if (m) return m[1]!.trim() + '\n'
  return 'export const handler = async () => ({ ok: true })\n'
}

const defaultAcceptance: AcceptanceRunner = async (_task, _code) => {
  // Default — assume pass. Real runners should plug in a vitest invocation.
  return []
}

async function loadBlueprint(root: string, blueprintId: string): Promise<Blueprint> {
  const path = resolve(root, 'gks/blueprint', `${blueprintId}.md`)
  let raw: string
  try {
    raw = await readFile(path, 'utf8')
  } catch (err) {
    throw new CodegenError(`blueprint ${blueprintId} not found at ${path}`, err)
  }
  // Extract status from frontmatter (simple regex; full YAML is overkill here).
  const statusMatch = raw.match(/^status:\s*([a-z]+)\s*$/m)
  const status = statusMatch?.[1] ?? 'unknown'
  return { id: blueprintId, status, body: raw }
}

async function loadPackageDeps(root: string): Promise<Set<string>> {
  try {
    const raw = await readFile(resolve(root, 'package.json'), 'utf8')
    const pkg = JSON.parse(raw)
    const deps = new Set<string>()
    for (const k of ['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies']) {
      const o = pkg[k]
      if (o && typeof o === 'object') for (const name of Object.keys(o)) deps.add(name)
    }
    return deps
  } catch {
    return new Set()
  }
}

export async function runTask(
  taskPath: string,
  opts: RunOptions = {},
): Promise<RunResult> {
  const root = resolve(process.cwd())
  const task = await loadTask(taskPath)
  const blueprint = await loadBlueprint(root, task.parent_blueprint)
  if (blueprint.status !== 'stable') {
    throw new CodegenError(
      `parent blueprint ${task.parent_blueprint} is '${blueprint.status}' — must be 'stable' before running tasks`,
    )
  }

  const slmClient: SlmClient = opts.slmClient ?? defaultMock
  const accept = opts.acceptanceRunner ?? defaultAcceptance
  const model = opts.model ?? DEFAULT_MODEL
  const maxRetries = opts.maxRetries ?? DEFAULT_MAX_RETRIES
  const packageDeps = await loadPackageDeps(root)

  const attempts: AttemptRecord[] = []
  let lastFailure: { kind: 'pattern' | 'test'; details: string[] } | undefined
  let lastSuccess: string | undefined

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    const prompt = buildPrompt({ task, blueprint, attempt, lastFailure })
    if (opts.dryRun) {
      attempts.push({
        attempt,
        promptHash: promptHash(prompt),
        slmModel: model,
        rawOutput: '',
        cleanedOutput: prompt, // surface the prompt for inspection
        patternErrors: [],
        acceptanceErrors: [],
      })
      return {
        taskId: task.id,
        attempts,
        finalStatus: 'success',
        exitCode: 0,
      }
    }

    const raw = await slmClient({ prompt, model, attempt, lastFailure })
    const cleaned = postProcess(raw)

    const patterns = checkForbiddenPatterns(cleaned)
    const importErrors = checkImports(cleaned, packageDeps)
    const allPatternErrors = [...patterns.errors, ...importErrors]

    if (allPatternErrors.length > 0) {
      attempts.push({
        attempt,
        promptHash: promptHash(prompt),
        slmModel: model,
        rawOutput: raw,
        cleanedOutput: cleaned,
        patternErrors: allPatternErrors,
        acceptanceErrors: [],
      })
      lastFailure = { kind: 'pattern', details: allPatternErrors }
      continue
    }

    const testErrors = await accept(task, cleaned)
    if (testErrors.length > 0) {
      attempts.push({
        attempt,
        promptHash: promptHash(prompt),
        slmModel: model,
        rawOutput: raw,
        cleanedOutput: cleaned,
        patternErrors: [],
        acceptanceErrors: testErrors,
      })
      lastFailure = { kind: 'test', details: testErrors }
      continue
    }

    attempts.push({
      attempt,
      promptHash: promptHash(prompt),
      slmModel: model,
      rawOutput: raw,
      cleanedOutput: cleaned,
      patternErrors: [],
      acceptanceErrors: [],
    })
    lastSuccess = cleaned
    return { taskId: task.id, attempts, finalStatus: 'success', exitCode: 0, output: cleaned }
  }

  // All retries exhausted. Optionally escalate.
  if (opts.escalate !== false && opts.escalator) {
    const r = await opts.escalator(task, blueprint, attempts)
    if (r.ok) {
      return {
        taskId: task.id,
        attempts,
        finalStatus: 'escalated-success',
        exitCode: 3,
        escalation: { layer: 'gemini', outcome: 'pass' },
        output: r.output,
      }
    }
    return {
      taskId: task.id,
      attempts,
      finalStatus: 'escalated-fail',
      exitCode: 4,
      escalation: { layer: 'opus', outcome: 'fail' },
    }
  }

  const finalStatus = lastFailure?.kind === 'pattern' ? 'pattern-fail' : 'acceptance-fail'
  return { taskId: task.id, attempts, finalStatus, exitCode: 1, output: lastSuccess }
}
