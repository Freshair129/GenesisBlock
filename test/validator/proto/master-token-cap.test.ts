import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import predicate, {
  ERROR_THRESHOLD,
  TOKEN_RATIO,
  WARN_THRESHOLD,
  estimateTokens,
  stripFrontmatter,
} from '../../../src/validator/proto/master-token-cap.js'
import type { AtomicIndexEntry } from '../../../src/validator/types.js'

function masterEntry(id: string): AtomicIndexEntry {
  return {
    id,
    phase: 0,
    type: 'master',
    status: 'draft',
    vault_id: 'default',
    path: `master/${id}.md`,
    tier: 'master',
  } as AtomicIndexEntry
}

/** Build a body of `wordCount` whitespace-separated words plus a small frontmatter. */
function bodyWith(wordCount: number): string {
  const words = Array.from({ length: wordCount }, (_, i) => `w${i}`).join(' ')
  return `---\nid: MASTER--X\ntier: master\n---\n\n${words}\n`
}

describe('PROTO--MASTER-TOKEN-CAP helpers', () => {
  it('stripFrontmatter removes the leading YAML block', () => {
    const out = stripFrontmatter('---\nid: x\n---\nhello\n')
    expect(out).toBe('hello\n')
  })

  it('estimateTokens is whitespace-words × TOKEN_RATIO', () => {
    expect(estimateTokens('one two three')).toBeCloseTo(3 * TOKEN_RATIO)
    expect(estimateTokens('  multiple   spaces   here ')).toBeCloseTo(3 * TOKEN_RATIO)
    expect(estimateTokens('')).toBe(0)
  })

  it('thresholds are 400 warn / 600 error', () => {
    expect(WARN_THRESHOLD).toBe(400)
    expect(ERROR_THRESHOLD).toBe(600)
  })
})

describe('PROTO--MASTER-TOKEN-CAP predicate', () => {
  let repoRoot: string

  beforeEach(async () => {
    repoRoot = await mkdtemp(join(tmpdir(), 'master-tokens-'))
    await mkdir(join(repoRoot, 'gks/master'), { recursive: true })
  })

  afterEach(async () => {
    await rm(repoRoot, { recursive: true, force: true })
  })

  it('passes when body is well under the warn threshold', async () => {
    const id = 'MASTER--SMALL'
    // 100 words * 1.3 = 130 tokens, under 400.
    await writeFile(join(repoRoot, 'gks/master', `${id}.md`), bodyWith(100), 'utf8')
    const result = await predicate({
      atomicIndex: [masterEntry(id)],
      repoRoot,
    })
    expect(result.ok).toBe(true)
    expect(result.violations).toEqual([])
  })

  it('warns when body exceeds the warn threshold but not the error one', async () => {
    const id = 'MASTER--MEDIUM'
    // 400 words * 1.3 = 520 tokens — over 400, under 600.
    await writeFile(join(repoRoot, 'gks/master', `${id}.md`), bodyWith(400), 'utf8')
    const result = await predicate({
      atomicIndex: [masterEntry(id)],
      repoRoot,
    })
    expect(result.violations).toHaveLength(1)
    expect(result.violations[0]!.severity).toBe('warning')
    expect(result.violations[0]!.message).toMatch(/warn cap/)
  })

  it('errors when body exceeds the error threshold', async () => {
    const id = 'MASTER--HUGE'
    // 500 words * 1.3 = 650 tokens — over 600.
    await writeFile(join(repoRoot, 'gks/master', `${id}.md`), bodyWith(500), 'utf8')
    const result = await predicate({
      atomicIndex: [masterEntry(id)],
      repoRoot,
    })
    expect(result.violations).toHaveLength(1)
    expect(result.violations[0]!.severity).toBe('error')
    expect(result.violations[0]!.message).toMatch(/error cap/)
  })
})
