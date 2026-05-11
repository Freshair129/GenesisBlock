/**
 * Symbol Graph type definitions — see ADR--SYMBOL-GRAPH-PERSISTENCE
 * + BLUEPRINT--SYMBOL-GRAPH-CORE for the rationale and full schema.
 *
 * These types mirror the SQLite schema 1:1; the JSONL exports are flattened
 * views of the same shape (sorted by primary key for git diff visibility).
 */

export type SymbolKind =
  | 'function'
  | 'method'
  | 'class'
  | 'interface'
  | 'type'
  | 'enum'
  | 'const'
  | 'module'

export type EdgeType =
  | 'defines'
  | 'imports'
  | 'calls'
  | 'extends'
  | 'implements'
  | 'references'

export interface Symbol {
  /** Composite id: `<relative-file>:<name>:<kind-shorthand>` (e.g. `src/foo.ts:bar:func`). */
  id: string
  name: string
  kind: SymbolKind
  /** Repo-relative POSIX path. */
  file: string
  start_line: number
  end_line: number
  exported: boolean
  /** Parent symbol id (e.g. method's class), or null for top-level. */
  parent_id: string | null
  /** Optional human-readable signature for display (`fn(a, b): T`). */
  signature: string | null
  /** Filled by Leiden after detection; null until then. */
  community_id: number | null
  /** ISO 8601 — set at upsert time. */
  created_at: string
}

export interface Edge {
  src_id: string
  dst_id: string
  type: EdgeType
  /** Uniform 1.0 in v1; reserved for future call-frequency / semantic weighting. */
  weight: number
  /** false when the parser couldn't statically resolve the target (e.g. dynamic call). */
  resolved: boolean
}

export interface Community {
  id: number
  size: number
  /** Heuristic label `<top-dir>/<top-symbol-name>`; nullable. */
  label: string | null
  modularity: number | null
  /** Reserved for hierarchical Leiden (Phase 2); null in v1. */
  parent_id: number | null
}

export interface SymbolGraphMeta {
  schema_version: number
  last_built_at: string
  parser: 'typescript' | 'tree-sitter'
  algorithm: 'leiden' | 'louvain'
  leiden_resolution: number
  leiden_seed: number
  symbol_count: number
  edge_count: number
  community_count: number
  parse_errors: ParseError[]
}

export interface ParseError {
  file: string
  message: string
}

export interface ParseResult {
  symbols: Symbol[]
  edges: Edge[]
}

export interface SymbolParser {
  /**
   * Parse a single source file and emit the symbols + edges it defines.
   *
   * MUST NOT throw on syntax error — return `{ symbols: [], edges: [] }` and
   * let the caller record the failure in `meta.parse_errors[]`.
   *
   * @param absolutePath absolute path to the file on disk
   * @param repoRoot     absolute path to the repo root; used to compute the
   *                     POSIX-relative file path for symbol ids
   */
  parseFile(absolutePath: string, repoRoot: string): ParseResult
}

export interface CommunityDetectionResult {
  /** Map<symbolId, communityId>. */
  partition: Map<string, number>
  modularity: number
  algorithm: 'leiden' | 'louvain'
}

export interface CommunityDetector {
  run(
    symbols: Symbol[],
    edges: Edge[],
    opts: { resolution: number; seed: number },
  ): CommunityDetectionResult
}
