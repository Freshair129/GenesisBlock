---
id: BLUEPRINT--MEMORY-BACKLINKS-INDEXER
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — backlinks indexer implementation plan
tags:
  - msp
  - memory
  - backlinks
  - blueprint
  - implementation
crosslinks:
  implements:
    - FEAT--MEMORY-BACKLINKS-INDEXER
  references:
    - ADR--MEMORY-BACKLINKS-INDEXER
    - CONCEPT--MEMORY-VECTOR-BACKLINKS
linked_symbols:
  - file: packages/msp/src/memory/backlinks/indexer.ts
  - file: packages/msp/src/memory/backlinks/walk.ts
  - file: packages/msp/src/memory/backlinks/edges.ts
  - file: packages/msp/src/memory/backlinks/atomic-write.ts
  - file: packages/msp/src/memory/backlinks/cli.ts
created_at: 2026-05-03T14:16:43.374+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  domain: blueprint
---

# BLUEPRINT — backlinks indexer

```yaml
metadata:
  title: "Memory backlinks indexer"
  parent_feat: FEAT--MEMORY-BACKLINKS-INDEXER

architectural_pattern: |
  Three pure modules + one CLI shell. Mirror the existing
  scripts/msp/re-indexer.ts shape so vendoring/maintenance is symmetric.
    - walk.ts          : async generator over gks/<type>/*.md
    - edges.ts         : extract edges from one atom's frontmatter
    - atomic-write.ts  : tmp + rename
    - indexer.ts       : compose them; expose rebuildBacklinks()
    - cli.ts           : flags parsing + invocation

data_logic: |
  rebuildBacklinks({ root, namespace, dryRun, check }):
    1. layout = backlinksPath(root, namespace)  // .brain/msp/projects/<ns>/vector/backlinks.jsonl
    2. edges: Edge[] = []
    3. for await (file of walk(`${root}/gks`)):
         fm = parseFrontmatter(file)
         edges.push(...edgesFromAtom(fm))
    4. edges.sort(byFromToType)
    5. content = edges.map(JSON.stringify).join('\n') + (edges.length > 0 ? '\n' : '')
    6. if dryRun: return summary
    7. if check: read existing; compare; return { changed: existing !== content }
    8. atomicWrite(layout, content)
    9. return { atomCount, edgeCount, changed }

geography:
  - "packages/msp/src/memory/backlinks/walk.ts"
  - "packages/msp/src/memory/backlinks/edges.ts"
  - "packages/msp/src/memory/backlinks/atomic-write.ts"
  - "packages/msp/src/memory/backlinks/indexer.ts"
  - "packages/msp/src/memory/backlinks/cli.ts"
  - "packages/msp/src/memory/backlinks/types.ts"
  - "packages/msp/test/memory/backlinks/edges.test.ts"
  - "packages/msp/test/memory/backlinks/indexer.test.ts"
  - "test/memory/backlinks/cli.test.ts"

api_contracts:
  - name: rebuildBacklinks
    signature: |
      async function rebuildBacklinks(opts: RebuildOpts): Promise<RebuildResult>
    types: |
      interface RebuildOpts {
        root: string
        namespace?: string             // default 'evaAI' per ADR--PATH-ENCODING
        dryRun?: boolean
        check?: boolean
      }
      interface Edge {
        from: string                   // atom id
        to: string                     // target id
        type: string                   // crosslink predicate
      }
      interface RebuildResult {
        atomCount: number
        edgeCount: number
        changed: boolean               // true when output differs from on-disk
      }

verification_plan:
  - vitest: edgesFromAtom yields one edge per crosslinks.* value across all predicates in FRAMEWORK--CROSSLINKS-VOCABULARY
  - vitest: edgesFromAtom returns [] when frontmatter has no crosslinks
  - vitest: indexer sort is stable (same input → same byte output across runs)
  - vitest: --check mode returns changed=true when input differs, changed=false otherwise
  - integration: CLI on this repo (after M2 + knowledge atoms) emits non-empty file with the expected count
```

## Implementation order

T1 WALK-ATOMS (async generator, skip 00_index/)
T2 EMIT-EDGES (per-atom extraction with all predicate keys)
T3 SORT-WRITE (atomic write + check mode + CLI)

## Connections
- [[FEAT--MEMORY-BACKLINKS-INDEXER]]
- [[ADR--MEMORY-BACKLINKS-INDEXER]]
- [[CONCEPT--MEMORY-VECTOR-BACKLINKS]]

