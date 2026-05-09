# 🔵 Proposal 05 — Symbol Graph as a first-class GKS layer

**Filed upstream**: TBD (will be opened on `Freshair129/GksV3` after this PR merges into MSP `main`)
**MSP origin**: PRs #57 → #62 (2026-05-09); see `gks/audit/AUDIT--SYMBOL-GRAPH-V1.md`

## Why

GKS today owns two derived knowledge artefacts:

- `gks/00_index/atomic_index.jsonl` — searchable atom index
- `.brain/<consumer>/projects/<ns>/vector/backlinks.jsonl` — atom-to-atom edge index

Both describe the **conceptual graph** over Markdown atoms. Neither captures the **structural graph** over the consumer's source code: which symbols call which, which classes extend which, which interfaces are implemented where, which logical communities the codebase clusters into.

MSP just shipped a Symbol Graph layer that fills this gap (see audit). The implementation is generic — `Symbol`, `Edge`, `Community` are language-neutral; the parser is a swappable interface; the Leiden adapter and the SQLite + JSONL persistence don't depend on anything MSP-specific. The natural home is **GKS** — the same package that already owns `atomic_index.jsonl`. MSP would then become a thin wrapper exposing the GKS-provided graph through `msp_symbol_*` MCP tools and the Knowledge Browser.

## What

Add a `@freshair129/gks/symbols` module to GKS exposing the API surface MSP just stabilised:

```typescript
// Build / load
export async function buildSymbolGraph(opts: BuildOpts): Promise<SymbolGraphMeta>
export function openSymbolGraph(root: string): SymbolStore | null  // null when not built

// Query
export function lookupSymbol(store, name: string, kind?: SymbolKind): Symbol[]
export function getNeighbors(store, id: string, depth?: number, types?: EdgeType[]): { nodes; edges }
export function getImpact(store, id: string): { callers: Array<{symbol; distance}>; count }
export function getCommunity(store, id: string): { community; members; edges }
export function searchSymbols(store, query: string, limit?: number): Array<Symbol & {score}>
export function getStats(store): SymbolGraphMeta

// Schema
export interface Symbol { id; name; kind; file; start_line; end_line; exported; parent_id; signature; community_id; created_at }
export interface Edge { src_id; dst_id; type; weight; resolved }
export interface Community { id; size; label; modularity; parent_id }
export interface SymbolGraphMeta { schema_version; last_built_at; parser; algorithm; symbol_count; edge_count; community_count; parse_errors[] }
```

Shapes verbatim from `src/symbols/types.ts` in MSP.

## Schema (SQLite)

Verbatim from `gks/blueprint/BLUEPRINT--SYMBOL-GRAPH-CORE.md`:

```sql
CREATE TABLE symbols (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  kind          TEXT NOT NULL,
  file          TEXT NOT NULL,
  start_line    INTEGER NOT NULL,
  end_line      INTEGER NOT NULL,
  exported      INTEGER NOT NULL DEFAULT 0,
  parent_id     TEXT,
  signature     TEXT,
  community_id  INTEGER,
  created_at    TEXT NOT NULL
);
CREATE INDEX idx_symbols_name      ON symbols(name);
CREATE INDEX idx_symbols_file      ON symbols(file);
CREATE INDEX idx_symbols_community ON symbols(community_id);
CREATE INDEX idx_symbols_kind      ON symbols(kind);

CREATE TABLE edges (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  src_id    TEXT NOT NULL,
  dst_id    TEXT NOT NULL,
  type      TEXT NOT NULL,            -- calls|extends|implements|imports|references|defines
  weight    REAL NOT NULL DEFAULT 1.0,
  resolved  INTEGER NOT NULL DEFAULT 1,
  UNIQUE(src_id, dst_id, type)
);
CREATE INDEX idx_edges_src ON edges(src_id, type);
CREATE INDEX idx_edges_dst ON edges(dst_id, type);

CREATE TABLE communities (id INTEGER PRIMARY KEY, size INTEGER, label TEXT, modularity REAL, parent_id INTEGER);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
```

JSONL exports (sorted by primary key) for git-diff visibility:

```
.brain/<consumer>/projects/<ns>/symbols/{symbols,edges,communities}.jsonl
```

## Storage location

Mirror existing pattern:

- `.brain/<consumer>/projects/<ns>/symbols/graph.db` — SQLite (gitignored)
- `.brain/<consumer>/projects/<ns>/symbols/{symbols,edges,communities}.jsonl` — JSONL (committable; sorted; deterministic)
- `.brain/<consumer>/projects/<ns>/symbols/meta.json` — `SymbolGraphMeta`

