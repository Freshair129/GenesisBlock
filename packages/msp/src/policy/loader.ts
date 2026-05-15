import { readFile, readdir } from 'node:fs/promises'
import { watch } from 'node:fs'
import { join } from 'node:path'
import { parse as parseYaml } from 'yaml'
import { z } from 'zod'

// Zod schema for policies

const MatchSchema = z.object({
  subject_kind: z.array(z.string()).optional(),
  action: z.array(z.string()).optional(),
  origin: z.array(z.string()).optional(),
})

const RuleSchema = z.object({
  id: z.string(),
  description: z.string().optional(),
  match: MatchSchema.optional(),
  condition: z.any().optional(), // Condition type from operators.ts
  effect: z.enum(['permit', 'deny']),
  priority: z.number().int().optional().default(100),
})

const PolicySchema = z.object({
  id: z.string(),
  description: z.string().optional(),
  rules: z.array(RuleSchema),
})

export type Policy = z.infer<typeof PolicySchema>
export type Rule = z.infer<typeof RuleSchema>

export interface PolicySet {
  version: number
  policies: Policy[]
}

let currentPolicySet: PolicySet = { version: 0, policies: [] }
let isWatching = false

/**
 * Load all policies from the policies/ directory.
 */
export async function loadPolicies(policiesDir: string): Promise<PolicySet> {
  const policies: Policy[] = []
  let files: string[] = []
  try {
    files = await readdir(policiesDir)
  } catch (err) {
    const e = err as NodeJS.ErrnoException
    if (e.code === 'ENOENT') {
      console.warn(`[policy] policies directory not found: ${policiesDir}`)
    } else {
      console.warn(`[policy] could not read policies directory: ${policiesDir} (${e.message})`)
    }
    return currentPolicySet
  }

  for (const file of files) {
    if (!file.endsWith('.yaml') && !file.endsWith('.yml')) continue
    const path = join(policiesDir, file)
    try {
      const text = await readFile(path, 'utf8')
      const parsed = parseYaml(text)
      const policy = PolicySchema.parse(parsed)
      policies.push(policy)
    } catch (err) {
      console.error(`[policy] failed to load policy from ${file}: ${(err as Error).message}`)
    }
  }

  currentPolicySet = {
    version: currentPolicySet.version + 1,
    policies,
  }
  return currentPolicySet
}

/**
 * Watch policies directory for changes and hot-reload.
 */
export function watchPolicies(policiesDir: string): void {
  if (isWatching) return
  isWatching = true

  watch(policiesDir, (eventType, filename) => {
    if (filename && (filename.endsWith('.yaml') || filename.endsWith('.yml'))) {
      console.info(`[policy] hot-reloading policies due to change in ${filename}...`)
      loadPolicies(policiesDir).catch((err) => {
        console.error(`[policy] hot-reload failed: ${err.message}`)
      })
    }
  })
}

export function getPolicySet(): PolicySet {
  return currentPolicySet
}
