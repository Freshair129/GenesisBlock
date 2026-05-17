---
id: ADR--SYMBOL-GRAPH-PERSISTENCE
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Symbol Graph persistence — SQLite for queries + JSONL exports for git
  diff visibility
tags:
  - msp
  - symbol-graph
  - persistence
  - sqlite
  - jsonl
  - decision
crosslinks:
  references:
    - FRAMEWORK--SYMBOL-GRAPH
    - CONCEPT--SYMBOL-GRAPH
created_at: 2026-05-09T16:50:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — Symbol Graph persistence

## Context

`[[CONCEPT--SYMBOL-GRAPH]]` describes a directed graph over source-code symbols (~5000 nodes + ~20000 edges at MSP's current size). The graph needs a storage layer that is:

1. **Fast for graph queries** — k-hop neighbors, reverse closure, community membership lookups
2. **Git-friendly** — diff-readable so reviewers can see what changed
3. **Reproducible** — deterministic builds; same input = same byte-identical output
4. **Cheap to install** — no native-module flakes on Node 20+22 CI matrix
5. **Per-namespace** — same shape as existing `.brain/msp/projects/<ns>/vector/backlinks.jsonl`

Three obvious candidates: pure JSONL, pure SQLite, or a hybrid.

## Decision

**Hybrid: SQLite for runtime queries, JSONL for git visibility.**

- **Primary index** → `.brain/msp/projects/<ns>/symbols/graph.db` (SQLite via `better-sqlite3`)
- **Git-committed exports** → `.brain/msp/projects/<ns>/symbols/{symbols,edges,communities}.jsonl`
- **Build artifacts** → `meta.json` with timestamp + parser + leiden seed

Rationale per dimension:

| Dimension | Choice | Why |
|---|---|---|
| Fast queries | SQLite | k-hop BFS over `edges` table with composite index `(src_id, type)` is O(degree) per hop. Pure-JSONL would need a load-into-memory step every CLI invocation. |
| Git diff visibility | JSONL exports | SQLite blobs don't `git diff` cleanly. JSONL one-row-per-line, sorted by `id`, gives reviewers a readable diff that shows what symbols/edges changed. |
| Reproducibility | Both — sorted writes | JSONL sorted lexicographically by ID; SQLite `INSERT OR REPLACE` in id order. Leiden seeded with `randomSeed=42`. Two builds on the same source = byte-identical SQLite + byte-identical JSONL. |
| Install cost | `better-sqlite3` ships prebuilt binaries for Node 20/22 Linux/Mac/Windows | One native dep but well-maintained (WiseLibs); historically stable across Node releases. Add to `dependencies` (not `optionalDependencies`). |
| Per-namespace | Same `.brain/msp/projects/<ns>/...` layout as backlinks | Existing pattern; tooling expectations carry over. |

## Consequences

**Positive**
- Fast queries (sub-millisecond k-hop on 5000 nodes)
- Reviewable changes via JSONL diff in PRs
- Mirror of GKS's "JSONL = source of truth, indexed copy = queryable" approach (see `atomic_index.jsonl` + `backlinks.jsonl`)

**Negative**
- Two write paths must stay consistent. Mitigation: `dump-jsonl` CLI subcommand that re-emits JSONL from current SQLite — used both as a build step and as a "fix drift" tool. Tested by round-trip assertion in `test/symbols/store.test.ts` (write JSONL, load to fresh SQLite, dump JSONL, compare).
- One additional native module. Mitigation: `better-sqlite3` is the standard Node SQLite binding; CI-tested across Node 20+22 by upstream.

## Schema sketch (full schema in `[[BLUEPRINT--SYMBOL-GRAPH-CORE]]`, PR-3)

```sql
CREATE TABLE symbols (
  id TEXT PRIMARY KEY,           -- "src/foo.ts:bar:func"
  name TEXT NOT NULL,
  kind TEXT NOT NULL,            -- function|method|class|interface|type|enum|const|module
  file TEXT NOT NULL,
  start_line INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  exported INTEGER NOT NULL,
  parent_id TEXT,
  signature TEXT,
  community_id INTEGER,
  created_at TEXT NOT NULL
);

CREATE TABLE edges (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  src_id TEXT NOT NULL,
  dst_id TEXT NOT NULL,
  type TEXT NOT NULL,            -- calls|extends|implements|imports|references|defines
  weight REAL NOT NULL DEFAULT 1.0,
  resolved INTEGER NOT NULL,
  UNIQUE(src_id, dst_id, type)
);

CREATE TABLE communities (id INTEGER PRIMARY KEY, size INTEGER, label TEXT, modularity REAL, parent_id INTEGER);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
```

JSONL exports are flattened views of the same data, sorted by primary key:

```json
// symbols.jsonl (one per line, sorted by id)
{"id":"src/foo.ts:bar:func","name":"bar","kind":"function","file":"src/foo.ts",...}

// edges.jsonl (one per line, sorted by (src_id, dst_id, type))
{"src":"src/foo.ts:bar:func","dst":"src/baz.ts:qux:func","type":"calls","weight":1.0,"resolved":1}

// communities.jsonl (one per line, sorted by id)
{"id":7,"size":12,"label":"backlinks/edges","modularity":0.0421,"members":[...]}
```

## Alternatives considered

1. **Pure JSONL (no SQLite).** Rejected. k-hop traversal would scan O(N) per hop on every CLI invocation. For N=20000 edges this is fine for one-off CLI use but unacceptable for the MCP `msp_symbol_neighbors` tool that's called per agent turn.
2. **External graph DB (Neo4j / Memgraph).** Rejected per user decision. Native Cypher + Leiden is overkill at our scale (<10k nodes); deploy + sync overhead. Revisit if MSP grows into a multi-repo polyglot codebase (>100k nodes).
3. **In-memory only (rebuild on every CLI start).** Rejected. ~30s rebuild cost would pollute every MCP tool latency budget. SQLite gives us cold-start speed.
4. **JSON file (single document).** Rejected. Doesn't diff well; loading is monolithic.

## What this ADR does NOT decide

- Specific JSONL line shape (frozen in PR-3 BLUEPRINT)
- Migration strategy for schema bumps (deferred — single `meta.schema_version` field reserved; no migration logic in v1, just rebuild)
- Whether to commit `graph.db` (no — gitignored; only JSONL committed)

## Source

- `[[CONCEPT--SYMBOL-GRAPH]]`, `[[FRAMEWORK--SYMBOL-GRAPH]]`
- npm registry inspection 2026-05-09: `better-sqlite3` ships prebuilds for Node 20/22
- Existing GKS patterns: `gks/00_index/atomic_index.jsonl` + `.brain/.../vector/backlinks.jsonl` (sorted JSONL)
