import { mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import { loadTask } from '../../src/codegen/load-task.js'
import { CodegenError } from '../../src/codegen/types.js'

async function tmpFile(content: string): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), 'msp-task-'))
  const path = join(dir, 'T1_test.task.yaml')
  await writeFile(path, content)
  return path
}

describe('loadTask', () => {
  it('parses a complete task', async () => {
    const path = await tmpFile(`# comment
id: TEST-TASK
parent_blueprint: BLUEPRINT--FOO
status: open
prompt: |
  do the thing
acceptance:
  - it works
  - it doesn't break anything else
geography:
  - src/foo.ts
`)
    const t = await loadTask(path)
    expect(t.id).toBe('TEST-TASK')
    expect(t.parent_blueprint).toBe('BLUEPRINT--FOO')
    expect(t.acceptance).toHaveLength(2)
    expect(t.geography).toEqual(['src/foo.ts'])
  })

  it('throws CodegenError on missing required field', async () => {
    const path = await tmpFile(`prompt: x\nacceptance: [a, b]\ngeography: [s]\n`)
    await expect(loadTask(path)).rejects.toBeInstanceOf(CodegenError)
  })

  it('throws CodegenError on empty acceptance', async () => {
    const path = await tmpFile(`id: T\nparent_blueprint: B\nprompt: p\nacceptance: []\ngeography: [s]\n`)
    await expect(loadTask(path)).rejects.toBeInstanceOf(CodegenError)
  })

  it('throws CodegenError on missing file', async () => {
    await expect(loadTask('/nonexistent/path.yaml')).rejects.toBeInstanceOf(CodegenError)
  })
})
