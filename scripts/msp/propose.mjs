#!/usr/bin/env node
/**
 * MSP propose wrapper — transparent passthrough to `gks propose-inbound`
 * with one fix: GKS 3.5.6 caps `--phase` at 5, but the master spec uses
 * P6 for AUDIT atoms. This wrapper accepts `--phase=6`, calls GKS with
 * `--phase=5`, then patches the resulting inbound file to set `phase: 6`.
 *
 * All other args are forwarded as-is. Exit code mirrors GKS.
 *
 * Usage:
 *   npm run msp:propose -- AUDIT--FOO --title="..." --body="..." --phase=6 --type=audit
 */

import { spawnSync } from 'node:child_process'
import { readFile, readdir, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'

function parseArgs(argv) {
  const out = { positionals: [], flags: {} }
  for (const a of argv) {
    if (a.startsWith('--')) {
      const eq = a.indexOf('=')
      if (eq === -1) {
        out.flags[a.slice(2)] = true
      } else {
        out.flags[a.slice(2, eq)] = a.slice(eq + 1)
      }
    } else {
      out.positionals.push(a)
    }
  }
  return out
}

async function patchPhaseSix(inboundDir, proposedId) {
  const entries = await readdir(inboundDir)
  const match = entries.find(
    (n) => n.startsWith(`${proposedId}.rev-`) && n.endsWith('.md'),
  )
  if (!match) return false
  const path = resolve(inboundDir, match)
  const text = await readFile(path, 'utf8')
  const patched = text.replace(/^phase: 5$/m, 'phase: 6')
  if (patched === text) return false
  await writeFile(path, patched, 'utf8')
  return true
}

async function main() {
  const { positionals, flags } = parseArgs(process.argv.slice(2))
  const wantsPhase6 = flags.phase === '6' || flags.phase === 6
  const root = resolve(flags.root ?? process.cwd())

  const args = [...process.argv.slice(2)]
  if (wantsPhase6) {
    const i = args.findIndex((a) => a === '--phase=6' || a === '--phase' && args[args.indexOf(a) + 1] === '6')
    if (i >= 0 && args[i] === '--phase=6') args[i] = '--phase=5'
    if (i >= 0 && args[i] === '--phase') args[i + 1] = '5'
  }

  const r = spawnSync('npx', ['gks', 'propose-inbound', ...args], {
    stdio: 'inherit',
  })
  if (r.status !== 0) {
    process.exit(r.status ?? 1)
  }

  if (wantsPhase6) {
    const proposedId = positionals[0]
    if (!proposedId) {
      console.error('msp:propose: cannot patch phase 6 — no proposed_id positional')
      process.exit(1)
    }
    const inboundDir = resolve(
      root,
      '.brain/msp/projects/evaAI/inbound',
    )
    const patched = await patchPhaseSix(inboundDir, proposedId)
    if (patched) {
      console.log(`✓ patched ${proposedId} to phase: 6 (per ADR--PATH-ENCODING M3d)`)
    } else {
      console.warn(`⚠ msp:propose: phase=6 requested but inbound file for ${proposedId} not found or already at phase 6`)
    }
  }
}

main().catch((err) => {
  console.error('msp:propose: failed:', err.message)
  process.exit(1)
})
