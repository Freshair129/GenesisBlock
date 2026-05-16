---
id: ALGO--SYMBOLS-FRAMEWORK-RECOGNITION
phase: 2
type: algo
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Framework recognition algorithms — Next.js routing, runtime tags, data fetching, ORM, MCP tools
tags:
  - msp
  - symbol-graph
  - algorithm
  - nextjs
  - prisma
  - mcp
  - recognition
crosslinks: {"implements":["ADR--SYMBOLS-FRAMEWORK-AWARENESS"],"references":["CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS","FRAMEWORK--SYMBOL-GRAPH"]}
linked_symbols:
  - {"file":"packages/msp/src/symbols/framework/routes.ts"}
  - {"file":"packages/msp/src/symbols/framework/nextjs.ts"}
  - {"file":"packages/msp/src/symbols/framework/runtime-tag.ts"}
  - {"file":"packages/msp/src/symbols/framework/data-fetching.ts"}
  - {"file":"packages/msp/src/symbols/framework/orm.ts"}
  - {"file":"packages/msp/src/symbols/framework/mcp-tools.ts"}
created_at: 2026-05-12T05:37:00.000+07:00
---

# ALGO — Framework recognition algorithms

Concrete recognition logic per framework. Each subsection is one recognizer; implementation goes in `packages/msp/src/symbols/framework/<name>.ts` per the geography declared in `linked_symbols`.

All recognizers conform to the interface in `[[ADR--SYMBOLS-FRAMEWORK-AWARENESS]]` §3:

```ts
interface FrameworkRecognizer {
  readonly id: string
  matches(absolutePath: string, sourceCode?: string): boolean
  recognize(absolutePath: string, repoRoot: string, sourceCode: string): Promise<{
    nodes: FrameworkNode[]
    edges: FrameworkEdge[]
  }>
}
```

---

## 1. Next.js Routes (`routes.ts`)

### Inputs
- Absolute file path
- Source code

### Algorithm
1. **`matches`**: return `true` if path matches any of:
   - `app/**/route.{ts,tsx,js,jsx}` (App Router API route)
   - `app/**/page.{ts,tsx,js,jsx}` (App Router page)
   - `pages/api/**/*.{ts,js}` (Pages Router API)
   - `pages/**/*.{ts,tsx,js,jsx}` (Pages Router page, excluding `_app`, `_document`, `_error`, `api/`)
