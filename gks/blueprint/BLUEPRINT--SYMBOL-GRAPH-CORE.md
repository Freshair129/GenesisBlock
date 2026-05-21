---
id: BLUEPRINT--SYMBOL-GRAPH-CORE
phase: 3
type: blueprint
scale_level: L2
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: BLUEPRINT — Symbol Graph core (parser + store + Leiden adapter)
tags: &a1
  - msp
  - symbol-graph
  - blueprint
  - implementation
  - typescript-compiler-api
  - sqlite
  - jsonl
  - leiden
crosslinks: &a2
  implements:
    - FEAT--MSP-GRAPH-CLI
    - FEAT--MSP-SYMBOL-MCP
  references:
    - FRAMEWORK--SYMBOL-GRAPH
    - CONCEPT--SYMBOL-GRAPH
    - CONCEPT--PARSER-CHOICE
    - ADR--SYMBOL-GRAPH-PERSISTENCE
    - ADR--LEIDEN-COMMUNITY-DETECTION
linked_symbols: &a3
  - file: packages/msp/src/symbols/types.ts
  - file: packages/msp/src/symbols/parser/typescript.ts
  - file: packages/msp/src/symbols/store/sqlite.ts
  - file: packages/msp/src/symbols/store/jsonl.ts
  - file: packages/msp/src/symbols/communities/leiden.ts
created_at: 2026-05-09T17:00:00.000+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--SYMBOL-GRAPH-CORE
  phase: 3
  type: blueprint
  scale_level: L2
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: BLUEPRINT — Symbol Graph core (parser + store + Leiden adapter)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-09T17:00:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--SYMBOL-GRAPH-CORE
    phase: 3
    type: blueprint
    scale_level: L2
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: BLUEPRINT — Symbol Graph core (parser + store + Leiden adapter)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-09T17:00:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      id: BLUEPRINT--SYMBOL-GRAPH-CORE
      phase: 3
      type: blueprint
      scale_level: L2
      status: stable
      tier: process
      source_type: axiomatic
      vault_id: default
      title: BLUEPRINT — Symbol Graph core (parser + store + Leiden adapter)
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-09T17:00:00.000+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Implementation plan
      attributes:
        domain: blueprint
      domain: blueprint
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# BLUEPRINT — Symbol Graph core

## Scope

This blueprint covers the **foundation layer** of the Symbol Graph rollout
(PR-3 of the 6-PR sequence): the parser interface, the TypeScript Compiler
API parser implementation, the SQLite store, the JSONL exporter / loader, and
the Leiden community-detection adapter. It does NOT cover:

- The `msp-graph` CLI (PR-4 — see `[[FEAT--MSP-GRAPH-CLI]]`)
- The 5 `msp_symbol_*` MCP tools (PR-4 — see `[[FEAT--MSP-SYMBOL-MCP]]`)
- The web UI tab (PR-5)
- The audit atom (PR-6)

## Architectural pattern

Five small modules behind the `SymbolParser` and `CommunityDetector`
interfaces, mirroring the shape of `src/memory/backlinks/` (pure modules +
deterministic writes + `--check` round-trip):

```
src/symbols/
  types.ts                       — Symbol, Edge, Community, ParseResult,
                                   SymbolGraphMeta, SymbolParser,
                                   CommunityDetector
  parser/
    typescript.ts                — ts.Program-based parseFile()
  store/
    sqlite.ts                    — SymbolStore class (better-sqlite3)
    jsonl.ts                     — dumpJsonl / loadJsonl
  communities/
    leiden.ts                    — detectCommunities() with Louvain fallback
```

Each module exports plain functions over plain data; no class hierarchies
beyond the SQLite wrapper. The hot path (parse + insert + query) is
synchronous via `better-sqlite3`; async wrappers exist for caller
convenience (JSONL I/O is async by nature).

## Parser interface

```typescript
interface SymbolParser {
  parseFile(absolutePath: string, repoRoot: string): { symbols: Symbol[]; edges: Edge[] }
}
```

- `absolutePath` — the file on disk
- `repoRoot` — used to compute the POSIX-relative file path that becomes
  the prefix of every symbol id

Implementations MUST NOT throw on syntax errors — return empty arrays and
let the caller record the failure in `meta.parse_errors[]`.

Symbol id format: `<relative-file>:<name>:<kind-shorthand>` where shorthand
is `function→func`, `class→cls`, `interface→iface`, `type→type`,
`enum→enum`, `const→const`, `method→meth`, `module→mod`. Methods use
`<class-name>.<method-name>` for the name component so `Foo.bar`'s id is
`src/x.ts:Foo.bar:meth`.

