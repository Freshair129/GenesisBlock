import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'

import { FORBIDDEN_FIELDS as DEFAULT_FORBIDDEN_FIELDS } from './rules/forbidden-fields.js'

export interface RequiredFieldsConfig {
  default: ReadonlyArray<string>
  byType: ReadonlyMap<string, ReadonlyArray<string>>
}

export interface AtomicContract {
  version: number
  forbiddenFields: ReadonlySet<string>
  requiredFields?: RequiredFieldsConfig
  source: 'yaml' | 'default'
  warnings: string[]
}

const DEFAULT_CONTRACT_PATH = '.brain/msp/LLM_Contract/atomic_contract.yaml'

/**
 * Load the atomic contract from `atomic_contract.yaml`. If the file is
 * missing or invalid, fall back to hardcoded defaults and surface a warning
 * via the returned `warnings` array (callers decide whether to print).
 *
 * The loader never throws — degradation is silent so the validator stays
 * usable in projects that haven't authored the YAML yet.
 */
export async function loadContract(
  root: string,
  contractPath = DEFAULT_CONTRACT_PATH,
): Promise<AtomicContract> {
  const fullPath = resolve(root, contractPath)
  const warnings: string[] = []

  let raw: string
  try {
    raw = await readFile(fullPath, 'utf8')
  } catch (err) {
    const code = (err as NodeJS.ErrnoException).code
    if (code === 'ENOENT') {
      warnings.push(
        `atomic_contract.yaml not found at ${fullPath} — using hardcoded defaults`,
      )
      return defaultContract(warnings)
    }
    warnings.push(`atomic_contract.yaml unreadable: ${(err as Error).message} — using defaults`)
    return defaultContract(warnings)
  }

  let parsed: unknown
  try {
    parsed = parseYaml(raw)
  } catch (err) {
    warnings.push(`atomic_contract.yaml invalid YAML: ${(err as Error).message} — using defaults`)
    return defaultContract(warnings)
  }

  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    warnings.push('atomic_contract.yaml: top-level must be a YAML object — using defaults')
    return defaultContract(warnings)
  }

  const obj = parsed as Record<string, unknown>
  const version = typeof obj.version === 'number' ? obj.version : 0
  if (version < 1) {
    warnings.push('atomic_contract.yaml: missing or unsupported `version` — using defaults')
    return defaultContract(warnings)
  }

  const forbidden = obj.forbidden_fields
  let forbiddenFields: ReadonlySet<string>
  if (Array.isArray(forbidden) && forbidden.every((x) => typeof x === 'string')) {
    forbiddenFields = new Set(forbidden as string[])
  } else {
    warnings.push('atomic_contract.yaml: forbidden_fields missing or not a string array — using defaults')
    forbiddenFields = DEFAULT_FORBIDDEN_FIELDS
  }

  const requiredFields = parseRequiredFields(obj.required_fields, warnings)

  return {
    version,
    forbiddenFields,
    requiredFields,
    source: 'yaml',
    warnings,
  }
}

function parseRequiredFields(
  raw: unknown,
  warnings: string[],
): RequiredFieldsConfig | undefined {
  if (raw === undefined || raw === null) return undefined
  if (typeof raw !== 'object' || Array.isArray(raw)) {
    warnings.push('atomic_contract.yaml: required_fields must be an object — skipping')
    return undefined
  }
  const obj = raw as Record<string, unknown>
  const def = obj.default
  if (!Array.isArray(def) || !def.every((x) => typeof x === 'string')) {
    warnings.push('atomic_contract.yaml: required_fields.default must be a string array — skipping')
    return undefined
  }
  const byTypeMap = new Map<string, ReadonlyArray<string>>()
  if (obj.by_type && typeof obj.by_type === 'object' && !Array.isArray(obj.by_type)) {
    for (const [type, list] of Object.entries(obj.by_type)) {
      if (Array.isArray(list) && list.every((x) => typeof x === 'string')) {
        byTypeMap.set(type, list as string[])
      } else {
        warnings.push(`atomic_contract.yaml: required_fields.by_type.${type} skipped (not a string array)`)
      }
    }
  }
  return { default: def as string[], byType: byTypeMap }
}

function defaultContract(warnings: string[]): AtomicContract {
  return {
    version: 1,
    forbiddenFields: DEFAULT_FORBIDDEN_FIELDS,
    requiredFields: undefined,
    source: 'default',
    warnings,
  }
}