2. **URL derivation**: strip extension and `(group)` segments per Next.js conventions. `[param]` → `:param`. `[...slug]` → `:slug*`. Index files → `/`.
3. **`recognize`**:
   - Emit one `Route` node with `attrs = { framework: 'nextjs', router: 'app' | 'pages', url: '<derived>' }`
   - For each exported HTTP-verb function (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`, `HEAD`) in App Router `route.ts`, emit `HANDLES` edge `Route → handler symbol`
   - For Pages Router `pages/api/*`, the default export is the handler; emit single `HANDLES` edge

### Complexity
- O(n) over file tree (path matching) + O(k) AST scan per matched file

### Edge cases
- Files outside `app/` and `pages/` → no false positives
- Catch-all `[...slug]` parameter — single edge, URL pattern reflects glob
- Route groups `(marketing)` segments → excluded from URL but kept in graph attrs for traceability
- Files with no HTTP-verb exports in App Router → still emit Route node, log warning (no handlers)

---

## 2. Next.js App Router refinements (`nextjs.ts`)

### Inputs
- Absolute file path

### Algorithm
Beyond the generic Route detection, App Router has specialized file conventions. Recognize each by filename and emit a distinct node kind:

| Filename | Node kind | Edge to emit |
|---|---|---|
| `page.{ts,tsx}` | `Page` | `RENDERS_AT` → derived URL |
| `layout.{ts,tsx}` | `Layout` | `WRAPS` → every co-located + descendant `Page` |
| `loading.{ts,tsx}` | `LoadingState` | `FALLBACK_FOR` → sibling `Page` |
| `error.{ts,tsx}` | `ErrorBoundary` | `CATCHES_FROM` → sibling `Page` |
| `template.{ts,tsx}` | `Template` | `WRAPS_PER_NAVIGATION` → sibling `Page` |
| `not-found.{ts,tsx}` | `NotFoundBoundary` | `CATCHES_404_FROM` → sibling/descendant `Page` |
| `route.{ts,tsx}` | (handled by routes.ts) | — |
| `middleware.{ts,tsx}` at repo root | `Middleware` | `INTERCEPTS` → all matched routes (per `config.matcher`) |

### Complexity
- O(n) over file tree

### Edge cases
- Multiple `layout.tsx` in nested directories → emit each separately + chain `WRAPS` edges parent → child
- `loading.tsx` with no co-located `page.tsx` → still emit, log "orphan loading state"
- `middleware.ts` with `config.matcher` regex → store regex in attrs; resolution to actual route IDs is the Cross-File Resolution stage's job (out of scope for this ALGO)

---

## 3. Runtime tag classification (`runtime-tag.ts`)

### Inputs
- Source code (string)

### Algorithm
1. Read first non-comment, non-whitespace **statement** of the file
2. If it is `'use client'` → tag all symbols emitted from this file with `attrs.runtime = 'client'`
3. If it is `'use server'` → tag with `attrs.runtime = 'server'`
4. Otherwise (default in App Router) → tag with `attrs.runtime = 'server'`
5. **Pages Router** files (`pages/**/*.tsx`) default to `runtime = 'server'` for `getServerSideProps`/`getStaticProps` exports, otherwise `'client'` (hydrated)

### Complexity
- O(1) per file (read first ~200 chars)

### Edge cases
- Both directives present (illegal in Next.js) → log error, mark `runtime = 'invalid'`, surface via PROTO invariant check
- Directive inside a function body (not file-top) → ignore (Next.js itself ignores)
- File with only type-export-no-runtime-code (e.g. `export type Foo = ...`) → no symbols emitted, no runtime tag needed

---

## 4. Data fetching detection (`data-fetching.ts`)

### Inputs
- Parsed AST of a Page or Route file

### Algorithm

**App Router (in `page.tsx` or `route.ts`):**
1. Detect named export `generateStaticParams` → emit `DataLoader` node, `LOADS_FOR` edge → enclosing Page/Route
2. Detect named export `generateMetadata` → emit `MetadataLoader` node, `META_FOR` edge → Page
3. Detect top-level `fetch(...)` calls in Server Component bodies → emit `ExternalFetch` edge → Page (annotate URL if statically determinable)

**Pages Router (in `pages/**/*.tsx`):**
4. Detect named export `getServerSideProps` → emit `DataLoader` (kind: `ssr`) → Page
5. Detect named export `getStaticProps` → emit `DataLoader` (kind: `ssg`) → Page
6. Detect named export `getStaticPaths` → emit `DataLoader` (kind: `paths`) → Page

**Route handlers (in `route.ts`):**
7. Each HTTP-verb export → emit one node per verb, `HANDLES` edge to the Route node (separate from the function's syntactic symbol)

### Complexity
- O(k) per file (one pass over top-level exports)

### Edge cases
- Re-exported data loader from another module (`export { getServerSideProps } from './lib'`) → trace the import, emit DataLoader against the original definition; falls back to local re-export node if cross-file resolution unavailable
- Multiple data loaders in one file → emit each separately

---

## 5. ORM extraction (`orm.ts`)

### Inputs
- File path (and source if not Prisma schema)

### Algorithm

**Prisma (`schema.prisma`):**
1. Parse with `@prisma/internals` (or a tolerant regex pass if dep weight is a concern — start with regex, upgrade if needed)
2. For each `model <Name> { ... }` block → emit `Entity` node with `attrs = { orm: 'prisma', fields: [...] }`
3. For each `@relation` directive → emit `RELATES_TO` edge between Entities

**Drizzle (`*.schema.{ts,js}`):**
4. AST-scan for top-level `export const <name> = pgTable(...)` (or `mysqlTable`, `sqliteTable`) → emit `Entity` node with `attrs = { orm: 'drizzle' }`
5. References to other tables in column definitions (`references(() => otherTable.id)`) → emit `RELATES_TO` edge

### Complexity
- O(n) over schema entries

### Edge cases
- Prisma schema with composite primary key → emit one Entity, store keys in attrs
- Drizzle inline schema (not in dedicated file) → still detected if `pgTable` import is present
- ORM file that exports types only (no schema definition) → emit no entities, no warning

---

## 6. MCP tool discovery (`mcp-tools.ts`)

### Inputs
- TypeScript AST (parsed by existing TypeScript parser)

### Algorithm
1. Walk top-level statements for any of:
   - `server.registerTool({ name: '<id>', ... })`
   - `mcpServer.tool({ name: '<id>', ... })`
   - Decorated class methods with `@mcp.tool` (if codebase uses decorators)
2. For each match:
   - Emit `Tool` node with `attrs = { mcp: true, name: '<id>' }`
   - Find the handler function referenced (inline arrow function, or named reference like `handler: doSomething`) → emit `IMPLEMENTS` edge `Tool → handler symbol`
3. If `inputSchema` is present (JSON schema or Zod schema) → store schema source location in attrs

### Complexity
- O(k) per AST scan

### Edge cases
- Dynamically registered tools (`for (const t of tools) server.registerTool(t)`) → cannot resolve statically; emit warning + skip
- Multiple registration call sites for the same tool name → emit one Tool node, multiple `IMPLEMENTS` edges, mark as ambiguous
- Tool name from a constant (`name: TOOL_NAMES.LOOKUP`) → follow constant if statically determinable; otherwise store source-location-as-name

---

## Composition

The registry in `src/symbols/framework/index.ts` iterates recognizers in order:

1. `routes` (general — sets up Route nodes)
2. `nextjs` (refines App Router file kinds)
3. `runtime-tag` (annotates emitted symbols with `runtime`)
4. `data-fetching` (adds loader nodes + edges)
5. `orm` (independent — file-pattern matched)
6. `mcp-tools` (independent — TS AST matched)

Order matters for `routes` → `nextjs` chain (refinement). Other pairs are independent and could parallelize.

## Verification

Each recognizer ships with ≥ 2 test fixtures (happy path + edge case) in `test/symbols/framework/<name>.test.ts`. Integration test on a fixture Next.js + Prisma project verifies the cross-recognizer composition produces a coherent graph (see `[[BLUEPRINT--SYMBOLS-FRAMEWORK-AWARENESS]]` for the fixture spec).

## Source

- `[[ADR--SYMBOLS-FRAMEWORK-AWARENESS]]` (decisions backing this algorithm set)
- `[[CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS]]` (motivation)
- Next.js App Router file conventions (https://nextjs.org/docs/app/building-your-application/routing)
- Prisma schema reference (https://www.prisma.io/docs/orm/reference/prisma-schema-reference)
- Model Context Protocol specification

## Connections
- [[FRAMEWORK--SYMBOL-GRAPH]]

