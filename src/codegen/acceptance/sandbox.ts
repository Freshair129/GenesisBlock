import { copyFile, cp, mkdir, rm, symlink, writeFile } from 'node:fs/promises'
import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { AcceptanceError } from './types.js'

const SANDBOX_PACKAGE_JSON = JSON.stringify(
  {
    name: 'msp-acceptance-sandbox',
    private: true,
    type: 'module',
  },
  null,
  2,
) + '\n'

const SANDBOX_VITEST_CONFIG = `import { defineConfig } from 'vitest/config'
export default defineConfig({
  test: {
    include: ['test/**/*.test.ts'],
    environment: 'node',
    reporters: ['json'],
  },
})
`

export async function createSandbox(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-accept-'))
}

export async function cleanupSandbox(path: string): Promise<void> {
  await rm(path, { recursive: true, force: true })
}

/**
 * Scaffold the sandbox: package.json, vitest.config.ts, node_modules link.
 * Symlink first; on failure (e.g. cross-fs / Windows), fall back to copy.
 */
export async function scaffoldSandbox(sandbox: string, repoRoot: string): Promise<void> {
  await writeFile(join(sandbox, 'package.json'), SANDBOX_PACKAGE_JSON)
  await writeFile(join(sandbox, 'vitest.config.ts'), SANDBOX_VITEST_CONFIG)

  const sourceModules = resolve(repoRoot, 'node_modules')
  const targetModules = join(sandbox, 'node_modules')
  try {
    await symlink(sourceModules, targetModules, 'dir')
  } catch (err) {
    const code = (err as NodeJS.ErrnoException).code
    if (code === 'EEXIST') return
    if (code === 'EPERM' || code === 'EXDEV') {
      // Cross-filesystem or restricted symlinks → fall back to copy.
      console.warn(`[sandbox] symlink failed (${code}); copying node_modules — slower`)
      await cp(sourceModules, targetModules, { recursive: true, dereference: false })
      return
    }
    throw new AcceptanceError(`scaffoldSandbox: ${(err as Error).message}`, 'sandbox', err)
  }
}

export async function writeCandidate(
  sandbox: string,
  geography: string[],
  code: string,
): Promise<void> {
  // Single-file candidate writes the same code to every geography path.
  // Multi-file split is a future enhancement.
  for (const rel of geography) {
    const dest = join(sandbox, rel)
    await mkdir(dirname(dest), { recursive: true })
    await writeFile(dest, code, 'utf8')
  }
}

export async function copyVerification(
  sandbox: string,
  repoRoot: string,
  files: { src: string; dest: string }[],
): Promise<void> {
  for (const f of files) {
    const dest = join(sandbox, f.dest)
    await mkdir(dirname(dest), { recursive: true })
    await copyFile(resolve(repoRoot, f.src), dest)
  }
}
