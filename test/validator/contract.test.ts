import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import { loadContract } from '../../src/validator/contract.js'
import { FORBIDDEN_FIELDS as DEFAULT_FF } from '../../src/validator/rules/forbidden-fields.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-contract-'))
}

async function writeContract(root: string, content: string): Promise<void> {
  const dir = join(root, '.brain/msp/LLM_Contract')
  await mkdir(dir, { recursive: true })
  await writeFile(join(dir, 'atomic_contract.yaml'), content)
}

describe('loadContract', () => {
  it('returns defaults when YAML is missing', async () => {
    const root = await freshRoot()
    const c = await loadContract(root)
    expect(c.source).toBe('default')
    expect(c.forbiddenFields).toBe(DEFAULT_FF)
    expect(c.warnings.length).toBeGreaterThan(0)
    expect(c.warnings[0]).toMatch(/not found/)
  })

  it('loads forbidden_fields from a valid YAML', async () => {
    const root = await freshRoot()
    await writeContract(root, 'version: 1\nforbidden_fields:\n  - alpha\n  - beta\n  - gamma\n')
    const c = await loadContract(root)
    expect(c.source).toBe('yaml')
    expect(c.forbiddenFields.has('alpha')).toBe(true)
    expect(c.forbiddenFields.has('beta')).toBe(true)
    expect(c.forbiddenFields.has('gamma')).toBe(true)
    expect(c.forbiddenFields.has('commit_hash')).toBe(false)
    expect(c.warnings).toEqual([])
  })

  it('warns and falls back when YAML is invalid', async () => {
    const root = await freshRoot()
    await writeContract(root, 'version: : :\nthis is not valid yaml\n')
    const c = await loadContract(root)
    expect(c.source).toBe('default')
    expect(c.warnings.some((w) => w.includes('invalid YAML'))).toBe(true)
  })

  it('warns and falls back when forbidden_fields is wrong type', async () => {
    const root = await freshRoot()
    await writeContract(root, 'version: 1\nforbidden_fields: "not an array"\n')
    const c = await loadContract(root)
    expect(c.source).toBe('yaml') // top-level is OK
    expect(c.forbiddenFields).toBe(DEFAULT_FF) // but the field defaulted
    expect(c.warnings.some((w) => w.includes('forbidden_fields'))).toBe(true)
  })

  it('warns and falls back when version is missing or unsupported', async () => {
    const root = await freshRoot()
    await writeContract(root, 'forbidden_fields:\n  - foo\n')
    const c = await loadContract(root)
    expect(c.source).toBe('default')
    expect(c.warnings.some((w) => w.includes('version'))).toBe(true)
  })
})
