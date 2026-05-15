import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const packageRoot = fileURLToPath(new URL('../..', import.meta.url))
const repoRoot = resolve(packageRoot, '../..')
const binSrc = `${packageRoot}/src/mcp/bin.ts`
const binDist = `${packageRoot}/dist/mcp/bin.js`

interface JsonRpc {
  jsonrpc: '2.0'
  id?: number | string
  method?: string
  params?: unknown
  result?: unknown
  error?: unknown
}

interface CallOpts {
  /** Spawn cwd. Defaults to repoRoot. */
  cwd?: string
  /** Extra args appended after the script (e.g. ['--root=G:\\msp']). */
  extraArgs?: string[]
  /** Replace env entirely — drops MSP_ROOT etc. set by callers above. */
  env?: NodeJS.ProcessEnv
}

/**
 * Pick the fastest reliable way to launch the bin:
 * - Prefer `node dist/mcp/bin.js` if a build exists (works in CI + Windows
 *   without depending on npx PATH resolution).
 * - Fall back to `npx tsx src/mcp/bin.ts` for fresh checkouts where `npm test`
 *   wasn't preceded by `npm run build`. CI's `npm ci` resolves npx properly.
 */
function spawnBin(args: readonly string[], cwd: string, env: NodeJS.ProcessEnv) {
  if (existsSync(binDist)) {
    return spawn(process.execPath, [binDist, ...args], { cwd, env })
  }
  return spawn('npx', ['tsx', binSrc, ...args], { cwd, env })
}

function callServer(messages: JsonRpc[], opts: CallOpts = {}): Promise<JsonRpc[]> {
  return new Promise((resolveProm, rejectProm) => {
    const child = spawnBin(
      opts.extraArgs ?? [],
      opts.cwd ?? repoRoot,
      opts.env ?? { ...process.env, MSP_ROOT: repoRoot },
    )
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
  it('responds to initialize + tools/list with all 24 tools', async () => {
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
      'msp_brain_resolve',
      'msp_candidate',
      'msp_compress',
      'msp_dispatch',
      'msp_episode_append',
      'msp_escalate',
      'msp_expand',
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
      'msp_symbol_trace',
      'msp_validate',
    ])
  }, 30_000)

  // Regression test for cwd-resolution bug discovered 2026-05-07: when launched
  // by Claude Desktop, cwd is C:\Windows\system32 — paths must come from --root.
  it('uses --root=<path> argv flag when cwd is unrelated and MSP_ROOT is unset', async () => {
    // Build a clean env that drops MSP_ROOT but keeps PATH (needed for npx/node).
    const cleanEnv: NodeJS.ProcessEnv = {}
    for (const k of ['PATH', 'Path', 'SystemRoot', 'TEMP', 'TMP', 'USERPROFILE', 'HOME', 'APPDATA']) {
      const v = process.env[k]
      if (v) cleanEnv[k] = v
    }

    const responses = await callServer(
      [
        {
          jsonrpc: '2.0',
          id: 1,
          method: 'initialize',
          params: {
            protocolVersion: '2024-11-05',
            capabilities: {},
            clientInfo: { name: 'cwd-bug-regression', version: '1' },
          },
        },
        { jsonrpc: '2.0', method: 'notifications/initialized' },
        {
          jsonrpc: '2.0',
          id: 2,
          method: 'tools/call',
          params: { name: 'msp_validate', arguments: { all: true } },
        },
      ],
      {
        // Spawn from a directory that is NOT the repo — emulates Claude Desktop
        // launching from C:\Windows\system32. tmpdir() works on every platform.
        cwd: tmpdir(),
        extraArgs: [`--root=${repoRoot}`],
        env: cleanEnv,
      },
    )

    const validateResponse = responses.find((r) => r.id === 2)
    expect(validateResponse?.result).toBeDefined()

    // The result is a tool-text-result wrapping JSON. Pull out content[0].text
    // and parse — we just care that it didn't error out with "atomic index unreadable".
    const content = (validateResponse!.result as { content: Array<{ text: string }>; isError?: boolean }).content
    expect(content[0].text).not.toMatch(/atomic index unreadable/)
    expect(content[0].text).not.toMatch(/C:\\\\Windows\\\\system32/)

    const parsed = JSON.parse(content[0].text) as { ok: boolean }
    expect(typeof parsed.ok).toBe('boolean')
  }, 60_000)
})
