/**
 * Genesis Block Runtime — type contracts.
 *
 * See:
 *   - `CONCEPT--GENESIS-BLOCK-RUNTIME` (gks/concept/) for what + why
 *   - `BLUEPRINT--GENESIS-BLOCK-RUNTIME` (gks/blueprint/) for the public-API plan
 *   - `SPEC--GENESIS-BLOCK-MANIFEST` (gks/spec/) for the manifest frontmatter
 */

/**
 * The five orthogonal dimensions of the EVA 4.0 core, mapped onto the v2.3
 * taxonomy prefixes. A `GENESIS--<NAME>` manifest declares which member
 * atoms fill each dimension.
 */
export type Dimension =
  | 'cognitive'
  | 'algo'
  | 'concept'
  | 'runbook'
  | 'params'

export const DIMENSIONS: readonly Dimension[] = [
  'cognitive',
  'algo',
  'concept',
  'runbook',
  'params',
] as const

/**
 * In-memory representation of a `GENESIS--<NAME>.md` manifest atom.
 *
 * `id` is the bare slug (e.g. `IDENTITY-ENGINE`), NOT the full atom id
 * `GENESIS--IDENTITY-ENGINE`. This keeps the runtime API symmetric with the
 * CLI: callers pass the slug, never the prefix.
 *
 * `members` is normalised: regardless of whether the source manifest used
 * the nested `members.core.<dim>` / `members.optional.<dim>` shape or a
 * flatter `members.<dim>` shape, the loader collapses both into the
 * per-dimension arrays here.
 */
export interface GenesisManifest {
  id: string
  members: {
    algo?: string[]
    concept?: string[]
    cognitive?: string[]
    runbook?: string[]
    params?: string[]
  }
  daci?: {
    driver?: string
    approver?: string
    contributor?: string[]
    informed?: string[]
  }
}

/**
 * One member atom resolved off disk: its id, dimension role, body text
 * (everything after the closing `---`), and absolute path. The body
 * already has trailing whitespace stripped.
 */
export interface LoadedMember {
  id: string
  dimension: Dimension
  body: string
  path: string
}

/**
 * `loadMembers()` returns one of these — a record keyed by dimension, with
 * the loaded atoms in the order they appeared in the manifest. Empty
 * dimensions are present as empty arrays so callers don't need to guard.
 */
export type LoadedMembers = Record<Dimension, LoadedMember[]>

/**
 * Options to `executeBlock()`.
 *
 * - `root` — project root (the one containing `gks/` and `.git/`). Resolved
 *   absolutely by the caller; the runtime does no `cwd` lookups.
 * - `prompt` — the user-facing runtime input. Concatenated last, after all
 *   composed member bodies.
 * - `tier` — optional override, forwarded to `dispatch()` as `budget_hint`.
 *   Unset means the dispatcher routes automatically.
 */
export interface ExecuteOptions {
  root: string
  prompt: string
  tier?: 'T1' | 'T2' | 'T3'
}

/**
 * Outcome of one `executeBlock()` call.
 *
 * `from_master` is set to `true` iff the block was found in the
 * Promoted-Block Registry (see `CONCEPT--PROMOTED-BLOCK-REGISTRY`) at
 * execution time. When unset, the block ran as a normal Genesis Block.
 * The field is optional so existing serialisers / pretty-printers stay
 * valid without updates.
 */
export interface ExecuteResult {
  block_id: string
  output: string
  members_loaded: number
  tier_used: 'T1' | 'T2' | 'T3'
  duration_ms: number
  from_master?: boolean
}
