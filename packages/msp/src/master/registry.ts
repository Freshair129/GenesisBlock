/**
 * Promoted-Block Registry — append-only JSONL log over `gks/master/`.
 *
 * Per `CONCEPT--PROMOTED-BLOCK-REGISTRY` and
 * `BLUEPRINT--MASTER-RUNTIME-INTEGRATION`, this module provides the fast
 * runtime-discoverable index over the canonical `gks/master/MASTER--*.md`
 * atoms. Each line of `<root>/gks/master/registry.jsonl` is one promotion
 * event; the executor reads it via `findActiveMaster()` to decide whether
 * a Genesis Block has graduated to Master tier.
 *
 * Authority:
 *   - `CONCEPT--PROMOTED-BLOCK-REGISTRY` (concept)
 *   - `BLUEPRINT--MASTER-RUNTIME-INTEGRATION` (plan)
 *
 * The registry is **derived state** (the Master atom is the source of
 * truth) and is gitignored. `readRegistry` therefore tolerates a missing
 * file (→ `[]`) and silently skips malformed lines.
 */
import { appendFile, mkdir, readFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'

/**
 * One promotion event captured in the registry.
 *
 * - `block_id` matches `GenesisManifest.id` (the bare slug, no `GENESIS--`
 *   prefix). E.g. `IDENTITY-ENGINE`.
 * - `promoted_at` is an ISO UTC timestamp (Z-suffixed). Wall-clock ICT is
 *   reserved for atom frontmatter `created_at:`; here we want comparable
 *   instants.
 * - `promotion_pr` is optional free-form text — typically the PR or commit
 *   that landed the Master atom + evidence ADR. Not parsed.
 * - `status` is the registry-only state (`'active' | 'archived'`); it is
 *   distinct from the Master atom's frontmatter `status:` (which is the
 *   standard atom lifecycle).
 */
export interface MasterEntry {
  readonly block_id: string
  readonly promoted_at: string
  readonly promotion_pr?: string
  readonly status: 'active' | 'archived'
}

const REGISTRY_RELPATH = 'gks/master/registry.jsonl'

/**
 * Read every entry from `<root>/gks/master/registry.jsonl`.
 *
 * Returns `[]` if the file does not exist (the F1 boot state). Malformed
 * JSON lines are silently skipped — the registry is derived state, so a
 * partially corrupted file shouldn't crash the executor; a future
 * rebuild command (out of F1 scope) can regenerate it from the
 * `gks/master/MASTER--*.md` atoms.
 */
export async function readRegistry(root: string): Promise<MasterEntry[]> {
  const path = resolve(root, REGISTRY_RELPATH)
  let raw: string
  try {
    raw = await readFile(path, 'utf8')
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return []
    throw err
  }
  const lines = raw.split(/\r?\n/)
  const out: MasterEntry[] = []
  for (const line of lines) {
    const trimmed = line.trim()
    if (trimmed.length === 0) continue
    const entry = tryParseEntry(trimmed)
    if (entry !== null) out.push(entry)
  }
  return out
}

/**
 * Append one entry to `<root>/gks/master/registry.jsonl`.
 *
 * Creates `<root>/gks/master/` (with parents) if missing. Writes
 * `JSON.stringify(entry) + '\n'`. Never reads or rewrites prior entries —
 * the registry is append-only by contract so concurrent appliers don't
 * race on the same file.
 */
export async function appendRegistry(
  root: string,
  entry: MasterEntry,
): Promise<void> {
  const path = resolve(root, REGISTRY_RELPATH)
  await mkdir(dirname(path), { recursive: true })
  await appendFile(path, `${JSON.stringify(entry)}\n`, 'utf8')
}

/**
 * Convenience: find the active registry entry for a given `block_id`.
 *
 * Returns the **last** active entry (last-write-wins) so a future
 * re-promotion event can shadow an older one. Returns `null` when the
 * block has never been promoted, the registry file is missing, or every
 * entry for the block is `archived`.
 */
export async function findActiveMaster(
  root: string,
  blockId: string,
): Promise<MasterEntry | null> {
  const entries = await readRegistry(root)
  let hit: MasterEntry | null = null
  for (const entry of entries) {
    if (entry.block_id === blockId && entry.status === 'active') {
      hit = entry
    }
  }
  return hit
}

function tryParseEntry(line: string): MasterEntry | null {
  let parsed: unknown
  try {
    parsed = JSON.parse(line)
  } catch {
    return null
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return null
  const obj = parsed as Record<string, unknown>
  const blockId = obj['block_id']
  const promotedAt = obj['promoted_at']
  const status = obj['status']
  if (typeof blockId !== 'string' || blockId.length === 0) return null
  if (typeof promotedAt !== 'string' || promotedAt.length === 0) return null
  if (status !== 'active' && status !== 'archived') return null
  const promotionPr = obj['promotion_pr']
  const entry: MasterEntry =
    typeof promotionPr === 'string'
      ? { block_id: blockId, promoted_at: promotedAt, promotion_pr: promotionPr, status }
      : { block_id: blockId, promoted_at: promotedAt, status }
  return entry
}
