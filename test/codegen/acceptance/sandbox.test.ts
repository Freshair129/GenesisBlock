import { lstat, mkdtemp, readFile, readdir } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

import {
  cleanupSandbox,
  copyVerification,
  createSandbox,
  scaffoldSandbox,
  writeCandidate,
} from '../../../src/codegen/acceptance/sandbox.js'

const repoRoot = fileURLToPath(new URL('../../..', import.meta.url))

describe('sandbox', () => {
  it('createSandbox + scaffoldSandbox places package.json + vitest.config.ts + node_modules', async () => {
    const dir = await createSandbox()
    try {
      await scaffoldSandbox(dir, repoRoot)
      const entries = await readdir(dir)
      expect(entries).toContain('package.json')
      expect(entries).toContain('vitest.config.ts')
      expect(entries).toContain('node_modules')
      const stat = await lstat(join(dir, 'node_modules'))
      // Either a symlink (preferred) or a directory (fallback). Both fine.
      expect(stat.isSymbolicLink() || stat.isDirectory()).toBe(true)
    } finally {
      await cleanupSandbox(dir)
    }
  }, 30_000)

  it('writeCandidate writes the same code to every geography path', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'sb-test-'))
    try {
      await writeCandidate(dir, ['src/foo.ts', 'src/sub/bar.ts'], 'export const x = 1\n')
      const a = await readFile(join(dir, 'src/foo.ts'), 'utf8')
      const b = await readFile(join(dir, 'src/sub/bar.ts'), 'utf8')
      expect(a).toBe('export const x = 1\n')
      expect(b).toBe('export const x = 1\n')
    } finally {
      await cleanupSandbox(dir)
    }
  })

  it('copyVerification copies files at the requested dest path', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'sb-test-'))
    try {
      await copyVerification(dir, repoRoot, [
        { src: 'package.json', dest: 'copy/of/package.json' },
      ])
      const text = await readFile(join(dir, 'copy/of/package.json'), 'utf8')
      expect(text).toContain('"name"')
    } finally {
      await cleanupSandbox(dir)
    }
  })

  it('cleanupSandbox removes the dir', async () => {
    const dir = await createSandbox()
    await cleanupSandbox(dir)
    await expect(readdir(dir)).rejects.toThrow()
  })
})
