---
id: BLUEPRINT--MSP-OBSIDIAN-CLIENT
phase: 3
type: blueprint
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — MSP Obsidian client implementation plan
tags:
  - msp
  - obsidian
  - client
  - blueprint
  - implementation
crosslinks: {"implements":["FEAT--MSP-OBSIDIAN-CLIENT"],"references":["ADR--MSP-OBSIDIAN-INTEGRATION","CONCEPT--OBSIDIAN-AS-RUNTIME"]}
linked_symbols:
  - {"file":"src/obsidian/client.ts"}
  - {"file":"src/obsidian/types.ts"}
  - {"file":"src/obsidian/rest.ts"}
  - {"file":"src/obsidian/filesystem.ts"}
  - {"file":"src/obsidian/env.ts"}
  - {"file":"test/obsidian/client.test.ts"}
created_at: 2026-05-04T05:25:22.543Z
---

# BLUEPRINT — MSP Obsidian client implementation plan

```yaml
metadata:
  title: "MSP Obsidian client wrapper"
  parent_feat: FEAT--MSP-OBSIDIAN-CLIENT

architectural_pattern: |
  One factory function (createObsidianClient) returns a discriminated client.
  - REST mode: thin wrapper around GksV3's createRestObsidianAdapter (re-exported
    from @freshair129/gks). MSP only adds: probe (HEAD /), env-var resolution,
    deprecation warning for OBSIDIAN_HOST, optional smartViewDeepLink helper.
  - Filesystem mode: tiny in-process implementation that reads gks/<type>/*.md
    directly and matches the same { search, readFile } shape. No graph traversal
    here (graph is GKS scope per ADR--MEMORY-BACKLINKS-INDEXER).

  No bundled embedder. No backlinks logic. mode='rest' only when probe succeeds
  AND OBSIDIAN_API_KEY is set; otherwise fall through to filesystem.

data_logic: |
  src/obsidian/client.ts
    export async function createObsidianClient(opts: ClientOpts = {}): Promise<ObsidianClient>
      1. resolve env: OBSIDIAN_URL || (warn if OBSIDIAN_HOST) || undefined
      2. resolve key: opts.apiKey ?? process.env.OBSIDIAN_API_KEY
      3. if url && key:
           probe = HEAD url with Bearer token + 1.5s timeout
           if 200/204 → return makeRestClient({ url, key, root })
      4. return makeFilesystemClient({ root })

  src/obsidian/rest.ts
    Wraps gks createRestObsidianAdapter:
      - search(query, { limit }) → adapter.search(query, { limit })
      - readFile(relPath) → adapter.resolveWikilink(toWikilink(relPath))?.body
      - activeFile() → GET /active/ on adapter's base URL
      - smartViewDeepLink(atomId) → obsidian://advanced-uri?...

  src/obsidian/filesystem.ts
    - search(query, { limit }) → glob gks/**/*.md, score by case-insensitive
      includes on title+body, return top-N
    - readFile(relPath) → fs.readFile(join(root, relPath))
    - activeFile = undefined (no concept of active file offline)
    - smartViewDeepLink = undefined (no Obsidian to link to)

geography:
  - "src/obsidian/client.ts"        # createObsidianClient factory
  - "src/obsidian/types.ts"         # ObsidianClient, ClientOpts
  - "src/obsidian/rest.ts"          # makeRestClient (wraps GKS adapter)
  - "src/obsidian/filesystem.ts"    # makeFilesystemClient
  - "src/obsidian/env.ts"           # resolveEnv (handles OBSIDIAN_HOST deprecation)
  - "test/obsidian/client.test.ts"  # both modes + deprecation + deep-link

api_contracts:
  - name: createObsidianClient
    signature: |
      function createObsidianClient(opts?: ClientOpts): Promise<ObsidianClient>
    types: |
      interface ClientOpts {
        root?: string
        url?: string
        apiKey?: string
        timeoutMs?: number
        fetch?: typeof fetch
      }
      interface ObsidianClient {
        readonly mode: 'rest' | 'filesystem'
        search(query: string, opts?: { limit?: number }): Promise<SearchHit[]>
        readFile(relPath: string): Promise<string>
        activeFile?: () => Promise<string | null>
        smartViewDeepLink?: (atomId: string) => string
      }

verification_plan:
  - vitest: filesystem mode reads real gks/ atoms (root=cwd) and finds them
  - vitest: rest mode triggered when probe (mocked fetch returning 200) +
    OBSIDIAN_API_KEY set
  - vitest: rest probe failure → fall through to filesystem
  - vitest: missing OBSIDIAN_API_KEY → mode = filesystem even if URL set
  - vitest: setting OBSIDIAN_HOST emits a one-shot deprecation warning to stderr
  - vitest: smartViewDeepLink('FRAME--FOO') returns the obsidian:// URI

  Test count: 233 → ~245 (M7a target)
```

## Implementation order

T1 ENV-RESOLVE       (env.ts: OBSIDIAN_URL canonical, OBSIDIAN_HOST → warn-once)
T2 FILESYSTEM-MODE   (filesystem.ts: glob + grep search + fs read)
T3 REST-MODE         (rest.ts: probe + delegate to GKS adapter)
T4 CLIENT-FACTORY    (client.ts: pick mode, return shape)
T5 DEEP-LINK         (smartViewDeepLink helper)
+ test/obsidian/client.test.ts + AUDIT
