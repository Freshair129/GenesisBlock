import { existsSync, readFileSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { parse as yamlParse } from 'yaml'

let registryCache: any = null
let lookupCache: Record<string, { cluster: string; role: string; phase: number; folder: string; tier: string }> | null = null

export function loadRegistry(root: string): any {
  if (registryCache) return registryCache

  let current = root
  let registryPath = join(current, 'atom_registry.yaml')
  let found = false
  for (let i = 0; i < 5; i++) {
    if (existsSync(registryPath)) {
      found = true
      break
    }
    const parent = dirname(current)
    if (parent === current) break
    current = parent
    registryPath = join(current, 'atom_registry.yaml')
  }

  if (!found) {
    return null
  }

  try {
    registryCache = yamlParse(readFileSync(registryPath, 'utf8'))
    return registryCache
  } catch (e) {
    return null
  }
}

export function lookupType(prefix: string, root: string) {
  if (lookupCache) return lookupCache[prefix.toLowerCase()] ?? null

  const registry = loadRegistry(root)
  if (!registry) return null

  const flat: Record<string, { cluster: string; role: string; phase: number; folder: string; tier: string }> = {}
  try {
    for (const [clusterName, cluster] of Object.entries(registry.taxonomy.clusters) as [string, any][]) {
      for (const [typeId, config] of Object.entries(cluster.types) as [string, any][]) {
        flat[typeId.toLowerCase()] = {
          cluster: clusterName,
          role: config.role,
          phase: config.phase,
          folder: config.folder,
          tier: config.tier,
        }
      }
    }
    lookupCache = flat
    return lookupCache[prefix.toLowerCase()] ?? null
  } catch (e) {
    return null
  }
}

export function buildAliases(id: string, existingAliases: unknown, root: string): string[] {
  const prefix = id.split('-')[0]!
  const primary = [prefix]

  const other = Array.isArray(existingAliases)
    ? (existingAliases.filter(x => typeof x === 'string' && !primary.includes(x)) as string[])
    : []

  return [...primary, ...other]
}
