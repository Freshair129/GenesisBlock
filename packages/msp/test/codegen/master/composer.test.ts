import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import {
  composeMasterAtoms,
  estimateTokens,
  formatAsPromptFragment,
} from '../../../src/codegen/master/composer.js'

const tmpRoots: string[] = []
afterEach(async () => {
  for (const dir of tmpRoots.splice(0)) {
    await rm(dir, { recursive: true, force: true })
  }
})

async function freshRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-master-composer-'))
  tmpRoots.push(root)
  return root
}

interface FixtureOpts {
  tier?: string
  type?: string
  status?: string
  body?: string
  /** Where to place the file relative to <root>/gks/. Default: 'master'. */
  subdir?: string
}

async function writeFixtureMaster(
  root: string,
  id: string,
  opts: FixtureOpts = {},
): Promise<void> {
  const subdir = opts.subdir ?? 'master'
  const dir = join(root, 'gks', subdir)
  await mkdir(dir, { recursive: true })
  const tier = opts.tier ?? 'master'
  const type = opts.type ?? 'master'
  const status = opts.status ?? 'draft'
  const body = opts.body ?? `# ${id}\n\nDefault fixture body.`
  const fmLines = [
    '---',
    `id: ${id}`,
    'phase: 0',
    `type: ${type}`,
    `status: ${status}`,
    `tier: ${tier}`,
    'source_type: axiomatic',
    'promoted_from: CONCEPT--FIXTURE',
    'promoted_at: 2026-05-09T07:00:00.000Z',
    'promotion_adr: ADR--MASTER-PROMOTION-FIXTURE',
    `title: ${id}`,
    'created_at: 2026-05-09T07:00:00.000Z',
    '---',
  ]
  await writeFile(join(dir, `${id}.md`), `${fmLines.join('\n')}\n\n${body}\n`, 'utf8')
}

describe('estimateTokens', () => {
  it('returns 0 for empty / whitespace-only text', () => {
    expect(estimateTokens('')).toBe(0)
    expect(estimateTokens('   \n\t  ')).toBe(0)
  })

  it('uses the words * 1.3 heuristic, rounded', () => {
    // 10 whitespace-separated tokens * 1.3 = 13
    const text = 'one two three four five six seven eight nine ten'
    expect(estimateTokens(text)).toBe(13)
  })
})

describe('composeMasterAtoms', () => {
  it('composes two Master atoms in input order with concatenated bodies', async () => {
    const root = await freshRoot()
    await writeFixtureMaster(root, 'MASTER--ALPHA', {
      body: '# Alpha\n\nFirst master body.',
    })
    await writeFixtureMaster(root, 'MASTER--BETA', {
      body: '# Beta\n\nSecond master body.',
    })

    const result = await composeMasterAtoms(['MASTER--ALPHA', 'MASTER--BETA'], root)
    expect(result.missing).toEqual([])
    expect(result.composed).toHaveLength(2)
    expect(result.composed[0]!.id).toBe('MASTER--ALPHA')
    expect(result.composed[0]!.body).toContain('First master body.')
    expect(result.composed[1]!.id).toBe('MASTER--BETA')
    expect(result.composed[1]!.body).toContain('Second master body.')
    expect(result.totalTokens).toBe(
      result.composed[0]!.tokenCount + result.composed[1]!.tokenCount,
    )
  })

  it('returns a missing id when the file does not exist', async () => {
    const root = await freshRoot()
    await writeFixtureMaster(root, 'MASTER--PRESENT', { body: 'present body' })

    const result = await composeMasterAtoms(
      ['MASTER--PRESENT', 'MASTER--ABSENT'],
      root,
    )
    expect(result.composed.map((c) => c.id)).toEqual(['MASTER--PRESENT'])
    expect(result.missing).toEqual(['MASTER--ABSENT'])
  })

  it('returns a missing id when the atom is not tier:master', async () => {
    const root = await freshRoot()
    // Place a tier:genesis atom at gks/concept/<id>.md (not in master/).
    await writeFixtureMaster(root, 'CONCEPT--GENESIS', {
      tier: 'genesis',
      type: 'concept',
      subdir: 'concept',
      body: 'genesis content',
    })

    const result = await composeMasterAtoms(['CONCEPT--GENESIS'], root)
    expect(result.composed).toHaveLength(0)
    expect(result.missing).toEqual(['CONCEPT--GENESIS'])
  })

  it('produces token counts via the words * 1.3 rounded heuristic', async () => {
    const root = await freshRoot()
    // Body has exactly 10 whitespace-separated tokens → 10 * 1.3 = 13.
    const body = 'alpha beta gamma delta epsilon zeta eta theta iota kappa'
    await writeFixtureMaster(root, 'MASTER--TOKENS', { body })

    const result = await composeMasterAtoms(['MASTER--TOKENS'], root)
    expect(result.composed).toHaveLength(1)
    expect(result.composed[0]!.tokenCount).toBe(13)
    expect(result.totalTokens).toBe(13)
  })

  it('preserves the order of the input ids', async () => {
    const root = await freshRoot()
    await writeFixtureMaster(root, 'MASTER--ONE', { body: 'one' })
    await writeFixtureMaster(root, 'MASTER--TWO', { body: 'two' })
    await writeFixtureMaster(root, 'MASTER--THREE', { body: 'three' })

    const result = await composeMasterAtoms(
      ['MASTER--TWO', 'MASTER--THREE', 'MASTER--ONE'],
      root,
    )
    expect(result.composed.map((c) => c.id)).toEqual([
      'MASTER--TWO',
      'MASTER--THREE',
      'MASTER--ONE',
    ])
  })
})

describe('formatAsPromptFragment', () => {
  it('joins atoms with section dividers and id markers', async () => {
    const root = await freshRoot()
    await writeFixtureMaster(root, 'MASTER--A', { body: 'body A' })
    await writeFixtureMaster(root, 'MASTER--B', { body: 'body B' })

    const result = await composeMasterAtoms(['MASTER--A', 'MASTER--B'], root)
    const fragment = formatAsPromptFragment(result)
    expect(fragment).toContain('<!-- MASTER--A [FULL] -->')
    expect(fragment).toContain('<!-- MASTER--B [FULL] -->')
    expect(fragment).toContain('body A')
    expect(fragment).toContain('body B')
    expect(fragment).toContain('\n\n---\n\n')
    // Marker for A appears before marker for B.
    expect(fragment.indexOf('<!-- MASTER--A [FULL] -->')).toBeLessThan(
      fragment.indexOf('<!-- MASTER--B [FULL] -->'),
    )
  })

  it('returns the empty string when nothing was composed', () => {
    const empty = formatAsPromptFragment({ composed: [], totalTokens: 0, missing: [] })
    expect(empty).toBe('')
  })
})
