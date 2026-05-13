import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { DispatchResult, DispatchTask } from '../../src/agents/types.js'
import type {
  GenesisManifest,
  LoadedMembers,
} from '../../src/genesis/types.js'
import type { MasterEntry } from '../../src/master/registry.js'

const mocks = vi.hoisted(() => ({
  dispatchMock: vi.fn(),
  loadManifestMock: vi.fn(),
  loadMembersMock: vi.fn(),
  findActiveMasterMock: vi.fn(),
}))

vi.mock('../../src/agents/dispatch.js', () => ({
  dispatch: (task: DispatchTask) => mocks.dispatchMock(task),
}))

vi.mock('../../src/genesis/loader.js', () => ({
  loadManifest: (blockId: string, root: string) =>
    mocks.loadManifestMock(blockId, root),
  loadMembers: (manifest: GenesisManifest, root: string) =>
    mocks.loadMembersMock(manifest, root),
}))

vi.mock('../../src/master/registry.js', () => ({
  findActiveMaster: (root: string, blockId: string) =>
    mocks.findActiveMasterMock(root, blockId),
}))

// Import AFTER vi.mock so the mocked deps are resolved.
import { executeBlock } from '../../src/genesis/executor.js'

function okDispatch(overrides: Partial<DispatchResult> = {}): DispatchResult {
  return {
    tier_used: 'T2',
    output: 'dispatch-output',
    duration_ms: 5,
    ...overrides,
  }
}

function emptyMembers(): LoadedMembers {
  return {
    cognitive: [],
    algo: [],
    concept: [],
    runbook: [],
    params: [],
  }
}

beforeEach(() => {
  mocks.dispatchMock.mockReset()
  mocks.loadManifestMock.mockReset()
  mocks.loadMembersMock.mockReset()
  mocks.findActiveMasterMock.mockReset()
  // Default: no master entry. Tests that need one override per-call.
  mocks.findActiveMasterMock.mockResolvedValue(null)
})

