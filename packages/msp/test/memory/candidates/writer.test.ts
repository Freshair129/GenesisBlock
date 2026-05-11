import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import { CandidateWriter } from '../../../src/memory/candidates/writer.js'
import {
  CandidateIdError,
  CandidateNotFoundError,
} from '../../../src/memory/candidates/types.js'

const tmpRoots: string[] = []
afterEach(async () => {
  for (const dir of tmpRoots.splice(0)) {
    await rm(dir, { recursive: true, force: true })
  }
})

async function freshRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-candidates-'))
  tmpRoots.push(root)
  return root
}

const VALID_INPUT = {
  type: 'concept',
  proposed_id: 'CONCEPT--TEST-FOO',
  title: 'Test Foo',
  body: 'Some body markdown.',
}

describe('CandidateWriter.write', () => {
  it('creates the candidate file under .brain/.../candidates/ and returns the path', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    const result = await w.write(VALID_INPUT)
    expect(result.path).toBe(
      resolve(root, '.brain/msp/projects/evaAI/candidates/CONCEPT--TEST-FOO.md'),
    )
    expect(result.overwritten).toBe(false)
    const raw = await readFile(result.path, 'utf8')
    expect(raw).toMatch(/^---\nproposed_id: CONCEPT--TEST-FOO\n/)
    expect(raw).toMatch(/^type: concept$/m)
    expect(raw).toMatch(/^status: candidate$/m)
    expect(raw).toMatch(/^proposed_by: agent$/m)
    expect(raw).toMatch(/^proposed_at: \d{4}-\d{2}-\d{2}T/m)
    expect(raw).toMatch(/# Test Foo\n\nSome body markdown\./)
  })

  it('honours custom namespace', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root, namespace: 'team-x' })
    const result = await w.write(VALID_INPUT)
    expect(result.path).toContain('/projects/team-x/candidates/')
  })

  it('honours proposedBy = human', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root, proposedBy: 'human' })
    const result = await w.write(VALID_INPUT)
    const raw = await readFile(result.path, 'utf8')
    expect(raw).toMatch(/^proposed_by: human$/m)
  })

  it('rejects malformed proposed_id', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    await expect(
      w.write({ ...VALID_INPUT, proposed_id: 'concept--lowercase' }),
    ).rejects.toBeInstanceOf(CandidateIdError)
    await expect(
      w.write({ ...VALID_INPUT, proposed_id: 'UNKNOWN--FOO' }),
    ).rejects.toBeInstanceOf(CandidateIdError)
    await expect(
      w.write({ ...VALID_INPUT, proposed_id: 'CONCEPT--' }),
    ).rejects.toBeInstanceOf(CandidateIdError)
  })

  it('emits overwritten=true when the file already exists', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    await w.write(VALID_INPUT)
    const second = await w.write({ ...VALID_INPUT, body: 'replaced.' })
    expect(second.overwritten).toBe(true)
    const raw = await readFile(second.path, 'utf8')
    expect(raw).toContain('replaced.')
    expect(raw).not.toContain('Some body markdown.')
  })

  it('writes optional rationale and confidence when provided', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    const result = await w.write({
      ...VALID_INPUT,
      rationale: 'why this matters',
      confidence: 0.85,
    })
    const raw = await readFile(result.path, 'utf8')
    expect(raw).toMatch(/^rationale: why this matters$/m)
    expect(raw).toMatch(/^confidence: 0\.85$/m)
  })

  it('omits rationale and confidence keys when not provided', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    const result = await w.write(VALID_INPUT)
    const raw = await readFile(result.path, 'utf8')
    expect(raw).not.toMatch(/^rationale:/m)
    expect(raw).not.toMatch(/^confidence:/m)
  })

  it('preserves a body that already starts with a heading', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    const result = await w.write({
      ...VALID_INPUT,
      title: 'Ignored',
      body: '# Custom Heading\n\nSome content.',
    })
    const raw = await readFile(result.path, 'utf8')
    const after = raw.split(/\n---\n\n/)[1]!
    expect(after.startsWith('# Custom Heading')).toBe(true)
    expect(after).not.toContain('# Ignored')
  })
})

describe('CandidateWriter.list', () => {
  it('returns [] when the directory does not exist', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    expect(await w.list()).toEqual([])
  })

  it('returns summaries sorted by proposed_at desc', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    await w.write({ ...VALID_INPUT, proposed_id: 'CONCEPT--A', title: 'A' })
    await new Promise((r) => setTimeout(r, 5))
    await w.write({ ...VALID_INPUT, proposed_id: 'CONCEPT--B', title: 'B' })
    const summaries = await w.list()
    expect(summaries.map((s) => s.proposed_id)).toEqual(['CONCEPT--B', 'CONCEPT--A'])
    expect(summaries[0]!.title).toBe('B')
    expect(summaries[0]!.status).toBe('candidate')
  })

  it('skips non-markdown files', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    await w.write(VALID_INPUT)
    const dir = w.candidatesDir()
    await writeFile(join(dir, 'notes.txt'), 'not a candidate', 'utf8')
    const summaries = await w.list()
    expect(summaries).toHaveLength(1)
  })
})

describe('CandidateWriter.read', () => {
  it('returns full record including body', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    await w.write(VALID_INPUT)
    const record = await w.read('CONCEPT--TEST-FOO')
    expect(record.proposed_id).toBe('CONCEPT--TEST-FOO')
    expect(record.title).toBe('Test Foo')
    expect(record.body).toContain('Some body markdown.')
  })

  it('throws CandidateNotFoundError when the file is missing', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    await expect(w.read('CONCEPT--MISSING')).rejects.toBeInstanceOf(CandidateNotFoundError)
  })
})

describe('CandidateWriter.delete', () => {
  it('removes the candidate file', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    await w.write(VALID_INPUT)
    await w.delete('CONCEPT--TEST-FOO')
    await expect(w.read('CONCEPT--TEST-FOO')).rejects.toBeInstanceOf(CandidateNotFoundError)
  })

  it('throws CandidateNotFoundError when the file is missing', async () => {
    const root = await freshRoot()
    const w = new CandidateWriter({ root })
    await expect(w.delete('CONCEPT--MISSING')).rejects.toBeInstanceOf(CandidateNotFoundError)
  })
})
