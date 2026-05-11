import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import predicate, {
  REQUIRED_SECTIONS,
  missingSections,
  stripFrontmatter,
} from '../../../src/validator/proto/master-body-schema.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'

function masterEntry(id: string): AtomicIndexEntry {
  return {
    id,
    phase: 0,
    type: 'master',
    status: 'draft',
    vault_id: 'default',
    path: `master/${id}.md`,
  } as AtomicIndexEntry & { tier: string }
}

function withTier(entry: AtomicIndexEntry, tier: string): AtomicIndexEntry {
  return { ...(entry as object), tier } as unknown as AtomicIndexEntry
}

function makeBody(sections: ReadonlyArray<string>): string {
  return [
    '---',
    'id: MASTER--FOO',
    'tier: master',
    '---',
    '',
    '# MASTER — Foo',
    '',
    ...sections.flatMap((s) => [s, '', 'lorem', '']),
  ].join('\n')
}

describe('PROTO--MASTER-BODY-SCHEMA helpers', () => {
  it('stripFrontmatter removes the leading YAML block', () => {
    const body = stripFrontmatter('---\nid: MASTER--X\n---\n\n## Intent\nx\n')
    expect(body).toMatch(/^## Intent\b/)
  })

  it('missingSections finds gaps and respects exact-string match', () => {
    const body = ['## Intent', '', '## Why', '', '## Directives'].join('\n')
    expect(missingSections(body)).toEqual(['## Apply when', '## Conflicts with'])
  })

  it('REQUIRED_SECTIONS lists the five canonical headings', () => {
    expect(REQUIRED_SECTIONS).toEqual([
      '## Intent',
      '## Why',
      '## Directives',
      '## Apply when',
      '## Conflicts with',
    ])
  })
})

describe('PROTO--MASTER-BODY-SCHEMA predicate', () => {
  let repoRoot: string

  beforeEach(async () => {
    repoRoot = await mkdtemp(join(tmpdir(), 'master-body-'))
    await mkdir(join(repoRoot, 'gks/master'), { recursive: true })
  })

  afterEach(async () => {
    await rm(repoRoot, { recursive: true, force: true })
  })

  it('passes when a Master atom has all five H2 sections', async () => {
    const id = 'MASTER--HAPPY'
    await writeFile(
      join(repoRoot, 'gks/master', `${id}.md`),
      makeBody(REQUIRED_SECTIONS),
      'utf8',
    )
    const result = await predicate({
      atomicIndex: [withTier(masterEntry(id), 'master')],
      repoRoot,
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('flags an error when one required section is missing', async () => {
    const id = 'MASTER--MISSING'
    await writeFile(
      join(repoRoot, 'gks/master', `${id}.md`),
      makeBody(['## Intent', '## Why', '## Directives', '## Apply when']),
      'utf8',
    )
    const result = await predicate({
      atomicIndex: [withTier(masterEntry(id), 'master')],
      repoRoot,
    })
    expect(result.ok).toBe(false)
    expect(result.violations).toHaveLength(1)
    expect(result.violations[0]!.atomId).toBe(id)
    expect(result.violations[0]!.severity).toBe('error')
    expect(result.violations[0]!.message).toMatch(/## Conflicts with/)
  })

  it('ignores non-Master atoms entirely', async () => {
    // Concept atom with no body sections — should be ignored because tier !== master.
    const concept = withTier(
      {
        id: 'CONCEPT--FOO',
        phase: 1,
        type: 'concept',
        status: 'stable',
        vault_id: 'default',
        path: 'concept/CONCEPT--FOO.md',
      } as AtomicIndexEntry,
      'genesis',
    )
    const result = await predicate({
      atomicIndex: [concept],
      repoRoot,
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })
})
