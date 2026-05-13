/**
 * Tests for `src/master/promote-apply.ts`. tmpdir-only — never touches
 * the live `gks/inbound/` or `gks/master/`.
 */
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import { applyPromotion } from '../../src/master/promote-apply.js'
import { readRegistry } from '../../src/master/registry.js'

const tmpRoots: string[] = []

afterEach(async () => {
  for (const dir of tmpRoots.splice(0)) {
    await rm(dir, { recursive: true, force: true })
  }
})

async function freshRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-apply-'))
  tmpRoots.push(root)
  return root
}

interface ProposalFixture {
  readonly masterId: string
  readonly blockId: string
  readonly extraFrontmatter?: readonly string[]
  readonly body?: string
}

async function writeProposalFixture(
  root: string,
  fx: ProposalFixture,
): Promise<string> {
  const inbound = join(root, 'gks', 'inbound')
  await mkdir(inbound, { recursive: true })
  const lines = [
    '---',
    `id: MASTER--${fx.masterId}`,
    'phase: 0',
    'type: master',
    'status: draft',
    'source_type: axiomatic',
    `promoted_from: GENESIS--${fx.blockId}`,
    'promoted_at: 2026-05-14T03:00:00.000Z',
    `promotion_adr: ADR--MASTER-PROMOTION-${fx.blockId}`,
    'vault_id: default',
    `title: ${fx.masterId} — Master`,
    'tags:',
    '  - msp',
    '  - master',
    'created_at: 2026-05-14T10:00:00.000+07:00',
    ...(fx.extraFrontmatter ?? []),
    '---',
    '',
    fx.body ?? `# MASTER — ${fx.masterId}\n\n## Intent\nTODO`,
  ]
  const path = join(inbound, `MASTER--${fx.masterId}.proposal.md`)
  await writeFile(path, lines.join('\n'), 'utf8')
  return path
}

describe('applyPromotion', () => {
  it('moves the proposal into gks/master/ and writes a registry entry', async () => {
    const root = await freshRoot()
    const proposalPath = await writeProposalFixture(root, {
      masterId: 'IDENTITY-ENGINE',
      blockId: 'IDENTITY-ENGINE',
    })

    const result = await applyPromotion(proposalPath, root)

    expect(result.master_id).toBe('MASTER--IDENTITY-ENGINE')
    expect(result.master_path).toContain(
      join('gks', 'master', 'MASTER--IDENTITY-ENGINE.md'),
    )
    expect(existsSync(result.master_path)).toBe(true)

    const written = await readFile(result.master_path, 'utf8')
    expect(written).toContain('id: MASTER--IDENTITY-ENGINE')
    expect(written).toContain('promoted_from: GENESIS--IDENTITY-ENGINE')
    expect(written).toContain('tier: master')

    const entries = await readRegistry(root)
    expect(entries).toHaveLength(1)
    expect(entries[0]!.block_id).toBe('IDENTITY-ENGINE')
    expect(entries[0]!.status).toBe('active')

    // Original proposal renamed to .applied.
    expect(existsSync(proposalPath)).toBe(false)
    expect(existsSync(`${proposalPath}.applied`)).toBe(true)
  })

  it('stamps tier: master when the proposal lacked it', async () => {
    const root = await freshRoot()
    const proposalPath = await writeProposalFixture(root, {
      masterId: 'NO-TIER',
      blockId: 'NO-TIER',
    })
    const result = await applyPromotion(proposalPath, root)
    const written = await readFile(result.master_path, 'utf8')
    // Look for `tier: master` in the frontmatter.
    const frontmatter = written.slice(0, written.indexOf('\n---', 4) + 4)
    expect(frontmatter).toMatch(/^tier:\s*master\s*$/m)
  })

  it('overwrites promoted_at with the current instant', async () => {
    const root = await freshRoot()
    const before = new Date()
    const proposalPath = await writeProposalFixture(root, {
      masterId: 'STAMP-TEST',
      blockId: 'STAMP-TEST',
    })
    const result = await applyPromotion(proposalPath, root)
    const after = new Date()
    const written = await readFile(result.master_path, 'utf8')
    const m = written.match(/^promoted_at:\s*(\S+)\s*$/m)
    expect(m).not.toBeNull()
    const stampedAt = new Date(m![1]!)
    expect(stampedAt.getTime()).toBeGreaterThanOrEqual(before.getTime() - 1)
    expect(stampedAt.getTime()).toBeLessThanOrEqual(after.getTime() + 1)
    // And it should differ from the proposal's original stamp.
    expect(m![1]).not.toBe('2026-05-14T03:00:00.000Z')
  })

  it('throws when the target master file already exists', async () => {
    const root = await freshRoot()
    const proposalPath = await writeProposalFixture(root, {
      masterId: 'DUPLICATE',
      blockId: 'DUPLICATE',
    })
    await mkdir(join(root, 'gks', 'master'), { recursive: true })
    await writeFile(
      join(root, 'gks', 'master', 'MASTER--DUPLICATE.md'),
      '---\nid: MASTER--DUPLICATE\n---\nexisting',
      'utf8',
    )
    await expect(applyPromotion(proposalPath, root)).rejects.toThrow(
      /already exists/,
    )
  })

  it('throws when the proposal frontmatter lacks promoted_from', async () => {
    const root = await freshRoot()
    const inbound = join(root, 'gks', 'inbound')
    await mkdir(inbound, { recursive: true })
    const proposalPath = join(inbound, 'MASTER--MALFORMED.proposal.md')
    await writeFile(
      proposalPath,
      [
        '---',
        'id: MASTER--MALFORMED',
        'type: master',
        '---',
        '',
        '# body',
      ].join('\n'),
      'utf8',
    )
    await expect(applyPromotion(proposalPath, root)).rejects.toThrow(
      /malformed proposal/,
    )
  })

  it('throws when the proposal file is missing', async () => {
    const root = await freshRoot()
    await expect(
      applyPromotion(join(root, 'gks', 'inbound', 'nope.md'), root),
    ).rejects.toThrow(/cannot read proposal/)
  })

  it('preserves the body verbatim after the frontmatter', async () => {
    const root = await freshRoot()
    const proposalPath = await writeProposalFixture(root, {
      masterId: 'BODY-TEST',
      blockId: 'BODY-TEST',
      body: '# MASTER — BODY-TEST\n\n## Intent\nThe rule.\n',
    })
    const result = await applyPromotion(proposalPath, root)
    const written = await readFile(result.master_path, 'utf8')
    expect(written).toContain('# MASTER — BODY-TEST')
    expect(written).toContain('## Intent\nThe rule.')
  })
})