The module symbol's `name` is the file path itself (so its id has the file
path appearing twice — once as path, once as name).

## TypeScript Compiler API extraction algorithm

For each input file, run:

1. `ts.createProgram({ rootNames: [absolutePath], options: DEFAULTS })` —
   one-shot program (single root). Defaults: `target=ES2022`,
   `module=NodeNext`, `moduleResolution=NodeNext`, `noEmit=true`,
   `skipLibCheck=true`, `strict=false`, `jsx=Preserve`, `esModuleInterop=true`.
2. `program.getSourceFile(absolutePath)` to get the `ts.SourceFile`.
3. Check `parseDiagnostics` on the source file. If non-empty, return empty
   arrays — partial-AST extraction would emit half-symbols and is out of
   scope for v1.
4. `program.getTypeChecker()` for cross-reference resolution.
5. Emit a `module` symbol for the file itself (`exported: true`,
   `parent_id: null`).
6. `ts.forEachChild(sourceFile, …)` to walk top-level declarations:
   - `ts.isFunctionDeclaration` → emit `function` symbol + `defines` edge
     from module → function. Walk body for `calls` and `references`.
   - `ts.isClassDeclaration` → emit `class` symbol + `defines` edge from
     module → class. Walk `heritageClauses` for `extends` / `implements`
     edges. Walk class members for `method` symbols (each emits a `defines`
     edge from class → method, plus body walk for `calls` / `references`).
   - `ts.isInterfaceDeclaration` → emit `interface` symbol + `defines` edge.
   - `ts.isTypeAliasDeclaration` → emit `type` symbol + `defines` edge.
   - `ts.isEnumDeclaration` → emit `enum` symbol + `defines` edge.
   - `ts.isVariableStatement` → for each `VariableDeclaration` with an
     identifier name, emit `const` symbol + `defines` edge. Walk
     initializer for nested `calls` / `references`.
   - `ts.isImportDeclaration` → emit one `imports` edge per imported
     binding. Targets are `external:<module>:<name>` placeholders for v1
     (cross-file binding deferred — `resolved=false`).

### Call resolution

Inside a function / method / const initializer body:

- `ts.isCallExpression(node)` — for `Identifier` callees, look up
  `checker.getSymbolAtLocation(callee)` and map to a previously-recorded
  declaration via the in-file `Map<ts.Symbol, string>` (`declMap`). Fall
  back to a name-based map (`Map<name, id>`, `nameMap`) when the checker
  can't resolve. Emit a `calls` edge with `resolved=true` if the checker
  matched, `resolved=false` if only the name match fired.
- `ts.isPropertyAccessExpression` callees (`obj.foo()`) — best-effort:
  resolve `expr.name` via `checker`, fall back to the in-file `nameMap`
  with `resolved=false`. Cross-file method binding is Phase 2.

### Type-position references

`ts.isTypeReferenceNode` with an `Identifier` `typeName` emits a
`references` edge (same resolution pattern as call resolution above).

### v1 scope deliberately omitted (Phase 2 TODOs)

- **Generics in detail** — type parameters and constraints are not modeled
  as nodes; type references inside generic positions are extracted normally
  but the generic structure is flattened.
- **Decorators** — `@decorator` factories are not extracted as edges.
- **Namespaces / module declarations** — `namespace foo { … }` is parsed
  but its children aren't recursed into.
- **Re-exports** — `export { x } from './y'` doesn't emit a forwarding
  edge; only the `imports` half is recorded.
- **JSX** — supported by the parser config but no specific JSX-tag
  extraction (deferred).
- **Cross-file binding** — `external:<module>:<name>` placeholders are
  resolved at build time by the CLI (PR-4); the parser stays per-file.

## SQLite schema (verbatim from [[ADR--SYMBOL-GRAPH-PERSISTENCE]])

