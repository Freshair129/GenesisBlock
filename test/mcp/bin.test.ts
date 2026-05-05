import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const repoRoot = fileURLToPath(new URL('../..', import.meta.url))
const binSrc = `${repoRoot}/src/mcp/bin.ts`

interface JsonRpc {
  jsonrpc: '2.0'
  id?: number | string
  method?: string
  params?: unknown
  result?: unknown
  error?: unknown
}

function callServer(messages: JsonRpc[]): Promise<JsonRpc[]> {
  return new Promise((resolveProm, rejectProm) => {
    const child = spawn('npx', ['tsx', binSrc], {
      cwd: repoRoot,
      env: { ...process.env, MSP_ROOT: repoRoot },
    })
    let stdout = ''
    let stderr = ''
    child.stdout.on('data', (d) => (stdout += d.toString()))
    child.stderr.on('data', (d) => (stderr += d.toString()))
    child.on('error', rejectProm)
    child.on('close', () => {
      const responses: JsonRpc[] = []
      for (const line of stdout.split('\n')) {
        const trimmed = line.trim()
        if (!trimmed) continue
        try {
          responses.push(JSON.parse(trimmed))
        } catch {
          // Non-JSON lines (logs etc.) — ignore.
        }
      }
      if (responses.length === 0 && stderr) rejectProm(new Error(`server stderr: ${stderr}`))
      else resolveProm(responses)
    })
    for (const m of messages) {
      child.stdin.write(JSON.stringify(m) + '\n')
    }
    child.stdin.end()
  })
}

describe('msp-mcp-server bin (spawned)', () => {
  it('responds to initialize + tools/list with all 10 tools', async () => {
    const responses = await callServer([
      {
        jsonrpc: '2.0',
        id: 1,
        method: 'initialize',
        params: {
          protocolVersion: '2024-11-05',
          capabilities: {},
          clientInfo: { name: 'smoke', version: '1' },
        },
      },
      { jsonrpc: '2.0', method: 'notifications/initialized' },
      { jsonrpc: '2.0', id: 2, method: 'tools/list' },
    ])
    const init = responses.find((r) => r.id === 1)
    const list = responses.find((r) => r.id === 2)
    expect(init?.result).toBeDefined()
    expect(list?.result).toBeDefined()
    const tools = (list!.result as { tools: Array<{ name: string }> }).tools
    expect(tools.map((t) => t.name).sort()).toEqual([
      'msp_backlinks_rebuild',
      'msp_episode_append',
      'msp_identity_get',
      'msp_identity_set',
      'msp_propose',
      'msp_recall',
      'msp_remember',
      'msp_run_task',
      'msp_session_append',
      'msp_validate',
    ])
  }, 30_000)
})
