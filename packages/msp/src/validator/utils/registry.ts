import { existsSync, readFileSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { parse as yamlParse } from 'yaml'

let schemaCache: any = null
let registryCache: any = null
let lookupCache: Record<string, { cluster: string; role: string; phase: number; folder: string; tier: string }> | null = null

function findFile(root: string, filename: string): string | null {
  let current = root
  for (let i = 0; i < 5; i++) {
    const candidate = join(current, filename)
    if (existsSync(candidate)) return candidate
    const parent = dirname(current)
    if (parent === current) break
    current = parent
  }
  return null
}

export function loadSchema(root: string): any {
  if (schemaCache) return schemaCache

  const schemaPath = findFile(root, 'atom_schema.yaml')
  if (!schemaPath) return null

  try {
    schemaCache = yamlParse(readFileSync(schemaPath, 'utf8'))
    return schemaCache
  } catch {
    return null
  }
}

export function loadRegistry(root: string): any {
  if (registryCache) return registryCache

  const registryPath = findFile(root, 'atom_registry.yaml')
  if (!registryPath) return null

  try {
    registryCache = yamlParse(readFileSync(registryPath, 'utf8'))
    return registryCache
  } catch {
    return null
  }
}

export function lookupType(prefix: string, root: string) {
  if (lookupCache) return lookupCache[prefix.toLowerCase()] ?? null

  // Prefer atom_schema.yaml; fall back to atom_registry.yaml for test mocks
  const source = loadSchema(root) ?? loadRegistry(root)
  if (!source) return null

  const flat: Record<string, { cluster: string; role: string; phase: number; folder: string; tier: string }> = {}
  try {
    const taxonomy = source.taxonomy ?? source.schema_config?.taxonomy
    if (!taxonomy?.clusters) return null

    for (const [clusterName, cluster] of Object.entries(taxonomy.clusters) as [string, any][]) {
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
  } catch {
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
