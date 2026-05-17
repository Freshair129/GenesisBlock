/**
 * Master Block promotion — apply step (human-triggered).
 *
 * Per `CONCEPT--PROMOTED-BLOCK-REGISTRY` and
 * `BLUEPRINT--MASTER-RUNTIME-INTEGRATION`, `applyPromotion()` is the
 * canonical follow-through to `msp-master-propose`: a human reviews the
 * `.proposal.md` under `gks/inbound/`, writes the evidence ADR, hand-edits
 * the body to fit `PROTO--MASTER-TOKEN-CAP`, and then runs
 * `msp-master-propose apply <proposalPath>`. That subcommand calls this
 * module.
 *
 * Behaviour:
 *   1. Read the proposal frontmatter; require `id: MASTER--<NAME>` and
 *      `promoted_from: GENESIS--<NAME>`.
 *   2. Stamp `tier: master`, refresh `promoted_at` to the current ISO UTC
 *      instant (the promotion event is *now*, not when `propose` ran).
 *   3. Write the result to `<root>/gks/master/MASTER--<NAME>.md`. Refuse
 *      to overwrite an existing master file (re-promotion goes through
 *      supersession per `MASTER--ATOM-CONTRADICTION-POLICY`).
 *   4. Append a `MasterEntry` to `gks/master/registry.jsonl`.
 *   5. Rename the proposal to `<original>.applied` (preserves the audit
 *      trail rather than deleting).
 *
 * No auto-promotion: this is invoked explicitly by a human via the CLI.
 */
