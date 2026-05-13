import { dispatch } from '../agents/dispatch.js'
import type { DispatchTask, Tier } from '../agents/types.js'
import { findActiveMaster } from '../master/registry.js'

import { composePrompt } from './composer.js'
import { loadManifest, loadMembers } from './loader.js'
import {
  DIMENSIONS,
  type ExecuteOptions,
  type ExecuteResult,
  type LoadedMembers,
} from './types.js'

function totalMembers(members: LoadedMembers): number {
  let n = 0
  for (const dim of DIMENSIONS) {
    n += members[dim].length
  }
  return n
}

/**
 * Execute a Genesis Block end-to-end.
 *
 * Orchestrates:
 *   1. loadManifest()      — parse the GENESIS--<blockId> frontmatter
 *   2. loadMembers()       — resolve every member atom and read its body
 *   3. findActiveMaster()  — check the Promoted-Block Registry; if the
 *                            block has graduated to Master tier, flip the
 *                            default-tier baseline up to T2 and surface
 *                            `from_master: true` in the result
 *   4. composePrompt()     — concatenate bodies by dimension + append userPrompt
 *   5. dispatch()          — send the composed prompt through the tier router
 *
 * `dispatch()` is called with `type: 'codegen'`, `severity: 'regular'`.
 * The `codegen` type biases routing toward T2 (per
 * `BLUEPRINT--AGENT-DISPATCHER`), which is the right default for the
 * structured-composite workload a Genesis Block represents.
 *
 * Tier resolution precedence:
 *   - `opts.tier` (explicit caller override) → forwarded as `budget_hint`.
 *   - Otherwise, if a registry hit exists → `'T2'` as the explicit floor.
 *   - Otherwise → no `budget_hint`, dispatcher routes automatically.
 *
 * Per `CONCEPT--PROMOTED-BLOCK-REGISTRY`, Master-tier blocks are presumed
 * important enough to warrant T2 minimum (the safe always-allowed cloud
 * tier per `BLUEPRINT--AGENT-DISPATCHER`). Callers can still force T3 by
 * passing `tier: 'T3'`; the cost-policy still applies — asking for T3 on a
 * regular-severity task will throw, NOT silently downgrade.
 */
export async function executeBlock(
  blockId: string,
  opts: ExecuteOptions,
): Promise<ExecuteResult> {
  const startedAt = Date.now()

  const manifest = await loadManifest(blockId, opts.root)
  const members = await loadMembers(manifest, opts.root)

  // Phase F1: surface registry membership to the runtime.
  const masterEntry = await findActiveMaster(opts.root, manifest.id)
  const fromMaster = masterEntry !== null

  const composedPrompt = composePrompt(manifest, members, opts.prompt)

  // Tier resolution: explicit opt > master-floor > none.
  let budgetHint: Tier | undefined
  if (opts.tier !== undefined) {
    budgetHint = opts.tier
  } else if (fromMaster) {
    budgetHint = 'T2'
  }

  const task: DispatchTask = {
    type: 'codegen',
    severity: 'regular',
    prompt: composedPrompt,
    ...(budgetHint ? { budget_hint: budgetHint } : {}),
  }

  const result = await dispatch(task)

  return {
    block_id: manifest.id,
    output: result.output,
    members_loaded: totalMembers(members),
    tier_used: result.tier_used,
    duration_ms: Date.now() - startedAt,
    ...(fromMaster ? { from_master: true } : {}),
  }
}
