import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { projectsRegistryPath } from '../../src/lib/msp-home.js'
import {
  readRegistry,
  registerProject,
  writeRegistry,
} from '../../src/projects/registry.js'
import { defaultRegistry } from '../../src/projects/types.js'

async function fresh(): Promise<void> {
  const dir = await mkdtemp(join(tmpdir(), 'msp-projects-registry-'))
  process.env['MSP_HOME'] = resolve(dir, '.msp')
}

let savedHome: string | undefined
beforeEach(() => {
  savedHome = process.env['MSP_HOME']
})
afterEach(() => {
  if (savedHome === undefined) delete process.env['MSP_HOME']
  else process.env['MSP_HOME'] = savedHome
})

describe('readRegistry', () => {
  it('returns empty default registry when file is missing', async () => {
    await fresh()
    const registry = await readRegistry()
    expect(registry.schemaVersion).toBe(1)
    expect(registry.projects).toEqual({})
    expect(registry.default).toBeUndefined()
    // Read does NOT create the file
    expect(existsSync(projectsRegistryPath())).toBe(false)
  })

  it('throws on invalid YAML', async () => {
    await fresh()
    const path = projectsRegistryPath()
    await mkdir(dirname(path), { recursive: true })
    await writeFile(path, 'not: valid: yaml: : :{{{', 'utf8')
    await expect(readRegistry()).rejects.toThrow(/not valid YAML/)
  })

  it('throws when schemaVersion is missing', async () => {
    await fresh()
    const path = projectsRegistryPath()
    await mkdir(dirname(path), { recursive: true })
    await writeFile(path, 'projects: {}\n', 'utf8')
    await expect(readRegistry()).rejects.toThrow(/schemaVersion/)
  })

  it('throws when schemaVersion > 1', async () => {
    await fresh()
    const path = projectsRegistryPath()
    await mkdir(dirname(path), { recursive: true })
    await writeFile(path, 'schemaVersion: 99\nprojects: {}\n', 'utf8')
    await expect(readRegistry()).rejects.toThrow(/schemaVersion=99/)
  })

  it('parses a full registry with default + multiple projects', async () => {
    await fresh()
    const path = projectsRegistryPath()
    await mkdir(dirname(path), { recursive: true })
    await writeFile(
      path,
      [
        'schemaVersion: 1',
        'default: eva',
        'projects:',
        '  default:',
        '    path: /tmp/default',
        '  eva:',
        '    path: /home/user/eva',
        '    embedder: nomic-embed-text-v1.5',
        '    description: research project',
        '',
      ].join('\n'),
      'utf8',
    )
    const registry = await readRegistry()
    expect(registry.schemaVersion).toBe(1)
    expect(registry.default).toBe('eva')
    expect(registry.projects).toEqual({
      default: { path: '/tmp/default' },
      eva: {
        path: '/home/user/eva',
        embedder: 'nomic-embed-text-v1.5',
        description: 'research project',
      },
    })
  })

  it('rejects a project entry missing the required `path` field', async () => {
    await fresh()
    const path = projectsRegistryPath()
    await mkdir(dirname(path), { recursive: true })
    await writeFile(
      path,
      [
        'schemaVersion: 1',
        'projects:',
        '  bad:',
        '    embedder: x',
        '',
      ].join('\n'),
      'utf8',
    )
    await expect(readRegistry()).rejects.toThrow(/missing required `path`/)
  })
})

describe('writeRegistry', () => {
  it('round-trips through readRegistry', async () => {
    await fresh()
    const registry = defaultRegistry()
    registry.projects.eva = { path: '/home/user/eva', embedder: 'x' }
    registry.default = 'eva'
    await writeRegistry(registry)
    const read = await readRegistry()
    expect(read).toEqual(registry)
  })

  it('forces schemaVersion to 1 on write', async () => {
    await fresh()
    const bogus = {
      schemaVersion: 99 as unknown as 1,
      projects: { x: { path: '/x' } },
    }
    await writeRegistry(bogus)
    const text = await readFile(projectsRegistryPath(), 'utf8')
    expect(text).toContain('schemaVersion: 1')
  })
})

describe('registerProject', () => {
  it('adds a new project entry', async () => {
    await fresh()
    await registerProject('eva', {
      path: '/home/user/eva',
      embedder: 'nomic-embed-text-v1.5',
    })
    const registry = await readRegistry()
    expect(registry.projects.eva).toEqual({
      path: '/home/user/eva',
      embedder: 'nomic-embed-text-v1.5',
    })
  })

  it('errors on duplicate project name (no silent overwrite)', async () => {
    await fresh()
    await registerProject('eva', { path: '/a' })
    await expect(registerProject('eva', { path: '/b' })).rejects.toThrow(
      /already registered/,
    )
  })

  it('errors on empty name', async () => {
    await fresh()
    await expect(
      registerProject('', { path: '/a' }),
    ).rejects.toThrow(/non-empty/)
  })

  it('errors on empty path', async () => {
    await fresh()
    await expect(registerProject('x', { path: '' })).rejects.toThrow(/path/)
  })
})
