#!/usr/bin/env node
import { join, resolve } from 'node:path'
import { parseArgs } from 'node:util'
import { loadVaults, resolveVault, getVaultRegistry } from './registry.js'

const HELP = `msp-vault — UCF vault management

Usage:
  msp-vault list [--root=<dir>]
  msp-vault show <vault_id> [--root=<dir>]
  msp-vault check <vault_id> <subject_kind> <subject_id> [--root=<dir>]

Flags:
  --root=<dir>       project root (default: cwd)
  --help             this message
`

async function main(): Promise<number> {
  const { values, positionals } = parseArgs({
    options: {
      root: { type: 'string' },
      help: { type: 'boolean' },
    },
    allowPositionals: true,
  })

  if (values.help || positionals.length === 0) {
    process.stdout.write(HELP)
    return 0
  }

  const root = resolve(values.root ?? process.cwd())
  const vaultsDir = join(root, 'policies') // For now, reuse policies/ for vaults in MVP
  // Actually, blueprint says ~/.msp/vaults/ but for repo-relative we use .msp/vaults/ or similar.
  // I'll check if .msp/vaults exists, else fallback to policies/ for dev.
  
  await loadVaults(vaultsDir)
  const command = positionals[0]

  switch (command) {
    case 'list':
      return doList()
    case 'show':
      return doShow(positionals[1])
    case 'check':
      return doCheck(positionals[1], positionals[2], positionals[3])
    default:
      process.stderr.write(`error: unknown command ${command}\n${HELP}`)
      return 1
  }
}

function doList(): number {
  const registry = getVaultRegistry()
  process.stdout.write(`Vaults (${registry.vaults.size}):\n`)
  for (const vault of registry.vaults.values()) {
    process.stdout.write(`  - ${vault.id}: ${vault.description ?? 'No description'}\n`)
  }
  return 0
}

function doShow(vaultId?: string): number {
  if (!vaultId) {
    process.stderr.write('error: show requires <vault_id>\n')
    return 1
  }
  const vault = resolveVault(vaultId)
  if (!vault) {
    process.stderr.write(`error: vault not found: ${vaultId}\n`)
    return 1
  }
  process.stdout.write(JSON.stringify(vault, null, 2) + '\n')
  return 0
}

function doCheck(vaultId?: string, kind?: string, subId?: string): number {
  if (!vaultId || !kind || !subId) {
    process.stderr.write('error: check requires <vault_id> <subject_kind> <subject_id>\n')
    return 1
  }
  const vault = resolveVault(vaultId)
  if (!vault) {
    process.stderr.write(`error: vault not found: ${vaultId}\n`)
    return 1
  }
  
  process.stdout.write(`Vault ${vaultId} check for ${kind}:${subId}:\n`)
  process.stdout.write(`  Read Namespaces: ${JSON.stringify(vault.read_from)}\n`)
  process.stdout.write(`  Write Namespace: ${JSON.stringify(vault.write_to ?? 'NONE')}\n`)
  
  return 0
}

main().then((code) => process.exit(code))
