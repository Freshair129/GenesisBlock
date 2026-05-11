import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { projectsRegistryPath } from '../../src/lib/msp-home.js'
import { writeRegistry } from '../../src/projects/registry.js'
import { parseMspconfig, resolveProject } from '../../src/projects/resolve.js'
import { defaultRegistry } from '../../src/projects/types.js'

async function fresh(): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), 'msp-projects-resolve-'))
  process.env['MSP_HOME'] = resolve(dir, '.msp')
  return dir
}

let savedHome: string | undefined
beforeEach(() => {
  savedHome = process.env['MSP_HOME']
})
afterEach(() => {
  if (savedHome === undefined) delete process.env['MSP_HOME']
  else process.env['MSP_HOME'] = savedHome
})

async function seedRegistry(
  entries: Record<string, { path: string }>,
  defaultName?: string,
): Promise<void> {
  const reg = defaultRegistry()
  reg.projects = entries
  if (defaultName) reg.default = defaultName
  await mkdir(dirname(projectsRegistryPath()), { recursive: true })
  await writeRegistry(reg)
}

describe('parseMspconfig', () => {
  it('parses single-line shorthand `project: <name>`', async () => {
    const dir = await fresh()
    const cfg = join(dir, '.mspconfig')
    await writeFile(cfg, 'project: eva\n', 'utf8')
    expect(await parseMspconfig(cfg)).toBe('eva')
  })

  it('parses full YAML form', async () => {
    const dir = await fresh()
    const cfg = join(dir, '.mspconfig')
    await writeFile(cfg, 'project: clinic\nnotes: foo\n', 'utf8')
    expect(await parseMspconfig(cfg)).toBe('clinic')
  })

  it('throws on missing `project` key', async () => {
    const dir = await fresh()
    const cfg = join(dir, '.mspconfig')
    await writeFile(cfg, 'something: else\n', 'utf8')
    await expect(parseMspconfig(cfg)).rejects.toThrow(/project/)
  })
})

describe('resolveProject — resolution priority', () => {
  it('CLI flag wins over env, mspconfig, default', async () => {
    const dir = await fresh()
    await seedRegistry({ alpha: { path: '/a' }, beta: { path: '/b' }, gamma: { path: '/g' } }, 'gamma')
    const cfg = join(dir, '.mspconfig')
    await writeFile(cfg, 'project: beta\n', 'utf8')
    const r = await resolveProject({
      cliFlag: 'alpha',
      env: 'beta',
      cwd: dir,
    })
    expect(r.source).toBe('cli')
    expect(r.name).toBe('alpha')
  })

  it('env wins over .mspconfig and default', async () => {
    const dir = await fresh()
    await seedRegistry({ a: { path: '/a' }, b: { path: '/b' } }, 'a')
    const cfg = join(dir, '.mspconfig')
    await writeFile(cfg, 'project: a\n', 'utf8')
    const r = await resolveProject({ env: 'b', cwd: dir })
    expect(r.source).toBe('env')
    expect(r.name).toBe('b')
  })

  it('.mspconfig wins over default', async () => {
    const dir = await fresh()
    await seedRegistry({ a: { path: '/a' }, b: { path: '/b' } }, 'a')
    const cfg = join(dir, '.mspconfig')
    await writeFile(cfg, 'project: b\n', 'utf8')
    const r = await resolveProject({ cwd: dir })
    expect(r.source).toBe('mspconfig')
    expect(r.name).toBe('b')
    expect(r.mspconfigPath).toBe(cfg)
  })

  it('walks up from cwd to find .mspconfig in an ancestor', async () => {
    const dir = await fresh()
    await seedRegistry({ root: { path: '/r' } })
    await writeFile(join(dir, '.mspconfig'), 'project: root\n', 'utf8')
    const sub = join(dir, 'a/b/c')
    await mkdir(sub, { recursive: true })
    const r = await resolveProject({ cwd: sub })
    expect(r.source).toBe('mspconfig')
    expect(r.name).toBe('root')
  })

  it('falls back to registry `default` field', async () => {
    const dir = await fresh()
    await seedRegistry({ alpha: { path: '/a' }, gamma: { path: '/g' } }, 'gamma')
    const r = await resolveProject({ cwd: dir })
    expect(r.source).toBe('default')
    expect(r.name).toBe('gamma')
  })

  it('falls back to literal `default` when no `default` field set', async () => {
    const dir = await fresh()
    await seedRegistry({ default: { path: '/d' } })
    const r = await resolveProject({ cwd: dir })
    expect(r.source).toBe('default')
    expect(r.name).toBe('default')
  })

  it('errors loudly when resolved name is not in the registry', async () => {
    const dir = await fresh()
    await seedRegistry({ alpha: { path: '/a' } })
    await expect(
      resolveProject({ cliFlag: 'unknown', cwd: dir }),
    ).rejects.toThrow(/not registered/)
  })

  it('errors loudly when registry is empty and no `default` is registered', async () => {
    const dir = await fresh()
    // No registry at all → empty default → fallback name 'default' not in registry
    await expect(resolveProject({ cwd: dir })).rejects.toThrow(/not registered/)
  })

  it('returns the entry for the resolved project', async () => {
    const dir = await fresh()
    await seedRegistry(
      { eva: { path: '/eva' } },
      'eva',
    )
    const r = await resolveProject({ cwd: dir })
    expect(r.entry.path).toBe('/eva')
  })
})