describe('executeBlock', () => {
  it('orchestrates loadManifest → loadMembers → composePrompt → dispatch', async () => {
    const manifest: GenesisManifest = {
      id: 'IDENTITY-ENGINE',
      members: { cognitive: ['COGNITIVE--A'], algo: ['ALGO--B'] },
    }
    mocks.loadManifestMock.mockResolvedValueOnce(manifest)

    const members = emptyMembers()
    members.cognitive = [
      {
        id: 'COGNITIVE--A',
        dimension: 'cognitive',
        body: 'cog-lens',
        path: '/fake/cog.md',
      },
    ]
    members.algo = [
      {
        id: 'ALGO--B',
        dimension: 'algo',
        body: 'algo-body',
        path: '/fake/algo.md',
      },
    ]
    mocks.loadMembersMock.mockResolvedValueOnce(members)
    mocks.dispatchMock.mockResolvedValueOnce(
      okDispatch({ output: 'final-output', tier_used: 'T2' }),
    )

    const result = await executeBlock('IDENTITY-ENGINE', {
      root: '/fake/root',
      prompt: 'user prompt here',
    })

    expect(mocks.loadManifestMock).toHaveBeenCalledWith(
      'IDENTITY-ENGINE',
      '/fake/root',
    )
    expect(mocks.loadMembersMock).toHaveBeenCalledWith(manifest, '/fake/root')

    expect(mocks.dispatchMock).toHaveBeenCalledOnce()
    const [sentTask] = mocks.dispatchMock.mock.calls[0]! as [DispatchTask]
    expect(sentTask.type).toBe('codegen')
    expect(sentTask.severity).toBe('regular')
    expect(sentTask.prompt).toContain('cog-lens')
    expect(sentTask.prompt).toContain('algo-body')
    expect(sentTask.prompt).toContain('user prompt here')
    expect(sentTask.prompt).toContain('## Context (Cognitive)')
    expect(sentTask.prompt).toContain('## User Request')
    expect(sentTask.budget_hint).toBeUndefined()

    expect(result.block_id).toBe('IDENTITY-ENGINE')
    expect(result.output).toBe('final-output')
    expect(result.members_loaded).toBe(2)
    expect(result.tier_used).toBe('T2')
    expect(typeof result.duration_ms).toBe('number')
    expect(result.duration_ms).toBeGreaterThanOrEqual(0)
    expect(result.from_master).toBeUndefined()
  })

  it('forwards opts.tier to dispatch as budget_hint', async () => {
    mocks.loadManifestMock.mockResolvedValueOnce({
      id: 'FOO',
      members: {},
    })
    mocks.loadMembersMock.mockResolvedValueOnce(emptyMembers())
    mocks.dispatchMock.mockResolvedValueOnce(okDispatch({ tier_used: 'T1' }))

    await executeBlock('FOO', {
      root: '/r',
      prompt: 'p',
      tier: 'T1',
    })

    const [sentTask] = mocks.dispatchMock.mock.calls[0]! as [DispatchTask]
    expect(sentTask.budget_hint).toBe('T1')
  })

  it('returns members_loaded = 0 when manifest has no members', async () => {
    mocks.loadManifestMock.mockResolvedValueOnce({ id: 'EMPTY', members: {} })
    mocks.loadMembersMock.mockResolvedValueOnce(emptyMembers())
    mocks.dispatchMock.mockResolvedValueOnce(okDispatch())

    const result = await executeBlock('EMPTY', { root: '/r', prompt: 'p' })
    expect(result.members_loaded).toBe(0)
  })

  it('propagates loadManifest errors', async () => {
    mocks.loadManifestMock.mockRejectedValueOnce(new Error('manifest gone'))
    await expect(
      executeBlock('MISSING', { root: '/r', prompt: 'p' }),
    ).rejects.toThrow(/manifest gone/)
    expect(mocks.dispatchMock).not.toHaveBeenCalled()
  })

  it('propagates dispatch errors', async () => {
    mocks.loadManifestMock.mockResolvedValueOnce({
      id: 'FOO',
      members: {},
    })
    mocks.loadMembersMock.mockResolvedValueOnce(emptyMembers())
    mocks.dispatchMock.mockRejectedValueOnce(new Error('dispatch crashed'))

    await expect(
      executeBlock('FOO', { root: '/r', prompt: 'p' }),
    ).rejects.toThrow(/dispatch crashed/)
  })

  it('uses the dispatcher tier_used in the result', async () => {
    mocks.loadManifestMock.mockResolvedValueOnce({ id: 'FOO', members: {} })
    mocks.loadMembersMock.mockResolvedValueOnce(emptyMembers())
    mocks.dispatchMock.mockResolvedValueOnce(okDispatch({ tier_used: 'T3' }))

    const result = await executeBlock('FOO', { root: '/r', prompt: 'p' })
    expect(result.tier_used).toBe('T3')
  })

  it('flags from_master=true and defaults to T2 when the registry has an active entry', async () => {
    const entry: MasterEntry = {
      block_id: 'IDENTITY-ENGINE',
      promoted_at: '2026-05-14T04:00:00.000Z',
      status: 'active',
    }
    mocks.findActiveMasterMock.mockResolvedValueOnce(entry)
    mocks.loadManifestMock.mockResolvedValueOnce({
      id: 'IDENTITY-ENGINE',
      members: {},
    })
    mocks.loadMembersMock.mockResolvedValueOnce(emptyMembers())
    mocks.dispatchMock.mockResolvedValueOnce(okDispatch({ tier_used: 'T2' }))

    const result = await executeBlock('IDENTITY-ENGINE', {
      root: '/r',
      prompt: 'p',
    })

    expect(mocks.findActiveMasterMock).toHaveBeenCalledWith(
      '/r',
      'IDENTITY-ENGINE',
    )
    const [sentTask] = mocks.dispatchMock.mock.calls[0]! as [DispatchTask]
    expect(sentTask.budget_hint).toBe('T2')
    expect(result.from_master).toBe(true)
  })

  it('honours an explicit opts.tier even when the block is master-promoted', async () => {
    mocks.findActiveMasterMock.mockResolvedValueOnce({
      block_id: 'BIG-BLOCK',
      promoted_at: '2026-05-14T04:00:00.000Z',
      status: 'active',
    })
    mocks.loadManifestMock.mockResolvedValueOnce({
      id: 'BIG-BLOCK',
      members: {},
    })
    mocks.loadMembersMock.mockResolvedValueOnce(emptyMembers())
    mocks.dispatchMock.mockResolvedValueOnce(okDispatch({ tier_used: 'T3' }))

    const result = await executeBlock('BIG-BLOCK', {
      root: '/r',
      prompt: 'p',
      tier: 'T3',
    })

    const [sentTask] = mocks.dispatchMock.mock.calls[0]! as [DispatchTask]
    expect(sentTask.budget_hint).toBe('T3')
    expect(result.from_master).toBe(true)
  })
})
