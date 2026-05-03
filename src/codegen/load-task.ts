import { readFile } from 'node:fs/promises'

import { parse as parseYaml } from 'yaml'

import { CodegenError, type Task } from './types.js'

export async function loadTask(path: string): Promise<Task> {
  let raw: string
  try {
    raw = await readFile(path, 'utf8')
  } catch (err) {
    throw new CodegenError(`cannot read task ${path}`, err)
  }

  // Strip leading comment lines so YAML parser sees a pure object.
  const cleaned = raw
    .split('\n')
    .filter((l) => !l.trim().startsWith('#'))
    .join('\n')

  let parsed: unknown
  try {
    parsed = parseYaml(cleaned)
  } catch (err) {
    throw new CodegenError(`task ${path}: invalid YAML`, err)
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new CodegenError(`task ${path}: top-level must be a YAML object`)
  }

  const obj = parsed as Record<string, unknown>
  for (const f of ['id', 'parent_blueprint', 'prompt'] as const) {
    if (typeof obj[f] !== 'string' || !(obj[f] as string).trim()) {
      throw new CodegenError(`task ${path}: missing required field '${f}'`)
    }
  }
  if (!Array.isArray(obj.acceptance) || obj.acceptance.length < 1) {
    throw new CodegenError(`task ${path}: 'acceptance' must be a non-empty array`)
  }
  if (!Array.isArray(obj.geography) || obj.geography.length < 1) {
    throw new CodegenError(`task ${path}: 'geography' must be a non-empty array`)
  }

  return {
    id: obj.id as string,
    parent_blueprint: obj.parent_blueprint as string,
    status: typeof obj.status === 'string' ? obj.status : undefined,
    prompt: obj.prompt as string,
    acceptance: (obj.acceptance as unknown[]).filter((x): x is string => typeof x === 'string'),
    geography: (obj.geography as unknown[]).filter((x): x is string => typeof x === 'string'),
    assignee: typeof obj.assignee === 'string' ? obj.assignee : undefined,
    created_at: typeof obj.created_at === 'string' ? obj.created_at : undefined,
  }
}