## Determinism

- JSONL writes sorted by primary key
- Leiden uses `randomSeed = 42` by default; mulberry32 RNG keeps it deterministic across runs
- `dump-jsonl` round-trip test asserts byte-identity (build → dump → load fresh → dump → byte-compare). MSP's `test/symbols/store.test.ts` already covers this and would migrate verbatim.

## Migration path

1. **Phase 1 (this proposal merging into GKS)**: GKS adds `@freshair129/gks/symbols` exporting the surface above. Schema + CLI + `meta.json` shape locked.
2. **Phase 2 (next MSP release)**: MSP's `src/symbols/{parser,store,communities}` becomes a re-export shim that calls into GKS:
   ```typescript
   export { buildSymbolGraph, openSymbolGraph, lookupSymbol, ... } from '@freshair129/gks/symbols'
   ```
   MSP's MCP tools, CLI, and Web UI keep their current public surface; only the wrapped implementation moves.
3. **Phase 3 (post-migration)**: MSP deletes the now-redundant impl. JSONL exports are byte-identical pre/post-migration (deterministic build), so consumer projects don't see a diff. Migration is invisible.

## Dependencies

- `better-sqlite3` (^11.x) — SQLite wrapper. Native module but ships prebuilt binaries for Node 20+22 Linux/Mac/Windows; CI-stable.
- `graphology` (^0.25.x) + `graphology-types` — graph data structure
- `@aflsolutions/graphology-communities-leiden` (^1.1.x) — primary community detection
- `graphology-communities-louvain` (^2.0.x) — fallback (loaded via dynamic `import()`)

> ⚠️ `@aflsolutions/graphology-communities-leiden` is a **single-maintainer fork** of the official `graphology` ecosystem. If GKS picks this up: recommend either (a) vendoring the algorithm into `node_modules/@freshair129/gks/dist/symbols/leiden-vendored.js`, or (b) contributing the fork upstream to the `graphology` org. Louvain fallback keeps the build alive if either fails.

## Parser surface

MSP's v1 ships only a TypeScript Compiler API parser. To stay language-neutral inside GKS, expose:

```typescript
export interface SymbolParser {
  parseFile(absolutePath: string, repoRoot: string): { symbols: Symbol[]; edges: Edge[] }
  // Optional capability flags
  readonly language: string  // 'typescript' | 'python' | ...
}
```

GKS would ship `@freshair129/gks/symbols/parsers/typescript` as the only built-in initially. Tree-sitter parsers (`@freshair129/gks/symbols/parsers/python`, etc.) follow in Phase 2 of GKS's own roadmap.

## What this proposal does NOT change

- No change to `atomic_index.jsonl` or `backlinks.jsonl` (sibling indices)
- No change to GKS atom schema, crosslinks vocabulary, or phase model
- No change to GKS CLI surface beyond adding `gks symbols build|query|stats|community|impact|dump-jsonl` (mirroring MSP's `msp:graph` aliases — exact subcommand names are GKS's call)
- MSP keeps its `msp_symbol_*` MCP tools (consumer-side surface)

## Open questions for upstream review

1. Should GKS ship the parser as a default extra (`@freshair129/gks/symbols`), a peer dep (consumer installs `tree-sitter` etc. themselves), or fully optional with a registry of parser plugins?
2. Schema versioning policy — current MSP v1 has `meta.schema_version = 1`; should GKS bump on absorb?
3. Where does the `dump-jsonl` round-trip test live? (Suggest: `tests/memory/symbols/store.test.ts` mirroring MSP's location.)
4. Should `meta.algorithm` field track Leiden vs Louvain, or push the choice into a higher-level config so consumers can plug their own community detector?

## Source

- MSP `gks/audit/AUDIT--SYMBOL-GRAPH-V1.md` (this PR)
- MSP `gks/blueprint/BLUEPRINT--SYMBOL-GRAPH-CORE.md` (PR #59)
- MSP `gks/adr/ADR--SYMBOL-GRAPH-PERSISTENCE.md` + `ADR--LEIDEN-COMMUNITY-DETECTION.md` (PR #58)
- MSP `gks/feat/FEAT--MSP-SYMBOL-MCP.md` + `FEAT--MSP-GRAPH-CLI.md` + `FEAT--SYMBOLS-WEB-TAB.md` (PR #58)
- Real run on the MSP repo: 897 symbols / 1843 edges / 143 communities (modularity 0.958, 62s build)
