import { describe, expect, it } from 'vitest'

import { createMspMcpServer, REGISTERED_TOOL_NAMES } from '../../src/mcp/server.js'

describe('createMspMcpServer', () => {
  it('returns a server object', () => {
    const server = createMspMcpServer()
    expect(server).toBeDefined()
  })

  it('registers exactly the 19 MSP-specific tools, no more no less', () => {
    expect([...REGISTERED_TOOL_NAMES].sort()).toEqual([
      'msp_backlinks_rebuild',
      'msp_candidate',
      'msp_compress',
      'msp_episode_append',
      'msp_identity_get',
      'msp_identity_set',
      'msp_project_list',
      'msp_project_register',
      'msp_project_resolve',
      'msp_recall',
      'msp_remember',
      'msp_run_task',
      'msp_session_append',
      'msp_symbol_community',
      'msp_symbol_impact',
      'msp_symbol_lookup',
      'msp_symbol_neighbors',
      'msp_symbol_search',
      'msp_validate',
    ])
  })

  it('does not duplicate any gks_* tool names', () => {
    for (const name of REGISTERED_TOOL_NAMES) {
      expect(name.startsWith('gks_')).toBe(false)
      expect(name.startsWith('msp_')).toBe(true)
    }
  })

  it('respects MSP_ROOT env var', () => {
    const old = process.env.MSP_ROOT
    process.env.MSP_ROOT = '/tmp/msp-test-root'
    try {
      const server = createMspMcpServer()
      expect(server).toBeDefined()
    } finally {
      process.env.MSP_ROOT = old
    }
  })
})
