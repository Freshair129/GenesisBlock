import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import { handler, name } from '../../../src/mcp/tools/candidate.js'
import { REGISTERED_TOOL_NAMES } from '../../../src/mcp/server.js'

const tmpRoots: string[] = []
afterEach(async () => {
  for (const dir of tmpRoots.splice(0)) {
    await rm(dir, { recursive: true, force: true })
  }
})

async function freshRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-candidate-tool-'))
  tmpRoots.push(root)
  return root
}

describe('msp_candidate tool', () => {
  it('has the right name', () => {
    expect(name).toBe('msp_candidate')
  })

  it('is registered in createMspMcpServer alongside the other 18 tools', () => {
    expect(REGISTERED_TOOL_NAMES).toContain('msp_candidate')
    expect(REGISTERED_TOOL_NAMES).toHaveLength(19)
  })

  it('writes a candidate file under .brain/.../candidates/ inside ctx.root', async () => {
    const root = await freshRoot()
    const result = await handler({ root })({
      proposed_id: 'CONCEPT--TOOL-FOO',
      type: 'concept',
      title: 'tool smoke',
      body: 'placeholder',
    })
    const payload = JSON.parse(result.content[0]!.text)
    expect(payload.proposed_id).toBe('CONCEPT--TOOL-FOO')
    expect(payload.candidate_path).toBe(
      resolve(root, '.brain/msp/projects/evaAI/candidates/CONCEPT--TOOL-FOO.md'),
    )
    expect(payload.overwritten).toBe(false)
    const raw = await readFile(payload.candidate_path, 'utf8')
    expect(raw).toMatch(/^proposed_id: CONCEPT--TOOL-FOO$/m)
  })

  it('rejects malformed proposed_id', async () => {
    const root = await freshRoot()
    await expect(
      handler({ root })({
        proposed_id: 'concept--lower',
        type: 'concept',
        title: 't',
        body: 'b',
      }),
    ).rejects.toThrow(/Invalid proposed_id/)
  })

  it('propagates rationale and confidence into frontmatter', async () => {
    const root = await freshRoot()
    const result = await handler({ root })({
      proposed_id: 'ADR--TOOL-BAR',
      type: 'adr',
      title: 'rationale propagation',
      body: 'b',
      rationale: 'because tests',
      confidence: 0.7,
    })
    const payload = JSON.parse(result.content[0]!.text)
    const raw = await readFile(payload.candidate_path, 'utf8')
    expect(raw).toMatch(/^rationale: because tests$/m)
    expect(raw).toMatch(/^confidence: 0\.7$/m)
  })

  it('honours args.root over ctx.root, isolating from real .brain', async () => {
    const ctxRoot = await freshRoot()
    const argRoot = await freshRoot()
    const result = await handler({ root: ctxRoot })({
      proposed_id: 'FEAT--TOOL-ISOLATION',
      type: 'feat',
      title: 'isolation',
      body: 'b',
      root: argRoot,
    })
    const payload = JSON.parse(result.content[0]!.text)
    expect(payload.candidate_path.startsWith(argRoot)).toBe(true)
    expect(payload.candidate_path.startsWith(ctxRoot)).toBe(false)
  })

  it('honours args.namespace', async () => {
    const root = await freshRoot()
    const result = await handler({ root })({
      proposed_id: 'BLUEPRINT--TOOL-NS',
      type: 'blueprint',
      title: 'ns',
      body: 'b',
      namespace: 'team-alpha',
    })
    const payload = JSON.parse(result.content[0]!.text)
    expect(payload.candidate_path).toContain('/projects/team-alpha/candidates/')
  })
})
