import { readFile, readdir } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { parse as parseYaml } from 'yaml'
import { VaultSchema, type Vault, type VaultRegistry } from './types.js'
import { type Namespace } from '@freshair129/gks'

let currentRegistry: VaultRegistry = { version: 0, vaults: new Map() }

/**
 * Load all vault configurations from the specified directory.
 * Default directory: ~/.msp/vaults/ (or project-relative .msp/vaults/ for local dev).
 */
export async function loadVaults(vaultsDir: string): Promise<VaultRegistry> {
  const vaults = new Map<string, Vault>()
  let files: string[] = []
  try {
    files = await readdir(vaultsDir)
  } catch (err) {
    // Gracefully handle missing directory
    return currentRegistry
  }

  for (const file of files) {
    if (!file.endsWith('.yaml') && !file.endsWith('.yml')) continue
    const path = join(vaultsDir, file)
    try {
      const text = await readFile(path, 'utf8')
      const parsed = parseYaml(text)
      const vault = VaultSchema.parse(parsed)
      vaults.set(vault.id, vault)
    } catch (err) {
      console.error(`[vault] failed to load vault from ${file}: ${(err as Error).message}`)
    }
  }

  currentRegistry = {
    version: currentRegistry.version + 1,
    vaults,
  }
  return currentRegistry
}

export function resolveVault(vaultId: string): Vault | null {
  return currentRegistry.vaults.get(vaultId) ?? null
}

/**
 * Returns the effective read namespaces for a vault.
 * Falls back to the vault's write_to if read_from is missing (should not happen per schema).
 */
export function vaultReadNamespaces(vault: Vault): Namespace[] {
  return vault.read_from
}

/**
 * Returns the effective write namespace for a vault.
 * Returns null if the vault is read-only.
 */
export function vaultWriteNamespace(vault: Vault): Namespace | null {
  return vault.write_to ?? null
}

export function getVaultRegistry(): VaultRegistry {
  return currentRegistry
}