```sql
CREATE TABLE symbols (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  file TEXT NOT NULL,
  start_line INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  exported INTEGER NOT NULL,
  parent_id TEXT,
  signature TEXT,
  community_id INTEGER,
  created_at TEXT NOT NULL
);

CREATE INDEX idx_symbols_name ON symbols(name);
CREATE INDEX idx_symbols_file ON symbols(file);
CREATE INDEX idx_symbols_kind ON symbols(kind);
CREATE INDEX idx_symbols_community ON symbols(community_id);

CREATE TABLE edges (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  src_id TEXT NOT NULL,
  dst_id TEXT NOT NULL,
  type TEXT NOT NULL,
  weight REAL NOT NULL DEFAULT 1.0,
  resolved INTEGER NOT NULL,
  UNIQUE(src_id, dst_id, type)
);

CREATE INDEX idx_edges_src_type ON edges(src_id, type);
CREATE INDEX idx_edges_dst_type ON edges(dst_id, type);

CREATE TABLE communities (
  id INTEGER PRIMARY KEY,
  size INTEGER NOT NULL,
  label TEXT,
  modularity REAL,
  parent_id INTEGER
);

CREATE TABLE meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

The `meta` table is keyed by string — known keys in v1:

| key | value |
|---|---|
| `schema_version` | `"1"` |
| `last_built_at` | ISO 8601 timestamp |
| `parser` | `"typescript"` |
| `algorithm` | `"leiden"` or `"louvain"` |
| `leiden_resolution` | numeric string (e.g. `"1.0"`) |
| `leiden_seed` | integer string (default `"42"`) |
| `symbol_count` | integer string |
| `edge_count` | integer string |
| `community_count` | integer string |
| `parse_errors` | JSON-encoded `Array<{file, message}>` |

### Migrations

v1 is `schema_version=1`. The bootstrap path runs `CREATE TABLE IF NOT
EXISTS` on every `open()` and stamps `schema_version` if absent. No
migration logic in v1; bumping the version requires a full rebuild
(`store.clearAll()` then re-import). Schema-version-driven migrations land
in Phase 2 alongside hierarchical communities.

### `JOURNAL_MODE = WAL`

Set at open time. Gives us atomic crash recovery and concurrent reader
safety, both useful when MCP tools query the store while a build is in
progress.

## JSONL export schema

Three files, one row per line, sorted by primary key, trailing `\n` only
when the file is non-empty:

### `symbols.jsonl` — sorted by `id`

```json
{"id":"src/foo.ts:bar:func","name":"bar","kind":"function","file":"src/foo.ts","start_line":1,"end_line":5,"exported":true,"parent_id":null,"signature":"function bar(): number","community_id":7,"created_at":"2026-05-09T10:00:00.000Z"}
```

Key order is fixed (id, name, kind, file, start_line, end_line, exported,
parent_id, signature, community_id, created_at). Booleans serialize as
`true`/`false`, nulls as `null`.

### `edges.jsonl` — sorted by `(src_id, dst_id, type)`

```json
{"src_id":"src/foo.ts:bar:func","dst_id":"src/baz.ts:qux:func","type":"calls","weight":1,"resolved":true}
```

Key order: `src_id, dst_id, type, weight, resolved`.

### `communities.jsonl` — sorted by `id` (numeric)

```json
{"id":7,"size":12,"label":"src/validator/idFormat","modularity":0.421,"parent_id":null}
```

Key order: `id, size, label, modularity, parent_id`.

## Leiden adapter API

```typescript
async function detectCommunities(
  symbols: Symbol[],
  edges: Edge[],
  opts: { resolution: number; seed: number },
): Promise<{
  partition: Map<string /* symbolId */, number /* communityId */>
  modularity: number
  algorithm: 'leiden' | 'louvain'
}>
```

### Module-load fallback

Primary: dynamic `import('@aflsolutions/graphology-communities-leiden')`.
On `import` failure, log to stderr (`process.stderr.write`) and try
`import('graphology-communities-louvain')`. If both fail, throw — the build
can't proceed without community detection. The `algorithm` field in the
return value records which one ran, for stamping `meta.algorithm`.

### Determinism

Both algorithms accept an `rng: () => number` callback rather than a
numeric seed. We derive a deterministic mulberry32 RNG from the integer
seed (`opts.seed`, default 42 per `[[ADR--LEIDEN-COMMUNITY-DETECTION]]`):

```typescript
function mulberry32(seed: number): () => number {
  let s = seed >>> 0
  return () => {
    s = (s + 0x6d2b79f5) >>> 0
    let t = s
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
```

Combined with sorted input (symbols by id, edges by src/dst/type), the
output is byte-identical across runs.

### Graph construction

The directed call/import edges collapse to **undirected** for community
detection — Leiden + Louvain both treat the graph as undirected by design.
Edges to symbols outside the graph (e.g. `external:./y:foo` import targets)
are skipped. Self-loops are skipped. Duplicate edges (in either direction)
are skipped.

### Heuristic label

`<top-dir>/<top-symbol-name>`:

- **top-dir** = most-common first directory segment of member files (ties
  broken alphabetically)
- **top-symbol-name** = highest-out-degree exported symbol (modules
  excluded; ties broken by id)
- Label is capped at 60 chars

If a community has no exported non-module symbol, fall back to the
highest-degree non-module symbol. If a community has only module symbols,
the label is the top-dir alone.

## Determinism contract

The build pipeline is deterministic when run twice on the same input:

1. **Parser** — file order is the caller's responsibility (CLI sorts the
   input glob in PR-4). Symbols and edges within a file are emitted in
   AST traversal order, which is stable for a given source file.
2. **SQLite writes** — `INSERT … ON CONFLICT … UPDATE` is order-independent
   given the `UNIQUE(src_id, dst_id, type)` constraint. The hot-path
   exporter (`allSymbols()` / `allEdges()`) sorts by primary key on read.
3. **JSONL writes** — sorted before serialization. Fixed key order via
   explicit object literals (NOT `Object.keys` iteration).
4. **Leiden seed** — fixed integer seed (default 42) → deterministic RNG
   → deterministic partition → deterministic communities.

The round-trip test asserts: `dump → load fresh SQLite → dump → byte-compare`.

## Geography

```
src/symbols/types.ts                        # type module — Symbol/Edge/Community/Meta
src/symbols/parser/typescript.ts            # TS Compiler API SymbolParser
src/symbols/store/sqlite.ts                 # SymbolStore class
src/symbols/store/jsonl.ts                  # dumpJsonl + loadJsonl
src/symbols/communities/leiden.ts           # detectCommunities + label heuristic
test/symbols/parser.test.ts                 # 5 tests — parser shapes
test/symbols/store.test.ts                  # 5 tests — store + JSONL round-trip
```

## Verification plan

- vitest: parser emits expected symbols + edges on minimal fixtures
  (empty / function / class with heritage / import / broken syntax)
- vitest: SymbolStore round-trips a symbol and an edge; getNeighbors(1)
  returns the right neighbor; getCommunityMembers tracks community_id
  updates correctly
- vitest: JSONL round-trip determinism — `dump → load fresh → dump`
  produces byte-identical files
- manual smoke test (in PR description):

  ```bash
  npx tsx -e "
    import {parseFile} from './src/symbols/parser/typescript.js'
    const r = parseFile('/tmp/foo.ts', '/tmp')
    console.log(r.symbols.length, r.edges.length)
  "
  ```

## Implementation order

```yaml
T1 TYPES        : src/symbols/types.ts (Symbol, Edge, Community, ParseResult,
                  SymbolGraphMeta, SymbolParser, CommunityDetector,
                  CommunityDetectionResult)
T2 PARSER       : src/symbols/parser/typescript.ts (parseFile, kind shorthand
                  table, declMap/nameMap resolution, body walk)
T3 STORE        : src/symbols/store/sqlite.ts (SymbolStore class with
                  open/close/migrate, upsert*, get*, getNeighbors,
                  setSymbolCommunity, meta KV)
T4 JSONL        : src/symbols/store/jsonl.ts (dumpJsonl with sorted writes,
                  loadJsonl with strict-typed parsers)
T5 LEIDEN       : src/symbols/communities/leiden.ts (mulberry32, dynamic
                  import + fallback, undirected graph build, label
                  heuristic)
T6 TESTS        : test/symbols/parser.test.ts + test/symbols/store.test.ts
T7 (DEFER)      : CLI lives in PR-4; MCP tools live in PR-4; web UI in PR-5
```

## Implementer: do NOT do

- Write `src/symbols/cli.ts` — that's PR-4
- Register MCP tools — that's PR-4
- Touch `src/index.ts` `/api/symbols/*` — that's PR-5
- Add a competing parser bundle (tree-sitter is Phase 2; gated by a
  follow-up CONCEPT)
- Implement hierarchical Leiden (`parent_id` on communities is reserved
  but not populated in v1)
- Migrate schema versions — v1 sticks with `schema_version=1`; v2 lands
  alongside hierarchical communities

## Source

- `[[FRAMEWORK--SYMBOL-GRAPH]]`, `[[CONCEPT--SYMBOL-GRAPH]]`
- `[[CONCEPT--PARSER-CHOICE]]` — TS Compiler API in v1
- `[[ADR--SYMBOL-GRAPH-PERSISTENCE]]` — SQLite + JSONL hybrid; schema verbatim
- `[[ADR--LEIDEN-COMMUNITY-DETECTION]]` — Leiden primary, Louvain fallback
- `[[FEAT--MSP-GRAPH-CLI]]` — CLI contract this blueprint underpins
- `[[FEAT--MSP-SYMBOL-MCP]]` — MCP tool contract this blueprint underpins
- Existing pattern: `src/memory/backlinks/` (sorted JSONL writer + check
  round-trip)
- npm registry inspection 2026-05-09: `better-sqlite3@11.x`,
  `graphology@0.25/0.26`, `graphology-types@0.24`,
  `@aflsolutions/graphology-communities-leiden@1.1.x`,
  `graphology-communities-louvain@2.0.x`