import { access, readFile, rename, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'

import { appendRegistry } from './registry.js'
import { buildAliases, lookupType } from '../validator/utils/registry.js'

export interface ApplyResult {
  readonly master_id: string
  readonly master_path: string
}

interface ParsedProposal {
  readonly frontmatter: string
  readonly body: string
  readonly masterId: string
  readonly blockId: string
}

const FRONTMATTER_DELIM = '---'

/**
 * Apply a Master proposal: move it from `gks/inbound/` to `gks/master/`,
 * rewrite frontmatter to canonical promoted form, and record the
 * promotion event in the registry.
 *
 * Throws on:
 *   - missing proposal file
 *   - malformed / missing frontmatter
 *   - missing `id: MASTER--<NAME>` or `promoted_from: GENESIS--<NAME>`
 *   - existing file at the target `gks/master/` path
 */
export async function applyPromotion(
  proposalPath: string,
  root: string,
): Promise<ApplyResult> {
  const absProposal = resolve(proposalPath)
  let raw: string
  try {
    raw = await readFile(absProposal, 'utf8')
  } catch (err) {
    throw new Error(
      `applyPromotion: cannot read proposal at ${absProposal}: ${(err as Error).message}`,
    )
  }

  const parsed = parseProposal(raw)
  if (parsed === null) {
    throw new Error(
      `applyPromotion: malformed proposal at ${absProposal} — frontmatter must include \`id: MASTER--<NAME>\` and \`promoted_from: GENESIS--<NAME>\``,
    )
  }

  const masterPath = resolve(root, 'gks', 'master', `${parsed.masterId}.md`)
  if (await pathExists(masterPath)) {
    throw new Error(
      `applyPromotion: target ${masterPath} already exists — re-promotion must go through supersession (MASTER--ATOM-CONTRADICTION-POLICY)`,
    )
  }

  const promotedAt = new Date().toISOString()
  const rewrittenFrontmatter = stampFrontmatter(parsed.frontmatter, promotedAt)
  const finalDoc = `${rewrittenFrontmatter}\n${parsed.body}`

  // Ensure parent dir exists. `appendRegistry` mkdir's the same dir; we
  // do it here too so the master file write doesn't ENOENT.
  const { mkdir } = await import('node:fs/promises')
  await mkdir(resolve(root, 'gks', 'master'), { recursive: true })
  await writeFile(masterPath, finalDoc, 'utf8')

  await appendRegistry(root, {
    block_id: parsed.blockId,
    promoted_at: promotedAt,
    status: 'active',
  })

  // Mark the proposal consumed. `rename` (vs delete) keeps the audit trail.
  const consumedPath = absProposal.endsWith('.applied')
    ? absProposal
    : `${absProposal}.applied`
  if (consumedPath !== absProposal) {
    await rename(absProposal, consumedPath).catch(() => {
      // Non-fatal: the promotion succeeded; cleanup is best-effort.
    })
  }

  return { master_id: parsed.masterId, master_path: masterPath }
}

function parseProposal(source: string): ParsedProposal | null {
  const normalised = source.replace(/\r\n/g, '\n')
  if (!normalised.startsWith(`${FRONTMATTER_DELIM}\n`)) return null
  const end = normalised.indexOf(`\n${FRONTMATTER_DELIM}`, FRONTMATTER_DELIM.length)
  if (end === -1) return null
  const frontmatter = normalised.slice(0, end + 1 + FRONTMATTER_DELIM.length)
  const bodyStart = end + 1 + FRONTMATTER_DELIM.length
  // Skip a single newline after the closing delim if present.
  const body = normalised.slice(bodyStart).replace(/^\n/, '')

  const idMatch = frontmatter.match(/^id:\s*(MASTER--[A-Z0-9][A-Z0-9_-]*)\s*$/m)
  const fromMatch = frontmatter.match(
    /^promoted_from:\s*(GENESIS--[A-Z0-9][A-Z0-9_-]*)\s*$/m,
  )
  if (!idMatch || !fromMatch) return null
  const masterId = idMatch[1]!
  const blockId = fromMatch[1]!.replace(/^GENESIS--/, '')
  return { frontmatter, body, masterId, blockId }
}

/**
 * Rewrite the frontmatter block so:
 *   - `tier: master` is present (added before the closing `---` if missing).
 *   - `promoted_at:` reflects the current promotion instant (overwritten
 *     if present, inserted before the closing `---` if absent).
 *
 * Other fields (id, promoted_from, status, title, tags, crosslinks,
 * created_at) are preserved verbatim. The closing delimiter is preserved
 * exactly.
 */
function stampFrontmatter(frontmatter: string, promotedAt: string): string {
  let updated = frontmatter

  // Overwrite or insert `promoted_at:`.
  if (/^promoted_at:.*$/m.test(updated)) {
    updated = updated.replace(/^promoted_at:.*$/m, `promoted_at: ${promotedAt}`)
  } else {
    updated = insertBeforeClosingDelim(updated, `promoted_at: ${promotedAt}`)
  }

  // Ensure `tier: master` is present.
  if (/^tier:\s*master\s*$/m.test(updated)) {
    // Already correct.
  } else if (/^tier:.*$/m.test(updated)) {
    updated = updated.replace(/^tier:.*$/m, 'tier: master')
  } else {
    updated = insertBeforeClosingDelim(updated, 'tier: master')
  }

  // Inject or overwrite `aliases:`.
  const idMatch = updated.match(/^id:\s*(MASTER--[A-Z0-9][A-Z0-9_-]*)\s*$/m)
  if (idMatch) {
    const masterId = idMatch[1]!
    
    // Extract existing aliases from updated frontmatter if present
    const aliasesMatch = updated.match(/^aliases:\r?\n((?:\s+-\s+.*\r?\n?)*)/m)
    let existing: string[] = []
    if (aliasesMatch) {
      existing = aliasesMatch[1]!
        .split('\n')
        .map(x => x.replace(/^\s*-\s*/, '').trim())
        .filter(x => x.length > 0)
    }

    const aliases = buildAliases(masterId, existing, process.cwd())
    const newAliasesBlock = `aliases:\n${aliases.map(a => `  - ${a}`).join('\n')}`
    
    const aliasesRegex = /^aliases:(?:\r?\n(?:\s+-\s+.*)*)?/m
    if (/^aliases:/m.test(updated)) {
      updated = updated.replace(aliasesRegex, newAliasesBlock)
    } else {
      updated = insertBeforeClosingDelim(updated, newAliasesBlock)
    }

    const prefix = masterId.split('--')[0]!
    const typeDef = lookupType(prefix, process.cwd())
    if (typeDef) {
      // Overwrite or inject cluster
      const clusterLine = `cluster: ${typeDef.cluster}`
      if (/^cluster:\s*.*$/m.test(updated)) {
        updated = updated.replace(/^cluster:\s*.*$/m, clusterLine)
      } else {
        updated = insertBeforeClosingDelim(updated, clusterLine)
      }

      // Overwrite or inject role
      const roleLine = `role: ${typeDef.role}`
      if (/^role:\s*.*$/m.test(updated)) {
        updated = updated.replace(/^role:\s*.*$/m, roleLine)
      } else {
        updated = insertBeforeClosingDelim(updated, roleLine)
      }
    }
  }

  return updated
}

function insertBeforeClosingDelim(frontmatter: string, line: string): string {
  // The frontmatter slice ends with `\n---`. Insert the new line just
  // before that trailing delimiter.
  const trailer = `\n${FRONTMATTER_DELIM}`
  if (!frontmatter.endsWith(trailer)) {
    return `${frontmatter}\n${line}`
  }
  const head = frontmatter.slice(0, frontmatter.length - trailer.length)
  return `${head}\n${line}${trailer}`
}

async function pathExists(p: string): Promise<boolean> {
  try {
    await access(p)
    return true
  } catch {
    return false
  }
}
