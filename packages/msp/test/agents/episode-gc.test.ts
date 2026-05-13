import { mkdir, mkdtemp, readdir, rm, stat, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { gcEpisodes } from '../../src/agents/episode-gc.js'

interface FakeEpisodeSpec {
  /** Days before "now" to use as created_at. */
  ageDays: number
  /** Severity to embed in body line `- task.severity: <x>`. */
  severity?: 'critical' | 'regular' | 'low'
  /** When true, body omits the `- tier_used:` line → inferred ok=false. */
  malformed?: boolean
  /** When true, frontmatter includes `ok: false`. */
  forceFail?: boolean
  /** When set, body's `## Output` block is empty (whitespace) → ok=false. */
  emptyOutput?: boolean
  /** Optional override for filename slug; defaults to a stable derived stamp. */
  slug?: string
}

function makeEpisodeMd(spec: FakeEpisodeSpec): { filename: string; content: string } {
  const created = new Date(Date.now() - spec.ageDays * 24 * 60 * 60 * 1000)
  const iso = created.toISOString()
  const stamp = spec.slug ?? iso.replace(/[:.]/g, '-')
  const id = `EPISODE--AGENT-RUN-${stamp}`
  const filename = `${id}.md`

  const severity = spec.severity ?? 'regular'
  const failLine = spec.forceFail === true ? 'ok: false\n' : ''
  const output = spec.emptyOutput === true ? '   ' : 'mock-output-text'
  const tierLine = spec.malformed === true ? '' : '- tier_used: T2\n'

  const frontmatter =
    `---\n` +
    `id: ${id}\n` +
    `phase: 5\n` +
    `type: episode\n` +
    `status: stable\n` +
    `tier: genesis\n` +
    `source_type: episodic\n` +
    `vault_id: default\n` +
    `title: "EPISODE — fake (${stamp})"\n` +
    `tags:\n` +
    `  - agents\n` +
    `  - dispatch\n` +
    `  - t2\n` +
    failLine +
    `created_at: ${iso}\n` +
    `---\n`

  const body =
    `\n# ${id}\n\n` +
    `**Prompt (truncated):** hello world\n\n` +
    tierLine +
    `- duration_ms: 12\n` +
    `- task.type: other\n` +
    `- task.severity: ${severity}\n\n` +
    `## Prompt\n\n` +
    '```\n' +
    `hello world\n` +
    '```\n\n' +
    `## Output\n\n` +
    '```\n' +
    `${output}\n` +
    '```\n'

  return { filename, content: frontmatter + body }
}

async function writeFakeEpisode(root: string, spec: FakeEpisodeSpec): Promise<string> {
  const dir = resolve(root, 'gks', 'episode')
  await mkdir(dir, { recursive: true })
  const { filename, content } = makeEpisodeMd(spec)
  const abspath = join(dir, filename)
  await writeFile(abspath, content, 'utf8')
  return abspath
}

async function listEpisodes(root: string): Promise<string[]> {
  const dir = resolve(root, 'gks', 'episode')
  try {
    const entries = await readdir(dir)
    return entries.filter((n) => n.startsWith('EPISODE--') && n.endsWith('.md'))
  } catch {
    return []
  }
}

async function listArchive(root: string): Promise<string[]> {
  const dir = resolve(root, 'gks', 'episode', '_archive')
  const out: string[] = []
  try {
    const months = await readdir(dir)
    for (const m of months) {
      const monthDir = join(dir, m)
      const st = await stat(monthDir)
      if (!st.isDirectory()) continue
      const files = await readdir(monthDir)
      for (const f of files) out.push(`${m}/${f}`)
    }
  } catch {
    /* no archive yet — fine */
  }
  return out
}

describe('gcEpisodes', () => {
  let root: string

  beforeEach(async () => {
    root = await mkdtemp(join(tmpdir(), 'msp-episode-gc-'))
  })

  afterEach(async () => {
    await rm(root, { recursive: true, force: true })
  })

  it('returns a zero report when the episode dir does not exist', async () => {
    const report = await gcEpisodes(root)
    expect(report).toEqual({
      total_scanned: 0,
      archived: 0,
      deleted: 0,
      kept: 0,
      errors: [],
    })
  })

  it('keeps all recent episodes (≤30 days)', async () => {
    await writeFakeEpisode(root, { ageDays: 1, slug: 'recent-1' })
    await writeFakeEpisode(root, { ageDays: 29, slug: 'recent-2' })

    const report = await gcEpisodes(root)
    expect(report.total_scanned).toBe(2)
    expect(report.kept).toBe(2)
    expect(report.archived).toBe(0)
    expect(report.deleted).toBe(0)
    expect(report.errors).toEqual([])

    const remaining = await listEpisodes(root)
    expect(remaining.length).toBe(2)
  })

  it('default policy: archives old ok+non-critical, keeps old error, keeps recent', async () => {
    // Fixture: 2 recent, 2 old-ok-non-critical, 1 old-error.
    await writeFakeEpisode(root, { ageDays: 1, slug: 'rec-a' })
    await writeFakeEpisode(root, { ageDays: 5, slug: 'rec-b' })
    await writeFakeEpisode(root, { ageDays: 60, slug: 'old-ok-1' })
    await writeFakeEpisode(root, { ageDays: 90, slug: 'old-ok-2' })
    await writeFakeEpisode(root, { ageDays: 100, slug: 'old-err', forceFail: true })

    // Dry-run first: nothing should move.
    const dryReport = await gcEpisodes(root, { dry_run: true })
    expect(dryReport.total_scanned).toBe(5)
    expect(dryReport.archived).toBe(2)
    expect(dryReport.kept).toBe(3)
    expect(dryReport.deleted).toBe(0)
    expect(await listEpisodes(root)).toHaveLength(5)
    expect(await listArchive(root)).toHaveLength(0)

    // Actual run: same plan, but executed.
    const report = await gcEpisodes(root)
    expect(report.total_scanned).toBe(5)
    expect(report.archived).toBe(2)
    expect(report.kept).toBe(3)
    expect(report.deleted).toBe(0)
    expect(report.errors).toEqual([])

    expect(await listEpisodes(root)).toHaveLength(3)
    expect(await listArchive(root)).toHaveLength(2)
  })

  it('archives to <root>/gks/episode/_archive/<YYYY-MM>/ by episode created_at', async () => {
    await writeFakeEpisode(root, { ageDays: 60, slug: 'old-ok-zzz' })
    await gcEpisodes(root)
    const archived = await listArchive(root)
    expect(archived).toHaveLength(1)
    // YYYY-MM regex
    expect(archived[0]).toMatch(/^\d{4}-\d{2}\/EPISODE--/)
  })

  it('keeps critical old successes (audit-trail rule)', async () => {
    await writeFakeEpisode(root, {
      ageDays: 90,
      slug: 'old-critical',
      severity: 'critical',
    })
    const report = await gcEpisodes(root)
    expect(report.total_scanned).toBe(1)
    expect(report.kept).toBe(1)
    expect(report.archived).toBe(0)
    expect(await listEpisodes(root)).toHaveLength(1)
  })

  it('--delete option unlinks instead of archiving', async () => {
    await writeFakeEpisode(root, { ageDays: 60, slug: 'old-del-1' })
    await writeFakeEpisode(root, { ageDays: 60, slug: 'old-del-2' })
    const report = await gcEpisodes(root, { delete: true })
    expect(report.deleted).toBe(2)
    expect(report.archived).toBe(0)
    expect(report.kept).toBe(0)
    expect(await listEpisodes(root)).toHaveLength(0)
    expect(await listArchive(root)).toHaveLength(0)
  })

  it('delete + dry_run still reports the plan without unlinking', async () => {
    await writeFakeEpisode(root, { ageDays: 60, slug: 'old-del-dry-1' })
    const report = await gcEpisodes(root, { delete: true, dry_run: true })
    expect(report.deleted).toBe(1)
    expect(await listEpisodes(root)).toHaveLength(1)
  })

  it('keep_days: 0 archives everything that is not an error', async () => {
    await writeFakeEpisode(root, { ageDays: 0.001, slug: 'k0-fresh-ok' })
    await writeFakeEpisode(root, { ageDays: 0.001, slug: 'k0-fresh-err', forceFail: true })
    await writeFakeEpisode(root, {
      ageDays: 0.001,
      slug: 'k0-fresh-critical',
      severity: 'critical',
    })

    const report = await gcEpisodes(root, { keep_days: 0 })
    expect(report.total_scanned).toBe(3)
    expect(report.archived).toBe(1)
    expect(report.kept).toBe(2)
  })

  it('treats malformed body (no tier_used line) as ok=false → keeps', async () => {
    await writeFakeEpisode(root, {
      ageDays: 90,
      slug: 'old-malformed',
      malformed: true,
    })
    const report = await gcEpisodes(root)
    expect(report.kept).toBe(1)
    expect(report.archived).toBe(0)
  })

  it('treats empty Output block as ok=false → keeps', async () => {
    await writeFakeEpisode(root, {
      ageDays: 90,
      slug: 'old-empty-out',
      emptyOutput: true,
    })
    const report = await gcEpisodes(root)
    expect(report.kept).toBe(1)
    expect(report.archived).toBe(0)
  })

  it('idempotent: a second pass sees zero eligible files', async () => {
    await writeFakeEpisode(root, { ageDays: 60, slug: 'idem-a' })
    await writeFakeEpisode(root, { ageDays: 60, slug: 'idem-b' })
    const first = await gcEpisodes(root)
    expect(first.archived).toBe(2)

    const second = await gcEpisodes(root)
    expect(second.total_scanned).toBe(0)
    expect(second.archived).toBe(0)
    expect(second.kept).toBe(0)
  })

  it('skips non-episode files in the directory', async () => {
    const dir = resolve(root, 'gks', 'episode')
    await mkdir(dir, { recursive: true })
    await writeFile(join(dir, 'README.md'), 'unrelated', 'utf8')
    await writeFile(join(dir, '.DS_Store'), 'meta', 'utf8')
    await writeFakeEpisode(root, { ageDays: 60, slug: 'real-ep' })

    const report = await gcEpisodes(root)
    expect(report.total_scanned).toBe(1)
    expect(report.archived).toBe(1)
  })

  it('records a non-fatal parse error and keeps the malformed atom', async () => {
    const dir = resolve(root, 'gks', 'episode')
    await mkdir(dir, { recursive: true })
    await writeFile(
      join(dir, 'EPISODE--AGENT-RUN-bogus.md'),
      'this has no frontmatter',
      'utf8',
    )
    const report = await gcEpisodes(root)
    expect(report.total_scanned).toBe(1)
    expect(report.kept).toBe(1)
    expect(report.archived).toBe(0)
    expect(report.errors.length).toBeGreaterThan(0)
    expect(report.errors[0]).toMatch(/parse\(EPISODE--AGENT-RUN-bogus\.md\)/)
  })
})
