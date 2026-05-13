import { mkdir, mkdtemp, readdir, rm, stat, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { main } from '../../src/agents/episode-gc-cli.js'

interface Streams {
  stdout: string
  stderr: string
}

function captureIO(argv: string[]): { streams: Streams; restore: () => void } {
  const streams: Streams = { stdout: '', stderr: '' }
  const origArgv = process.argv
  const origStdoutWrite = process.stdout.write.bind(process.stdout)
  const origStderrWrite = process.stderr.write.bind(process.stderr)

  process.argv = ['node', '/fake/episode-gc-cli.js', ...argv]
  process.stdout.write = ((chunk: string | Uint8Array) => {
    streams.stdout += typeof chunk === 'string' ? chunk : Buffer.from(chunk).toString('utf8')
    return true
  }) as typeof process.stdout.write
  process.stderr.write = ((chunk: string | Uint8Array) => {
    streams.stderr += typeof chunk === 'string' ? chunk : Buffer.from(chunk).toString('utf8')
    return true
  }) as typeof process.stderr.write

  return {
    streams,
    restore: () => {
      process.argv = origArgv
      process.stdout.write = origStdoutWrite
      process.stderr.write = origStderrWrite
    },
  }
}

async function writeOldEpisode(root: string, slug: string): Promise<void> {
  const dir = resolve(root, 'gks', 'episode')
  await mkdir(dir, { recursive: true })
  const created = new Date(Date.now() - 60 * 24 * 60 * 60 * 1000)
  const iso = created.toISOString()
  const id = `EPISODE--AGENT-RUN-${slug}`
  const content =
    `---\n` +
    `id: ${id}\n` +
    `phase: 5\n` +
    `type: episode\n` +
    `status: stable\n` +
    `tier: genesis\n` +
    `source_type: episodic\n` +
    `vault_id: default\n` +
    `title: "EPISODE — fake (${slug})"\n` +
    `tags:\n` +
    `  - agents\n` +
    `  - dispatch\n` +
    `  - t2\n` +
    `created_at: ${iso}\n` +
    `---\n\n` +
    `# ${id}\n\n` +
    `- tier_used: T2\n` +
    `- task.severity: regular\n\n` +
    `## Output\n\n` +
    '```\n' +
    `mock-output\n` +
    '```\n'
  await writeFile(join(dir, `${id}.md`), content, 'utf8')
}

async function listEpisodes(root: string): Promise<string[]> {
  const dir = resolve(root, 'gks', 'episode')
  try {
    const all = await readdir(dir)
    return all.filter((n) => n.startsWith('EPISODE--') && n.endsWith('.md'))
  } catch {
    return []
  }
}

async function archiveCount(root: string): Promise<number> {
  const dir = resolve(root, 'gks', 'episode', '_archive')
  let count = 0
  try {
    const months = await readdir(dir)
    for (const m of months) {
      const st = await stat(join(dir, m))
      if (!st.isDirectory()) continue
      const files = await readdir(join(dir, m))
      count += files.length
    }
  } catch {
    /* no archive */
  }
  return count
}

let captured: { streams: Streams; restore: () => void } | undefined
let root: string

beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), 'msp-episode-gc-cli-'))
})

afterEach(async () => {
  captured?.restore()
  captured = undefined
  await rm(root, { recursive: true, force: true })
})

describe('msp-episode-gc CLI', () => {
  it('--help prints usage and returns 0', async () => {
    captured = captureIO(['--help'])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/msp-episode-gc/)
    expect(captured.streams.stdout).toMatch(/Usage:/)
    expect(captured.streams.stdout).toMatch(/--keep-days/)
    expect(captured.streams.stdout).toMatch(/--apply/)
  })

  it('default (no --apply) is implicit dry-run: does not mutate', async () => {
    await writeOldEpisode(root, 'cli-implicit-1')
    captured = captureIO([`--root=${root}`])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/dry-run/)
    expect(captured.streams.stdout).toMatch(/re-run with --apply/)
    expect(await listEpisodes(root)).toHaveLength(1)
    expect(await archiveCount(root)).toBe(0)
  })

  it('--apply actually archives eligible episodes', async () => {
    await writeOldEpisode(root, 'cli-apply-1')
    await writeOldEpisode(root, 'cli-apply-2')
    captured = captureIO([`--root=${root}`, '--apply'])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/apply archive/)
    expect(await listEpisodes(root)).toHaveLength(0)
    expect(await archiveCount(root)).toBe(2)
  })

  it('--apply --delete unlinks eligible episodes', async () => {
    await writeOldEpisode(root, 'cli-del-1')
    captured = captureIO([`--root=${root}`, '--apply', '--delete'])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/apply --delete/)
    expect(await listEpisodes(root)).toHaveLength(0)
    expect(await archiveCount(root)).toBe(0)
  })

  it('--apply --dry-run keeps dry-run semantics (no mutation)', async () => {
    await writeOldEpisode(root, 'cli-applydry-1')
    captured = captureIO([`--root=${root}`, '--apply', '--dry-run'])
    const code = await main()
    expect(code).toBe(0)
    expect(captured.streams.stdout).toMatch(/dry-run/)
    expect(await listEpisodes(root)).toHaveLength(1)
    expect(await archiveCount(root)).toBe(0)
  })

  it('--json emits GcReport shape', async () => {
    await writeOldEpisode(root, 'cli-json-1')
    captured = captureIO([`--root=${root}`, '--apply', '--json'])
    const code = await main()
    expect(code).toBe(0)
    const parsed = JSON.parse(captured.streams.stdout) as {
      total_scanned: number
      archived: number
      deleted: number
      kept: number
      errors: string[]
    }
    expect(parsed.total_scanned).toBe(1)
    expect(parsed.archived).toBe(1)
    expect(parsed.deleted).toBe(0)
    expect(parsed.kept).toBe(0)
    expect(parsed.errors).toEqual([])
  })

  it('returns 2 on bad --keep-days', async () => {
    captured = captureIO([`--root=${root}`, '--keep-days=abc'])
    const code = await main()
    expect(code).toBe(2)
    expect(captured.streams.stderr).toMatch(/--keep-days/)
  })

  it('returns 0 with empty report when episode dir does not exist', async () => {
    captured = captureIO([`--root=${root}`, '--json'])
    const code = await main()
    expect(code).toBe(0)
    const parsed = JSON.parse(captured.streams.stdout) as { total_scanned: number }
    expect(parsed.total_scanned).toBe(0)
  })

  it('--keep-days=0 with --apply archives all non-error successes', async () => {
    await writeOldEpisode(root, 'k0-apply-a')
    await writeOldEpisode(root, 'k0-apply-b')
    captured = captureIO([`--root=${root}`, '--keep-days=0', '--apply'])
    const code = await main()
    expect(code).toBe(0)
    expect(await listEpisodes(root)).toHaveLength(0)
    expect(await archiveCount(root)).toBe(2)
  })
})
